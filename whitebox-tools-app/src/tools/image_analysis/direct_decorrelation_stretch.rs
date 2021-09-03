/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// The Direct Decorrelation Stretch (DDS) is a simple type of saturation stretch. The stretch is
/// applied to a colour composite image and is used to improve the saturation, or colourfulness,
/// of the image. The DDS operates by reducing the achromatic (grey) component of a pixel's colour
/// by a scale factor (*k*), such that the red (r), green (g), and blue (b) components of the output
/// colour are defined as:
///
/// r<sub>*k*</sub> = r - *k* min(r, g, b)
///
/// g<sub>*k*</sub> = g - *k* min(r, g, b)
///
/// b<sub>*k*</sub> = b - *k* min(r, g, b)
///
/// The achromatic factor (*k*) can range between 0 (no effect) and 1 (full saturation stretch),
/// although typical values range from 0.3 to 0.7. A linear stretch is used afterwards to adjust
/// overall image brightness. Liu and Moore (1996) recommend applying a colour balance stretch,
/// such as `BalanceContrastEnhancement` before using the DDS.
///
/// # Reference
/// Liu, J.G., and Moore, J. (1996) Direct decorrelation stretch technique for RGB colour composition.
/// International Journal of Remote Sensing, 17:5, 1005-1018.
///
/// # See Also
/// `CreateColourComposite`, `BalanceContrastEnhancement`
pub struct DirectDecorrelationStretch {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DirectDecorrelationStretch {
    /// Public constructor.
    pub fn new() -> DirectDecorrelationStretch {
        let name = "DirectDecorrelationStretch".to_string();
        let toolbox = "Image Processing Tools/Image Enhancement".to_string();
        let description = "Performs a direct decorrelation stretch enhancement on a colour-composite image of multispectral data.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Colour Composite Image File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input colour composite image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Achromatic Factor (0-1)".to_owned(), 
            flags: vec!["-k".to_owned()], 
            description: "Achromatic factor (k) ranges between 0 (no effect) and 1 (full saturation stretch), although typical values range from 0.3 to 0.7.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Percent to clip the upper tail".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Optional percent to clip the upper tail by during the stretch."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=image.tif -o=output.tif -k=0.4",
            short_exe, name
        )
        .replace("*", &sep);

        DirectDecorrelationStretch {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DirectDecorrelationStretch {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut achromatic_factor = 0.5f64;
        let mut clip_percent = 0.01f64;
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
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
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-k" {
                if keyval {
                    achromatic_factor = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    achromatic_factor = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-clip_percent" || flag_val == "-clip" {
                if keyval {
                    clip_percent = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    clip_percent = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
                if clip_percent < 0f64 {
                    clip_percent = 0f64;
                }
                if clip_percent > 50f64 {
                    clip_percent = 50f64;
                }
                clip_percent = clip_percent / 100f64;
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if achromatic_factor < 0f64 {
            achromatic_factor = 0f64;
        }
        if achromatic_factor > 1f64 {
            achromatic_factor = 1f64;
        }

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading image data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        // let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let rgb_nodata = 0f64;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let (mut red, mut green, mut blue): (u32, u32, u32);
                let (mut r_out, mut g_out, mut b_out): (f64, f64, f64);
                let mut min_val: u32;
                for row in (0..rows).filter(|row_val| row_val % num_procs == tid) {
                    let mut data_r = vec![0u8; columns as usize];
                    let mut data_g = vec![0u8; columns as usize];
                    let mut data_b = vec![0u8; columns as usize];
                    let mut histo_red = [0usize; 256];
                    let mut histo_green = [0usize; 256];
                    let mut histo_blue = [0usize; 256];
                    let mut num_cells = 0;
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            red = z as u32 & 0xFF;
                            green = (z as u32 >> 8) & 0xFF;
                            blue = (z as u32 >> 16) & 0xFF;

                            min_val = red;
                            if green < min_val {
                                min_val = green;
                            }
                            if blue < min_val {
                                min_val = blue;
                            }

                            r_out = red as f64 - achromatic_factor * min_val as f64;
                            g_out = green as f64 - achromatic_factor * min_val as f64;
                            b_out = blue as f64 - achromatic_factor * min_val as f64;

                            if r_out > 255f64 {
                                r_out = 255f64;
                            }
                            if g_out > 255f64 {
                                g_out = 255f64;
                            }
                            if b_out > 255f64 {
                                b_out = 255f64;
                            }

                            if r_out < 0f64 {
                                r_out = 0f64;
                            }
                            if g_out < 0f64 {
                                g_out = 0f64;
                            }
                            if b_out < 0f64 {
                                b_out = 0f64;
                            }

                            data_r[col as usize] = r_out as u8;
                            data_g[col as usize] = g_out as u8;
                            data_b[col as usize] = b_out as u8;

                            histo_red[r_out as usize] += 1;
                            histo_green[g_out as usize] += 1;
                            histo_blue[b_out as usize] += 1;
                            num_cells += 1;
                        }
                    }
                    tx.send((
                        row,
                        data_r,
                        histo_red,
                        data_g,
                        histo_green,
                        data_b,
                        histo_blue,
                        num_cells,
                    ))
                    .unwrap();
                }
            });
        }

