/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/07/2017
Last Modified: 18/10/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to create a colour-composite image from three bands of multi-spectral imagery. 
/// The user must specify the names of the input images to enter into the red, green, and blue channels 
/// of the resulting composite image. The output image uses the 32-bit aRGB colour model, and therefore, 
/// in addition to red, green and blue bands, the user may optionally specify a fourth image that will 
/// be used to determine pixel opacity (the 'a' channel). If no opacity image is specified, each pixel 
/// will be opaque. This can be useful for cropping an image to an irregular-shaped boundary. The opacity 
/// channel can also be used to create transparent gradients in the composite image.
/// 
/// A balance contrast enhancement (BCE) can optionally be performed on the bands prior to creation of
/// the colour composite. While this operation will add to the runtime of `CreateColourComposite`, if
/// the individual input bands have not already had contrast enhancements, then it is advisable that
/// the BCE option be used to improve the quality of the resulting colour composite image.
/// 
/// NoData values in any of the input images are assigned NoData values in the output image and are not
/// taken into account when performing the BCE operation. Please note, not all images have NoData values
/// identified. When this is the case, and when the background value is 0 (often the case with 
/// multispectral imagery), then the `CreateColourComposite` tool can be told to ignore zero values using 
/// the `--zeros` flag.
/// 
/// # See Also
/// `BalanceContrastEnhancement`, `SplitColourComposite`
pub struct CreateColourComposite {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CreateColourComposite {
    /// Public constructor.
    pub fn new() -> CreateColourComposite {
        let name = "CreateColourComposite".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description =
            "Creates a colour-composite image from three bands of multispectral imagery."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Red Band Image File".to_owned(),
            flags: vec!["--red".to_owned()],
            description: "Input red band image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Green Band Image File".to_owned(),
            flags: vec!["--green".to_owned()],
            description: "Input green band image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Blue Band Image File".to_owned(),
            flags: vec!["--blue".to_owned()],
            description: "Input blue band image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Opacity Band Image File (Optional)".to_owned(),
            flags: vec!["--opacity".to_owned()],
            description: "Input opacity band image file (optional).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Colour Composite File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output colour composite file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Perform balance contrast enhancement?".to_owned(),
            flags: vec!["--enhance".to_owned()],
            description:
                "Optional flag indicating whether a balance contrast enhancement is performed."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Treat zeros as nodata?".to_owned(),
            flags: vec!["--zeros".to_owned()],
            description:
                "Optional flag to indicate if zeros are nodata values."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --red=band3.tif --green=band2.tif --blue=band1.tif -o=output.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --red=band3.tif --green=band2.tif --blue=band1.tif --opacity=a.tif -o=output.tif --enhance --zeros", short_exe, name).replace("*", &sep);

        CreateColourComposite {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CreateColourComposite {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input1_file = String::new();
        let mut input2_file = String::new();
        let mut input3_file = String::new();
        let mut input4_file = String::new();
        let mut input4_used = false;
        let mut output_file = String::new();
        let mut enhance = false;
        let mut no_zeros = false;
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-red" {
                input1_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-green" {
                input2_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-blue" {
                input3_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-opacity" {
                input4_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                input4_used = true;
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-enhance" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    enhance = true;
                }
            } else if flag_val == "-zeros" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    // treat zero values as nodata.
                    no_zeros = true;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input1_file.contains(&sep) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }
        if !input3_file.contains(&sep) && !input3_file.contains("/") {
            input3_file = format!("{}{}", working_directory, input3_file);
        }
        if input4_used {
            if !input4_file.contains(&sep) && !input4_file.contains("/") {
                input4_file = format!("{}{}", working_directory, input4_file);
            }
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading red band data...")
        };
        let input_r = Arc::new(Raster::new(&input1_file, "r")?);
        if verbose {
            println!("Reading green band data...")
        };
        let input_g = Arc::new(Raster::new(&input2_file, "r")?);
        if verbose {
            println!("Reading blue band data...")
        };
        let input_b = Arc::new(Raster::new(&input3_file, "r")?);

