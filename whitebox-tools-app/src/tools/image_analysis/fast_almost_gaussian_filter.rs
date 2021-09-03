/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/05/2018
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;

/// The tool is somewhat modified from Dr. Kovesi's original Matlab code in that it
/// works with both greyscale and RGB images (decomposes to HSI and uses the intensity
/// data) and it handles the case of rasters that contain NoData values. This adds
/// complexity to the original 20 additions and 5 multiplications assertion of the
/// original paper.
///
/// Also note, for small values of sigma (< 1.8), you should probably just use the
/// regular GaussianFilter tool.
///
/// # Reference
/// P. Kovesi 2010 Fast Almost-Gaussian Filtering, Digital Image Computing:
/// Techniques and Applications (DICTA), 2010 International Conference on.
pub struct FastAlmostGaussianFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FastAlmostGaussianFilter {
    pub fn new() -> FastAlmostGaussianFilter {
        // public constructor
        let name = "FastAlmostGaussianFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a fast approximate Gaussian filter on an image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            name: "Standard Deviation (pixels)".to_owned(),
            flags: vec!["--sigma".to_owned()],
            description: "Standard deviation distance in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.8".to_owned()),
            optional: false,
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --sigma=2.0",
            short_exe, name
        )
        .replace("*", &sep);

        FastAlmostGaussianFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FastAlmostGaussianFilter {
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
        let mut sigma = 1.8;
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
            } else if flag_val == "-sigma" {
                if keyval {
                    sigma = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    sigma = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if sigma < 1.8 {
            println!("Warning: Sigma values less than 1.8 cannot be achieved using this filter. Perhaps use the GaussianFilter tool instead.");
            sigma = 1.8;
        }

        let n = 5;
        let w_ideal = (12f64 * sigma * sigma / n as f64 + 1f64).sqrt();
        let mut wl = w_ideal.floor() as isize;
        if wl % 2 == 0 {
            wl -= 1;
        } // must be an odd integer
        let wu = wl + 2;
        let m =
            ((12f64 * sigma * sigma - (n * wl * wl) as f64 - (4 * n * wl) as f64 - (3 * n) as f64)
                / (-4 * wl - 4) as f64)
                .round() as isize;

        let sigma_actual =
            (((m * wl * wl) as f64 + ((n - m) as f64) * (wu * wu) as f64 - n as f64) / 12f64)
                .sqrt();
        if verbose {
            println!("Actual sigma: {:.3}", sigma_actual);
        }

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1)?;
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let mut val: f64;
        let mut sum: f64;
        let mut sum_n: i32;
        let mut i_prev: f64;
        let mut n_prev: i32;
        let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
        let mut num_cells: i32;

        for iteration_num in 0..n {
            if verbose {
                println!("Loop {} of {}", iteration_num + 1, n);
            }

            let midpoint = if iteration_num <= m {
                (wl as f64 / 2f64).floor() as isize
            } else {
                (wu as f64 / 2f64).floor() as isize
            };

            if iteration_num == 0 {
                // first iteration
                let input_fn: Box<dyn Fn(isize, isize) -> f64> = if !is_rgb_image {
                    // It's a greyscale image; just read the value.
                    Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                } else {
                    // It's an RGB image. Get the intensity value from the IHS decomposition of the RGB value.
                    Box::new(|row: isize, col: isize| -> f64 {
                        let value = input.get_value(row, col);
                        if value != nodata {
                            return value2i(value);
                        }
                        nodata
                    })
                };

                // Create the integral images.
                for row in 0..rows {
                    sum = 0f64;
                    sum_n = 0;
                    for col in 0..columns {
                        val = input_fn(row, col);
                        if val == nodata {
                            val = 0f64;
                        } else {
                            sum_n += 1;
                        }
                        sum += val;
                        if row > 0 {
                            i_prev = integral.get_value(row - 1, col);
                            n_prev = integral_n.get_value(row - 1, col);
                            integral.set_value(row, col, sum + i_prev);
                            integral_n.set_value(row, col, sum_n + n_prev);
                        } else {
                            integral.set_value(row, col, sum);
                            integral_n.set_value(row, col, sum_n);
                        }
                    }
                }
            } else {
                // Create the integral image based on previous iteration output.
                // We don't need to recalculate the num_cells integral image.
                for row in 0..rows {
                    sum = 0f64;
                    for col in 0..columns {
                        val = output.get_value(row, col);
                        if val == nodata {
                            val = 0f64;
                        }
                        sum += val;
                        if row > 0 {
                            i_prev = integral.get_value(row - 1, col);
                            integral.set_value(row, col, sum + i_prev);
                        } else {
                            integral.set_value(row, col, sum);
                        }
                    }
                }
            }

            if iteration_num < n - 1 {
                // not the last iteration
                // Perform Filter
                for row in 0..rows {
                    y1 = row - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    y2 = row + midpoint;
                    if y2 >= rows {
                        y2 = rows - 1;
                    }

                    for col in 0..columns {
                        if input.get_value(row, col) != nodata {
                            x1 = col - midpoint - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }
                            x2 = col + midpoint;
                            if x2 >= columns {
                                x2 = columns - 1;
                            }

                            num_cells = integral_n[(y2, x2)] + integral_n[(y1, x1)]
                                - integral_n[(y1, x2)]
                                - integral_n[(y2, x1)];
                            if num_cells > 0 {
                                sum = integral[(y2, x2)] + integral[(y1, x1)]
                                    - integral[(y1, x2)]
                                    - integral[(y2, x1)];
                                output.set_value(row, col, sum / num_cells as f64);
                            } else {
                                // should never hit here since input(row, col) != nodata above, therefore, num_cells >= 1
                                output.set_value(row, col, 0f64);
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!(
                                "Progress (Loop {} of {}): {}%",
                                iteration_num + 1,
                                n,
                                progress
                            );
                            old_progress = progress;
                        }
                    }
                }
            } else {
                // last iteration

                let output_fn: Box<dyn Fn(isize, isize, f64) -> f64> = if !is_rgb_image {
                    // simply return the value.
                    Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                } else {
                    // convert it back into an rgb value, using the modified intensity value.
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            return hsi2value(h, s, value);
                        }
                        nodata
                    })
                };

                // Perform Filter
                for row in 0..rows {
                    y1 = row - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    y2 = row + midpoint;
                    if y2 >= rows {
                        y2 = rows - 1;
                    }

                    for col in 0..columns {
                        if input.get_value(row, col) != nodata {
                            x1 = col - midpoint - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }
                            x2 = col + midpoint;
                            if x2 >= columns {
                                x2 = columns - 1;
                            }

                            num_cells = integral_n[(y2, x2)] + integral_n[(y1, x1)]
                                - integral_n[(y1, x2)]
                                - integral_n[(y2, x1)];
                            if num_cells > 0 {
                                sum = integral[(y2, x2)] + integral[(y1, x1)]
                                    - integral[(y1, x2)]
                                    - integral[(y2, x1)];
                                val = output_fn(row, col, sum / num_cells as f64);
                            } else {
                                // should never hit here since input(row, col) != nodata above, therefore, num_cells >= 1
                                val = output_fn(row, col, 0f64);
                            }

                            output.set_value(row, col, val);
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!(
                                "Progress (Loop {} of {}): {}%",
                                iteration_num + 1,
                                n,
                                progress
                            );
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma: {}", sigma));
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

fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    (r + g + b) / 3f64
}

fn value2hsi(value: f64) -> (f64, f64, f64) {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    let i = (r + g + b) / 3f64;

    let rn = r / (r + g + b);
    let gn = g / (r + g + b);
    let bn = b / (r + g + b);

    let mut h = if rn != gn || rn != bn {
        ((0.5 * ((rn - gn) + (rn - bn))) / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt())
            .acos()
    } else {
        0f64
    };
    if b > g {
        h = 2f64 * PI - h;
    }

    let s = 1f64 - 3f64 * rn.min(gn).min(bn);

    (h, s, i)
}

fn hsi2value(h: f64, s: f64, i: f64) -> f64 {
    let mut r: u32;
    let mut g: u32;
    let mut b: u32;

    let x = i * (1f64 - s);

    if h < 2f64 * PI / 3f64 {
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (y * 255f64).round() as u32;
        g = (z * 255f64).round() as u32;
        b = (x * 255f64).round() as u32;
    } else if h < 4f64 * PI / 3f64 {
        let h = h - 2f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (x * 255f64).round() as u32;
        g = (y * 255f64).round() as u32;
        b = (z * 255f64).round() as u32;
    } else {
        let h = h - 4f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (z * 255f64).round() as u32;
        g = (x * 255f64).round() as u32;
        b = (y * 255f64).round() as u32;
    }

    if r > 255u32 {
        r = 255u32;
    }
    if g > 255u32 {
        g = 255u32;
    }
    if b > 255u32 {
        b = 255u32;
    }

    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}