        let mut red_band: Array2D<u8> = Array2D::new(rows, columns, 0, 0)?;
        let mut green_band: Array2D<u8> = Array2D::new(rows, columns, 0, 0)?;
        let mut blue_band: Array2D<u8> = Array2D::new(rows, columns, 0, 0)?;
        let mut histo_red = [0usize; 256];
        let mut histo_green = [0usize; 256];
        let mut histo_blue = [0usize; 256];
        let mut num_cells = 0;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            red_band.set_row_data(data.0, data.1);
            for i in 0..256 {
                histo_red[i] += data.2[i];
            }

            green_band.set_row_data(data.0, data.3);
            for i in 0..256 {
                histo_green[i] += data.4[i];
            }

            blue_band.set_row_data(data.0, data.5);
            for i in 0..256 {
                histo_blue[i] += data.6[i];
            }

            num_cells += data.7;
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut stretch_max = 0f64;
        let clip_tail = (num_cells as f64 * clip_percent) as usize;
        let mut count_red = 0;
        let mut count_green = 0;
        let mut count_blue = 0;
        for i in (0..256).rev() {
            if count_red + histo_red[i] > clip_tail {
                stretch_max = (i + 1) as f64;
                break;
            } else {
                count_red += histo_red[i];
            }
            if count_green + histo_green[i] > clip_tail {
                stretch_max = (i + 1) as f64;
                break;
            } else {
                count_green += histo_green[i];
            }
            if count_blue + histo_blue[i] > clip_tail {
                stretch_max = (i + 1) as f64;
                break;
            } else {
                count_blue += histo_blue[i];
            }
        }
        if stretch_max > 255f64 {
            stretch_max = 255f64;
        }

        let mut stretch_min = 0f64;
        count_red = 0;
        count_green = 0;
        count_blue = 0;
        for i in 0..256 {
            if count_red + histo_red[i] > clip_tail {
                if i > 0 {
                    stretch_min = (i - 1) as f64;
                } else {
                    stretch_min = 0f64;
                }
                break;
            } else {
                count_red += histo_red[i];
            }
            if count_green + histo_green[i] > clip_tail {
                if i > 0 {
                    stretch_min = (i - 1) as f64;
                } else {
                    stretch_min = 0f64;
                }
                break;
            } else {
                count_green += histo_green[i];
            }
            if count_blue + histo_blue[i] > clip_tail {
                if i > 0 {
                    stretch_min = (i - 1) as f64;
                } else {
                    stretch_min = 0f64;
                }
                break;
            } else {
                count_blue += histo_blue[i];
            }
        }

        let stretch_range = stretch_max - stretch_min;

        // println!("{}, {} , {}", stretch_min, stretch_max, stretch_range);

