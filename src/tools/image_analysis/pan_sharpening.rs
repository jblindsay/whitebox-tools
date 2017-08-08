/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 27, 2017
Last Modified: July 27, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use structures::Array2D;
use tools::WhiteboxTool;

/// Tool struct containing the essential descriptors required to interact with the tool.
pub struct PanchromaticSharpening {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl PanchromaticSharpening {
    /// Public constructor.
    pub fn new() -> PanchromaticSharpening {
        let name = "PanchromaticSharpening".to_string();

        let description = "Increases the spatial resolution of image data by combining multispectral bands with panchromatic data.".to_string();

        let mut parameters = "--red          Input red band raster file.\n".to_owned();
        parameters.push_str("--green        Input green raster file.\n");
        parameters.push_str("--blue         Input blue raster file.\n");
        parameters.push_str("--composite    Optional input colour-composite image file.\n");
        parameters.push_str("--pan          Input panchromatic image file.\n");
        parameters.push_str("-o, --output   Output colour composite image file.\n");
        parameters.push_str("--method       Options include 'brovey' and 'ihs' (default is 'brovey').\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --red=red.dep --green=green.dep --blue=blue.dep --pan=pan.dep --output=pan_sharp.dep --method='brovey'
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --composite=image.dep --pan=pan.dep --output=pan_sharp.dep --method='ihs'", short_exe, name).replace("*", &sep);

        PanchromaticSharpening {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PanchromaticSharpening {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut red_file = String::new();
        let mut green_file = String::new();
        let mut blue_file = String::new();
        let mut composite_file = String::new();
        let mut use_composite = false;
        let mut pan_file = String::new();
        let mut output_file = String::new();
        let mut fusion_method = String::from("brovey");

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-red" || vec[0].to_lowercase() == "--red" {
                if keyval {
                    red_file = vec[1].to_string();
                } else {
                    red_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-g" || vec[0].to_lowercase() == "-green" ||
                      vec[0].to_lowercase() == "--green" {
                if keyval {
                    green_file = vec[1].to_string();
                } else {
                    green_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-b" || vec[0].to_lowercase() == "-blue" ||
                      vec[0].to_lowercase() == "--blue" {
                if keyval {
                    blue_file = vec[1].to_string();
                } else {
                    blue_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-p" || vec[0].to_lowercase() == "-pan" ||
                      vec[0].to_lowercase() == "--pan" {
                if keyval {
                    pan_file = vec[1].to_string();
                } else {
                    pan_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-c" || vec[0].to_lowercase() == "-composite" ||
                      vec[0].to_lowercase() == "--composite" {
                if keyval {
                    composite_file = vec[1].to_string();
                } else {
                    composite_file = args[i + 1].to_string();
                }
                use_composite = true;
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "-output" ||
                      vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-method" || vec[0].to_lowercase() == "--method" {
                if keyval {
                    fusion_method = vec[1].to_string();
                } else {
                    fusion_method = args[i + 1].to_string();
                }
                if fusion_method.to_lowercase().contains("bro") {
                    fusion_method = String::from("brovey");
                } else {
                    fusion_method = String::from("ihs");
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

        if !red_file.contains(&sep) {
            red_file = format!("{}{}", working_directory, red_file);
        }
        if !green_file.contains(&sep) {
            green_file = format!("{}{}", working_directory, green_file);
        }
        if !blue_file.contains(&sep) {
            blue_file = format!("{}{}", working_directory, blue_file);
        }
        if !composite_file.contains(&sep) {
            composite_file = format!("{}{}", working_directory, composite_file);
        }
        if !pan_file.contains(&sep) {
            pan_file = format!("{}{}", working_directory, pan_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let num_procs = num_cpus::get() as isize;

        // let get_row_from_y; //: &Fn(f64) -> isize;
        // let get_column_from_x; //: &Fn(f64) -> isize;
        let mut input: Array2D<f64>;
        let rows_ms: isize;
        let columns_ms: isize;
        let mut nodata_ms = 0f64;
        let north: f64;
        let east: f64;
        let resolution_x: f64;
        let resolution_y: f64;

        if use_composite {
            if verbose {
                println!("Reading multispec image data...")
            };
            let input_c = Raster::new(&composite_file, "r")?;

            rows_ms = input_c.configs.rows as isize;
            columns_ms = input_c.configs.columns as isize;
            nodata_ms = input_c.configs.nodata;

            north = input_c.configs.north;
            east = input_c.configs.east;
            resolution_x = input_c.configs.resolution_x;
            resolution_y = input_c.configs.resolution_y;

            input = input_c.get_data_as_array2d();

        } else {
            if verbose {
                println!("Reading red band data...")
            };
            let input_r = Raster::new(&red_file, "r")?;
            if verbose {
                println!("Reading green band data...")
            };
            let input_g = Raster::new(&green_file, "r")?;
            if verbose {
                println!("Reading blue band data...")
            };
            let input_b = Raster::new(&blue_file, "r")?;

            // make sure the input files have the same size
            if input_r.configs.rows != input_g.configs.rows ||
               input_r.configs.columns != input_g.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                                      "The input files must have the same number of rows and columns and spatial extent."));
            }
            if input_r.configs.rows != input_b.configs.rows ||
               input_r.configs.columns != input_b.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                                      "The input files must have the same number of rows and columns and spatial extent."));
            }

            let nodata_r = input_r.configs.nodata;
            let nodata_g = input_g.configs.nodata;
            let nodata_b = input_b.configs.nodata;

            rows_ms = input_r.configs.rows as isize;
            columns_ms = input_r.configs.columns as isize;

            north = input_r.configs.north;
            east = input_r.configs.east;
            resolution_x = input_r.configs.resolution_x;
            resolution_y = input_r.configs.resolution_y;


            input = Array2D::new(rows_ms, columns_ms, nodata_ms, nodata_ms)?; // : Array2D<f64>
            let (mut r, mut g, mut b): (f64, f64, f64);
            let (mut r_out, mut g_out, mut b_out): (u32, u32, u32);
            let r_min = input_r.configs.display_min;
            let r_range = input_r.configs.display_max - input_r.configs.display_min;
            let g_min = input_g.configs.display_min;
            let g_range = input_g.configs.display_max - input_g.configs.display_min;
            let b_min = input_b.configs.display_min;
            let b_range = input_b.configs.display_max - input_b.configs.display_min;
            for row in 0..rows_ms {
                for col in 0..columns_ms {
                    r = input_r[(row, col)];
                    g = input_g[(row, col)];
                    b = input_b[(row, col)];
                    if r != nodata_r && g != nodata_g && b != nodata_b {
                        r = (r - r_min) / r_range * 255f64;
                        if r < 0f64 {
                            r = 0f64;
                        }
                        if r > 255f64 {
                            r = 255f64;
                        }
                        r_out = r as u32;

                        g = (g - g_min) / g_range * 255f64;
                        if g < 0f64 {
                            g = 0f64;
                        }
                        if g > 255f64 {
                            g = 255f64;
                        }
                        g_out = g as u32;

                        b = (b - b_min) / b_range * 255f64;
                        if b < 0f64 {
                            b = 0f64;
                        }
                        if b > 255f64 {
                            b = 255f64;
                        }
                        b_out = b as u32;

                        input[(row, col)] = ((255 << 24) | (b_out << 16) | (g_out << 8) | r_out) as
                                            f64;
                    }
                }
            }
        }

        let input = Arc::new(input);

        if verbose {
            println!("Reading pan image data...")
        };
        let pan = Arc::new(Raster::new(&pan_file, "r")?);
        let rows_pan = pan.configs.rows as isize;
        let columns_pan = pan.configs.columns as isize;
        let nodata_pan = pan.configs.nodata;
        let pan_min = pan.configs.display_min;
        let pan_range = pan.configs.display_max - pan.configs.display_min;

        let start = time::now();

        let mut output = Raster::initialize_using_file(&output_file, &pan);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGBA32;
        let nodata_out = 0f64;
        output.reinitialize_values(nodata_out);

        if fusion_method == String::from("brovey") {
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let pan = pan.clone();
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let get_column_from_x =
                        |x: f64| -> isize { ((x - east) / resolution_x).floor() as isize };
                    let get_row_from_y =
                        |y: f64| -> isize { ((north - y) / resolution_y).floor() as isize };
                    let mut p: f64;
                    let mut adj: f64;
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let (mut r_out, mut g_out, mut b_out): (u32, u32, u32);
                    let (mut x, mut y): (f64, f64);
                    let (mut source_col, mut source_row): (isize, isize);
                    let (mut z_ms, mut z_pan): (f64, f64);
                    for row in (0..rows_pan).filter(|r| r % num_procs == tid) {
                        y = pan.get_y_from_row(row);
                        source_row = get_row_from_y(y);
                        let mut data = vec![nodata_out; columns_pan as usize];
                        for col in 0..columns_pan {
                            x = pan.get_x_from_column(col);
                            source_col = get_column_from_x(x);
                            z_pan = pan[(row, col)];
                            z_ms = input[(source_row, source_col)];
                            if z_ms != nodata_ms && z_pan != nodata_pan {
                                p = (z_pan - pan_min) / pan_range;
                                if p < 0f64 {
                                    p = 0f64;
                                }
                                if p > 1f64 {
                                    p = 1f64;
                                }

                                r = (z_ms as u32 & 0xFF) as f64;
                                g = ((z_ms as u32 >> 8) & 0xFF) as f64;
                                b = ((z_ms as u32 >> 16) & 0xFF) as f64;

                                adj = (r + g + b) / 3f64;

                                r_out = (r * p / adj * 255f64) as u32;
                                g_out = (g * p / adj * 255f64) as u32;
                                b_out = (b * p / adj * 255f64) as u32;

                                if r_out > 255 {
                                    r_out = 255;
                                }
                                if g_out > 255 {
                                    g_out = 255;
                                }
                                if b_out > 255 {
                                    b_out = 255;
                                }

                                data[col as usize] =
                                    ((255 << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for row in 0..rows_pan {
                let data = rx.recv().unwrap();
                output.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows_pan - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

        } else {
            // ihs

            // find the overall maximum in the ms data
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut overall_max = f64::NEG_INFINITY;
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let mut z: f64;
                    for row in (0..rows_ms).filter(|r| r % num_procs == tid) {
                        for col in 0..columns_ms {
                            z = input[(row, col)];
                            if z != nodata_ms {
                                r = (z as u32 & 0xFF) as f64;
                                g = ((z as u32 >> 8) & 0xFF) as f64;
                                b = ((z as u32 >> 16) & 0xFF) as f64;

                                if r > overall_max {
                                    overall_max = r;
                                }
                                if g > overall_max {
                                    overall_max = g;
                                }
                                if b > overall_max {
                                    overall_max = b;
                                }
                            }
                        }
                    }
                    tx.send(overall_max).unwrap();
                });
            }

            let mut overall_max = f64::NEG_INFINITY;
            for tid in 0..num_procs {
                let data = rx.recv().unwrap();
                if data > overall_max {
                    overall_max = data;
                }
                if verbose {
                    progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // println!("overall max {}", overall_max);

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let pan = pan.clone();
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let get_column_from_x =
                        |x: f64| -> isize { ((x - east) / resolution_x).floor() as isize };
                    let get_row_from_y =
                        |y: f64| -> isize { ((north - y) / resolution_y).floor() as isize };
                    let mut p: f64;
                    let mut min_rgb: f64;
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let (mut i, mut h, mut s): (f64, f64, f64);
                    let (mut r_out, mut g_out, mut b_out): (u32, u32, u32);
                    let (mut x, mut y): (f64, f64);
                    let (mut source_col, mut source_row): (isize, isize);
                    let (mut z_ms, mut z_pan): (f64, f64);
                    for row in (0..rows_pan).filter(|r| r % num_procs == tid) {
                        y = pan.get_y_from_row(row);
                        source_row = get_row_from_y(y);
                        let mut data = vec![nodata_out; columns_pan as usize];
                        for col in 0..columns_pan {
                            x = pan.get_x_from_column(col);
                            source_col = get_column_from_x(x);
                            z_pan = pan[(row, col)];
                            z_ms = input[(source_row, source_col)];
                            if z_ms != nodata_ms && z_pan != nodata_pan {
                                p = (z_pan - pan_min) / pan_range;
                                if p < 0f64 {
                                    p = 0f64;
                                }
                                if p > 1f64 {
                                    p = 1f64;
                                }

                                r = (z_ms as u32 & 0xFF) as f64 / overall_max;
                                g = ((z_ms as u32 >> 8) & 0xFF) as f64 / overall_max;
                                b = ((z_ms as u32 >> 16) & 0xFF) as f64 / overall_max;

                                if r != g || g != b {
                                    // RGB to IHS transformation
                                    i = r + g + b;

                                    min_rgb = r.min(g).min(b);
                                    h = if i == 3f64 {
                                        0f64
                                    } else if b == min_rgb {
                                        (g - b) / (i - 3f64 * b)
                                    } else if r == min_rgb {
                                        (b - r) / (i - 3f64 * r) + 1f64
                                    } else {
                                        //g == min_rgb
                                        (r - g) / (i - 3f64 * g) + 2f64
                                    };

                                    s = if h <= 1f64 {
                                        (i - 3f64 * b) / i
                                    } else if h <= 2f64 {
                                        (i - 3f64 * r) / i
                                    } else {
                                        // h <= 3f64
                                        (i - 3f64 * g) / i
                                    };

                                    // update i for the panchromatic value
                                    i = p * 3f64;

                                    // IHS to RGB transformation
                                    if h <= 1f64 {
                                        r = i * (1f64 + 2f64 * s - 3f64 * s * h) / 3f64;
                                        g = i * (1f64 - s + 3f64 * s * h) / 3f64;
                                        b = i * (1f64 - s) / 3f64;
                                    } else if h <= 2f64 {
                                        r = i * (1f64 - s) / 3f64;
                                        g = i * (1f64 + 2f64 * s - 3f64 * s * (h - 1f64)) / 3f64;
                                        b = i * (1f64 - s + 3f64 * s * (h - 1f64)) / 3f64;
                                    } else {
                                        // h <= 3f64
                                        r = i * (1f64 - s + 3f64 * s * (h - 2f64)) / 3f64;
                                        g = i * (1f64 - s) / 3f64;
                                        b = i * (1f64 + 2f64 * s - 3f64 * s * (h - 2f64)) / 3f64;
                                    }
                                } else {
                                    r *= p;
                                    g *= p;
                                    b *= p;
                                }

                                r_out = (r * 255f64) as u32;
                                g_out = (g * 255f64) as u32;
                                b_out = (b * 255f64) as u32;

                                if r_out > 255 {
                                    r_out = 255;
                                }
                                if g_out > 255 {
                                    g_out = 255;
                                }
                                if b_out > 255 {
                                    b_out = 255;
                                }

                                data[col as usize] =
                                    ((255 << 24) | (b_out << 16) | (g_out << 8) | r_out) as f64;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for row in 0..rows_pan {
                let data = rx.recv().unwrap();
                output.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows_pan - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;

        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        if use_composite {
            output.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
        } else {
            output.add_metadata_entry(format!("Input red-band file: {}", red_file));
            output.add_metadata_entry(format!("Input green-band file: {}", green_file));
            output.add_metadata_entry(format!("Input blue-band file: {}", blue_file));
        }
        output.add_metadata_entry(format!("Input panchromatic file: {}", pan_file));
        output.add_metadata_entry(format!("Pan-sharpening fusion method: {}", fusion_method));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

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

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}