        let rows = input_r.configs.rows as isize;
        let columns = input_r.configs.columns as isize;
        let mut nodata_r = input_r.configs.nodata;
        let mut nodata_g = input_g.configs.nodata;
        let mut nodata_b = input_b.configs.nodata;
        if no_zeros {
            nodata_r = 0f64;
            nodata_g = 0f64;
            nodata_b = 0f64;
        }
        let red_min = input_r.configs.display_min;
        let green_min = input_g.configs.display_min;
        let blue_min = input_b.configs.display_min;
        let red_range = input_r.configs.display_max - red_min;
        let green_range = input_g.configs.display_max - green_min;
        let blue_range = input_b.configs.display_max - blue_min;
        let a_min: f64;
        let a_range: f64;
        let input_a = match input4_used {
            true => {
                if verbose {
                    println!("Reading opacity data...")
                };
                let opacity = Raster::new(&input4_file, "r")?;
                a_min = opacity.configs.display_min;
                a_range = opacity.configs.display_max - a_min;
                if input_r.configs.rows != opacity.configs.rows
                    || input_r.configs.columns != opacity.configs.columns
                {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                          "The input files must have the same number of rows and columns and spatial extent."));
                }
                Arc::new(opacity.get_data_as_array2d())
            }
            false => {
                let opacity: Array2D<f64> = Array2D::new(rows, columns, 255f64, nodata_r)?;
                a_min = 0f64;
                a_range = 255f64;
                Arc::new(opacity)
            }
        };

        let start = Instant::now();

        // make sure the input files have the same size
        if input_r.configs.rows != input_g.configs.rows
            || input_r.configs.columns != input_g.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }
        if input_r.configs.rows != input_b.configs.rows
            || input_r.configs.columns != input_b.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input_r = input_r.clone();
            let input_g = input_g.clone();
            let input_b = input_b.clone();
            let input_a = input_a.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut red_val: f64;
                let mut green_val: f64;
                let mut blue_val: f64;
                let mut a_val: f64;
                let (mut r, mut g, mut b, mut a): (u32, u32, u32, u32);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata_r; columns as usize];
                    for col in 0..columns {
                        red_val = input_r[(row, col)];
                        green_val = input_g[(row, col)];
                        blue_val = input_b[(row, col)];
                        if red_val != nodata_r && green_val != nodata_g && blue_val != nodata_b {
                            red_val = (red_val - red_min) / red_range * 255f64;
                            if red_val < 0f64 {
                                red_val = 0f64;
                            }
                            if red_val > 255f64 {
                                red_val = 255f64;
                            }
                            r = red_val as u32;

                            green_val = (green_val - green_min) / green_range * 255f64;
                            if green_val < 0f64 {
                                green_val = 0f64;
                            }
                            if green_val > 255f64 {
                                green_val = 255f64;
                            }
                            g = green_val as u32;

                            blue_val = (blue_val - blue_min) / blue_range * 255f64;
                            if blue_val < 0f64 {
                                blue_val = 0f64;
                            }
                            if blue_val > 255f64 {
                                blue_val = 255f64;
                            }
                            b = blue_val as u32;

                            a_val = input_a[(row, col)];
                            a_val = (a_val - a_min) / a_range * 255f64;
                            if a_val < 0f64 {
                                a_val = 0f64;
                            }
                            if a_val > 255f64 {
                                a_val = 255f64;
                            }
                            a = a_val as u32;
                            data[col as usize] = ((a << 24) | (b << 16) | (g << 8) | r) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input_r);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGBA32;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if enhance {
            let mut z: f64;
            let (mut r, mut g, mut b, mut a): (u32, u32, u32, u32);
            let (mut r_out, mut g_out, mut b_out): (u32, u32, u32);
            let (mut r_outf, mut g_outf, mut b_outf): (f64, f64, f64);
            let e = 100f64;
            let l = 0f64;
            let h = 255f64;
            let mut num_pixels = 0f64;
            let mut r_l = i32::max_value() as f64;
            let mut r_h = i32::min_value() as f64;
            let mut r_e = 0f64;
            let mut r_sqr_total = 0f64;
            let mut g_l = i32::max_value() as f64;
            let mut g_h = i32::min_value() as f64;
            let mut g_e = 0f64;
            let mut g_sqr_total = 0f64;
            let mut b_l = i32::max_value() as f64;
            let mut b_h = i32::min_value() as f64;
            let mut b_e = 0f64;
            let mut b_sqr_total = 0f64;

            for row in 0..rows {
                for col in 0..columns {
                    z = output[(row, col)];
                    if z != nodata_r {
                        num_pixels += 1f64;
                        r = z as u32 & 0xFF;
                        g = (z as u32 >> 8) & 0xFF;
                        b = (z as u32 >> 16) & 0xFF;

                        if (r as f64) < r_l {
                            r_l = r as f64;
                        }
                        if (r as f64) > r_h {
                            r_h = r as f64;
                        }
                        r_e += r as f64;
                        r_sqr_total += (r * r) as f64;

                        if (g as f64) < g_l {
                            g_l = g as f64;
                        }
                        if (g as f64) > g_h {
                            g_h = g as f64;
                        }
                        g_e += g as f64;
                        g_sqr_total += (g * g) as f64;

                        if (b as f64) < b_l {
                            b_l = b as f64;
                        }
                        if (b as f64) > b_h {
                            b_h = b as f64;
                        }
                        b_e += b as f64;
                        b_sqr_total += (b * b) as f64;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing Enhancement (1 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            r_e = r_e / num_pixels;
            g_e = g_e / num_pixels;
            b_e = b_e / num_pixels;

            let r_s = r_sqr_total as f64 / num_pixels as f64;
            let g_s = g_sqr_total as f64 / num_pixels as f64;
            let b_s = b_sqr_total as f64 / num_pixels as f64;

            let r_b = (r_h * r_h * (e - l) - r_s * (h - l) + r_l * r_l * (h - e))
                / (2f64 * (r_h * (e - l) - r_e * (h - l) + r_l * (h - e)));
            let r_a = (h - l) / ((r_h - r_l) * (r_h + r_l - 2f64 * r_b));
            let r_c = l - r_a * ((r_l - r_b) * (r_l - r_b));

            let g_b = (g_h * g_h * (e - l) - g_s * (h - l) + g_l * g_l * (h - e))
                / (2f64 * (g_h * (e - l) - g_e * (h - l) + g_l * (h - e)));
            let g_a = (h - l) / ((g_h - g_l) * (g_h + g_l - 2f64 * g_b));
            let g_c = l - g_a * ((g_l - g_b) * (g_l - g_b));

            let b_b = (b_h * b_h * (e - l) - b_s * (h - l) + b_l * b_l * (h - e))
                / (2f64 * (b_h * (e - l) - b_e * (h - l) + b_l * (h - e)));
            let b_a = (h - l) / ((b_h - b_l) * (b_h + b_l - 2f64 * b_b));
            let b_c = l - b_a * ((b_l - b_b) * (b_l - b_b));

            for row in 0..rows {
                for col in 0..columns {
                    z = output[(row, col)];
                    if z != nodata_r {
                        r = z as u32 & 0xFF;
                        g = (z as u32 >> 8) & 0xFF;
                        b = (z as u32 >> 16) & 0xFF;
                        a = (z as u32 >> 24) & 0xFF;

                        r_outf = r_a * ((r as f64 - r_b) * (r as f64 - r_b)) + r_c;
                        g_outf = g_a * ((g as f64 - g_b) * (g as f64 - g_b)) + g_c;
                        b_outf = b_a * ((b as f64 - b_b) * (b as f64 - b_b)) + b_c;

                        if r_outf > 255f64 {
                            r_outf = 255f64;
                        }
                        if g_outf > 255f64 {
                            g_outf = 255f64;
                        }
                        if b_outf > 255f64 {
                            b_outf = 255f64;
                        }

                        if r_outf < 0f64 {
                            r_outf = 0f64;
                        }
                        if g_outf < 0f64 {
                            g_outf = 0f64;
                        }
                        if b_outf < 0f64 {
                            b_outf = 0f64;
                        }

                        r_out = r_outf as u32;
                        g_out = g_outf as u32;
                        b_out = b_outf as u32;

                        output[(row, col)] =
                            ((a << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing Enhancement (2 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input red band file: {}", input1_file));
        output.add_metadata_entry(format!("Input green band file: {}", input2_file));
        output.add_metadata_entry(format!("Input blue band file: {}", input3_file));
        if input4_used {
            output.add_metadata_entry(format!("Input opacity file: {}", input4_file));
        }
        output.add_metadata_entry(format!("Balance contrast enhancement: {}", enhance));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