        // Perform a linear stretch using the max data.
        let red_band = Arc::new(red_band);
        let green_band = Arc::new(green_band);
        let blue_band = Arc::new(blue_band);

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let red_band = red_band.clone();
            let green_band = green_band.clone();
            let blue_band = blue_band.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let (mut red, mut green, mut blue, mut a): (u32, u32, u32, u32);
                for row in (0..rows).filter(|row_val| row_val % num_procs == tid) {
                    let mut data = vec![rgb_nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            red = red_band[(row, col)] as u32;
                            if red < stretch_min as u32 {
                                red = stretch_min as u32;
                            }
                            if red > stretch_max as u32 {
                                red = stretch_max as u32;
                            }

                            green = green_band[(row, col)] as u32;
                            if green < stretch_min as u32 {
                                green = stretch_min as u32;
                            }
                            if green > stretch_max as u32 {
                                green = stretch_max as u32;
                            }

                            blue = blue_band[(row, col)] as u32;
                            if blue < stretch_min as u32 {
                                blue = stretch_min as u32;
                            }
                            if blue > stretch_max as u32 {
                                blue = stretch_max as u32;
                            }

                            red = (((red as f64 - stretch_min) / stretch_range) * 255f64) as u32;
                            green =
                                (((green as f64 - stretch_min) / stretch_range) * 255f64) as u32;
                            blue = (((blue as f64 - stretch_min) / stretch_range) * 255f64) as u32;
                            a = (z as u32 >> 24) & 0xFF;

                            data[col as usize] =
                                ((a << 24) | (blue << 16) | (green << 8) | red) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.nodata = rgb_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGBA32;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Achromatic factor: {}", achromatic_factor));
        output.add_metadata_entry(format!("Clip percent: {}", clip_percent * 100f64));
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

        /* The following is a single-threaded version that was used for testing */

        // let mut output = Raster::initialize_using_file(&output_file, &input);
        // output.configs.nodata = rgb_nodata
        // output.configs.photometric_interp = PhotometricInterpretation::RGB;
        // output.configs.data_type = DataType::U32;
        // let mut z: f64;
        // let (mut red, mut green, mut blue, mut a): (i32, i32, i32, i32);
        // // let (mut r_out, mut g_out, mut b_out): (f64, f64, f64);
        // let (mut r_out, mut g_out, mut b_out): (i32, i32, i32);
        // let mut min_val: i32;
        // let mut histo_red = [0usize; 256];
        // let mut histo_green = [0usize; 256];
        // let mut histo_blue = [0usize; 256];
        // let mut num_cells = 0;
        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = input[(row, col)];
        //         if z != nodata {
        //             red = (z as u32 & 0xFF) as i32;
        //             green = ((z as u32 >> 8) & 0xFF) as i32;
        //             blue = ((z as u32 >> 16) & 0xFF) as i32;
        //             a = ((z as u32 >> 24) & 0xFF) as i32;

        //             min_val = red;
        //             if green < min_val { min_val = green; }
        //             if blue < min_val { min_val = blue; }

        //             r_out = (red as f64 - achromatic_factor * min_val as f64) as i32;
        //             g_out = (green as f64 - achromatic_factor * min_val as f64) as i32;
        //             b_out = (blue as f64 - achromatic_factor * min_val as f64) as i32;

        //             if r_out > 255 { r_out = 255; }
        //             if g_out > 255 { g_out = 255; }
        //             if b_out > 255 { b_out = 255; }

        //             if r_out < 0 { r_out = 0; }
        //             if g_out < 0 { g_out = 0; }
        //             if b_out < 0 { b_out = 0; }

        //             output[(row, col)] = ((a << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64;

        //             histo_red[r_out as usize] += 1;
        //             histo_green[g_out as usize] += 1;
        //             histo_blue[b_out as usize] += 1;
        //             num_cells += 1;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress (Loop 1 of 2): {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // let mut stretch_max = 0i32;
        // let clip_tail = (num_cells as f64 * clip_percent) as usize;
        // let mut count_red = 0;
        // let mut count_green = 0;
        // let mut count_blue = 0;
        // for i in (0..256).rev() {
        //     if count_red + histo_red[i] > clip_tail {
        //         stretch_max = i as i32;
        //         break;
        //     } else {
        //         count_red += histo_red[i];
        //     }
        //     if count_green + histo_green[i] > clip_tail {
        //         stretch_max = i as i32;
        //         break;
        //     } else {
        //         count_green += histo_green[i];
        //     }
        //     if count_blue + histo_blue[i] > clip_tail {
        //         stretch_max = i as i32;
        //         break;
        //     } else {
        //         count_blue += histo_blue[i];
        //     }
        // }

        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = output[(row, col)];
        //         if z != rgb_nodata {
        //             red = (z as u32 & 0xFF) as i32;
        //             green = ((z as u32 >> 8) & 0xFF) as i32;
        //             blue = ((z as u32 >> 16) & 0xFF) as i32;
        //             a = ((z as u32 >> 24) & 0xFF) as i32;

        //             if red < 0 { red = 0; }
        //             if green < 0 { green = 0; }
        //             if blue < 0 { blue = 0; }

        //             if red > stretch_max { red = stretch_max; }
        //             if green > stretch_max { green = stretch_max; }
        //             if blue > stretch_max { blue = stretch_max; }

        //             r_out = (255f64 * red as f64 / stretch_max as f64) as i32;
        //             g_out = (255f64 * green as f64 / stretch_max as f64) as i32;
        //             b_out = (255f64 * blue as f64 / stretch_max as f64) as i32;

        //             output[(row, col)] = ((a << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress (Loop 2 of 2): {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
