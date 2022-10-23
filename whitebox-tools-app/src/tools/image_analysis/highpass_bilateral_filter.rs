/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/06/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to perform a high-pass bilateral filter. A high-pass filter is one which emphasizes short-scale
/// variation within an image, usually by differencing the input image value from the result of a low-pass, smoothing filter.
/// In this case, the low-pass filter is an edge-preserving bilateral filter (`BilateralFilter`). High-pass filters are
/// often dominated by edges (transitions from high values to low values or vice versa) within an image. Because the
/// bilateral filter is an edge-preserving filter, the high-pass bilateral filter output is not dominated by edges, instead
/// emphasizing local-scale image texture patters. This filter is excellent for mapping image textures.
/// 
/// The size of the filter is determined  by setting the standard deviation distance parameter (`--sigma_dist`); the 
/// larger the standard deviation the larger the resulting filter kernel. The standard deviation can be any number in 
/// the range 0.5-20 and is specified in the unit of pixels. The standard deviation intensity parameter (`--sigma_int`), 
/// specified in the same units as the image values, determines the intensity domain contribution to kernel weightings.
/// If the input image is an RGB composite, the intensity value is filtered, and the intensity parameter should
/// lie 0 > parameter < 1, with typical values ranging from 0.05 to 0.25. If the input image is not an RGB colour
/// composite, appropriate values of this parameter will depend on the range of input values and will likely be
/// considerably higher.
///
/// # References
/// Tomasi, C., & Manduchi, R. (1998, January). Bilateral filtering for gray and color images. In null (p. 839). IEEE.
///
/// # See Also
/// `BilateralFilter`
pub struct HighPassBilateralFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HighPassBilateralFilter {
    pub fn new() -> HighPassBilateralFilter {
        // public constructor
        let name = "HighPassBilateralFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a high-pass bilateral filter, by differencing an input image by the bilateral filter by Tomasi and Manduchi (1998).".to_string();

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
            name: "Distance Standard Deviation (pixels)".to_owned(),
            flags: vec!["--sigma_dist".to_owned()],
            description: "Standard deviation in distance in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.75".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Intensity Standard Deviation (intensity units)".to_owned(),
            flags: vec!["--sigma_int".to_owned()],
            description: "Standard deviation in intensity in pixels.".to_owned(),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --sigma_dist=2.5 --sigma_int=4.0", short_exe, name).replace("*", &sep);

        HighPassBilateralFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HighPassBilateralFilter {
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
        let mut filter_size = 0usize;
        let mut sigma_dist = 0.75;
        let mut sigma_int = 1.0;
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
            } else if flag_val == "-sigma_dist" {
                if keyval {
                    sigma_dist = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    sigma_dist = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-sigma_int" {
                if keyval {
                    sigma_int = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    sigma_int = args[i + 1]
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

        if sigma_dist < 0.5 {
            sigma_dist = 0.5;
        } else if sigma_dist > 20.0 {
            sigma_dist = 20.0;
        }

        if sigma_int < 0.001 {
            sigma_int = 0.001;
        }

        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma_dist);
        let two_sigma_sqr_d = 2.0 * sigma_dist * sigma_dist;

        let recip_root_2_pi_times_sigma_i = 1.0 / ((2.0 * PI).sqrt() * sigma_int);
        let two_sigma_sqr_i = 2.0 * sigma_int * sigma_int;

        // figure out the size of the filter
        let mut weight: f64;
        for i in 0..250 {
            weight =
                recip_root_2_pi_times_sigma_d * (-1.0 * ((i * i) as f64) / two_sigma_sqr_d).exp();
            if weight <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }

        // the filter dimensions must be odd numbers such that there is a middle pixel
        if filter_size % 2 == 0 {
            filter_size += 1;
        }

        if filter_size < 3 {
            filter_size = 3;
        }

        let num_pixels_in_filter = filter_size * filter_size;
        let mut dx = vec![0isize; num_pixels_in_filter];
        let mut dy = vec![0isize; num_pixels_in_filter];
        let mut weights_d = vec![0.0; num_pixels_in_filter];

        // fill the filter d_x and d_y values and the distance-weights
        let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size {
            for col in 0..filter_size {
                x = col as isize - midpoint;
                y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                weight = recip_root_2_pi_times_sigma_d
                    * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                weights_d[a] = weight;
                a += 1;
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights_d = Arc::new(weights_d);

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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let dx = dx.clone();
            let dy = dy.clone();
            let weights_d = weights_d.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<dyn Fn(isize, isize) -> f64> = if !is_rgb_image {
                    Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                } else {
                    Box::new(|row: isize, col: isize| -> f64 {
                        let value = input.get_value(row, col);
                        if value != nodata {
                            return value2i(value);
                        }
                        nodata
                    })
                };

                let (mut sum, mut z_final): (f64, f64);
                let mut z: f64;
                let mut zn: f64;
                let (mut x, mut y): (isize, isize);
                let mut weight: f64;
                let mut weights_i = vec![0.0; num_pixels_in_filter];

                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
                        if z != nodata {
                            //fill weights_i with the appropriate intensity weights
                            sum = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_fn(y, x);
                                if zn != nodata {
                                    weight = recip_root_2_pi_times_sigma_i
                                        * (-1.0 * ((zn - z) * (zn - z)) / two_sigma_sqr_i).exp();
                                    weight *= weights_d[a];
                                    weights_i[a] = weight;
                                    sum += weight;
                                }
                            }

                            z_final = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_fn(y, x);
                                if zn != nodata {
                                    z_final += weights_i[a] * zn / sum;
                                }
                            }

                            data[col as usize] = z - z_final;
                        }
                    }

                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma distance: {}", sigma_dist));
        output.add_metadata_entry(format!("Sigma intensity: {}", sigma_int));
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
