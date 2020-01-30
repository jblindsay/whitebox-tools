/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to reduce colour bias in a colour composite image based on the
/// technique described by Liu (1991). Colour bias is a common phenomena with colour images
/// derived from multispectral imagery, whereby a higher average brightness value in one
/// band results in over-representation of that band in the colour composite. The tool
/// essentially applies a parabolic stretch to each of the three bands in a user specified
/// RGB colour composite, forcing the histograms of each band to have the same minimum,
/// maximum, and average values while maintaining their overall histogram shape. For greater
/// detail on the operation of the tool, please see Liu (1991). Aside from the names of the
/// input and output colour composite images, the user must also set the value of E, the
/// desired output band mean, where 20 < E < 235.
///
/// # Reference
/// Liu, J.G. (1991) Balance contrast enhancement technique and its application in image
/// colour composition. *International Journal of Remote Sensing*, 12:10.
///
/// # See Also
/// `DirectDecorrelationStretch`, `HistogramMatching`, `HistogramMatchingTwoImages`, `HistogramEqualization`, `GaussianContrastStretch`
pub struct BalanceContrastEnhancement {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BalanceContrastEnhancement {
    /// Public constructor.
    pub fn new() -> BalanceContrastEnhancement {
        let name = "BalanceContrastEnhancement".to_string();
        let toolbox = "Image Processing Tools/Image Enhancement".to_string();
        let description = "Performs a balance contrast enhancement on a colour-composite image of multispectral data."
            .to_string();

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

        parameters.push(ToolParameter {
            name: "Band Mean Value".to_owned(),
            flags: vec!["--band_mean".to_owned()],
            description: "Band mean value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("100.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=image.tif -o=output.tif --band_mean=120", short_exe, name).replace("*", &sep);

        BalanceContrastEnhancement {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BalanceContrastEnhancement {
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
        let mut e = 100f64;
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
            } else if flag_val == "-band_mean" {
                if keyval {
                    e = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    e = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
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

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let l = 0f64;
        let h = 255f64;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let (mut r, mut g, mut b): (u32, u32, u32);
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
                for row in (0..rows).filter(|rt| rt % num_procs == tid) {
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
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
                }
                tx.send((
                    r_l,
                    r_h,
                    r_e,
                    r_sqr_total,
                    g_l,
                    g_h,
                    g_e,
                    g_sqr_total,
                    b_l,
                    b_h,
                    b_e,
                    b_sqr_total,
                    num_pixels,
                ))
                .unwrap();
            });
        }

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

        for tid in 0..num_procs {
            let (
                tr_l,
                tr_h,
                tr_e,
                tr_sqr_total,
                tg_l,
                tg_h,
                tg_e,
                tg_sqr_total,
                tb_l,
                tb_h,
                tb_e,
                tb_sqr_total,
                tnum_pixels,
            ) = rx.recv().unwrap();

            if tr_l < r_l {
                r_l = tr_l;
            }
            if tr_h < r_h {
                r_h = tr_h;
            }
            r_e += tr_e;
            r_sqr_total += tr_sqr_total;

            if tg_l < g_l {
                g_l = tg_l;
            }
            if tg_h < g_h {
                g_h = tg_h;
            }
            g_e += tg_e;
            g_sqr_total += tg_sqr_total;

            if tb_l < b_l {
                b_l = tb_l;
            }
            if tb_h < b_h {
                b_h = tb_h;
            }
            b_e += tb_e;
            b_sqr_total += tb_sqr_total;

            num_pixels += tnum_pixels;

            if verbose {
                progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
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

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let (mut r, mut g, mut b, mut a): (u32, u32, u32, u32);
                let (mut r_out, mut g_out, mut b_out): (u32, u32, u32);
                let (mut r_outf, mut g_outf, mut b_outf): (f64, f64, f64);
                for row in (0..rows).filter(|rt| rt % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
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

                            data[col as usize] =
                                ((a << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGBA32;
        for row in 0..rows {
            let data = rx.recv().unwrap();
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
        output.add_metadata_entry(format!("Band mean value: {}", e));
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
