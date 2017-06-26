/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: June 26, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::i32;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use structures::Array2D;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct DevFromMeanElev {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DevFromMeanElev {
    pub fn new() -> DevFromMeanElev { // public constructor
        let name = "DevFromMeanElev".to_string();
        
        let description = "Calculates deviation from mean elevation.".to_string();
        
        let mut parameters = "-i, --input   Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--filter      Size of the filter kernel (default is 11).\n");
        parameters.push_str("--filterx     Optional size of the filter kernel in the x-direction (default is 11; not used if --filter is specified).\n");
        parameters.push_str("--filtery     Optional size of the filter kernel in the y-direction (default is 11; not used if --filter is specified).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep --filter=25", short_exe, name).replace("*", &sep);
    
        DevFromMeanElev { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DevFromMeanElev {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut filter_size_x = 11usize;
        let mut filter_size_y = 11usize;
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<usize>().unwrap();
                }
                filter_size_y = filter_size_x;
            } else if vec[0].to_lowercase() == "-filterx" || vec[0].to_lowercase() == "--filterx" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<usize>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-filtery" || vec[0].to_lowercase() == "--filtery" {
                if keyval {
                    filter_size_y = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_y = args[i + 1].to_string().parse::<usize>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if filter_size_x < 3 {
            filter_size_x = 3;
        }
        if filter_size_y < 3 {
            filter_size_y = 3;
        }

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size_x as f64 / 2f64).floor() == (filter_size_x as f64 / 2f64) {
            filter_size_x += 1;
        }
        if (filter_size_y as f64 / 2f64).floor() == (filter_size_y as f64 / 2f64) {
            filter_size_y += 1;
        }

        let midpoint_x = (filter_size_x as f64 / 2f64).floor() as isize;
        let midpoint_y = (filter_size_y as f64 / 2f64).floor() as isize;
        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        // first bin the data
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let num_sig_digits = 2;
        let multiplier = 10f64.powi(num_sig_digits);
        let min_val = input.configs.minimum;
        let min_bin = (min_val * multiplier).floor() as i32;
        let bin_nodata32 = i32::MIN;

        let mut binned_data: Array2D<i32> = Array2D::new(rows, columns, bin_nodata32, bin_nodata32)?;

        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();

        let mut starting_row;
        let mut ending_row = 0;
        let mut id = 0;
        while ending_row < rows {
            let input = input.clone();
            let rows = rows.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let mut z: f64;
                let mut val: i32;
                for row in starting_row..ending_row {
                    let mut data = vec![bin_nodata32; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            val = (z * multiplier).floor() as i32 - min_bin;
                            data[col as usize] = val;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            binned_data.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Binning data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // create the integral images
        let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral2: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, bin_nodata32)?;

        let mut val: f64;
        let mut sum: f64;
        let mut sum_sqr: f64;
        let mut sum_n: i32;
        let (mut i_prev, mut i2_prev): (f64, f64);
        let mut n_prev: i32;
        for row in 0..rows {
            sum = 0f64;
            sum_sqr = 0f64;
            sum_n = 0;
            for col in 0..columns {
                val = binned_data.get_value(row, col) as f64;
                if val == bin_nodata32 as f64 {
                    val = 0f64;
                } else {
                    sum_n += 1;
                }
                sum += val;
                sum_sqr += val * val;
                if row > 0 {
                    i_prev = integral.get_value(row - 1, col);
                    i2_prev = integral2.get_value(row - 1, col);
                    n_prev = integral_n.get_value(row - 1, col);
                    integral.set_value(row, col, sum + i_prev);
                    integral2.set_value(row, col, sum_sqr + i2_prev);
                    integral_n.set_value(row, col, sum_n + n_prev);
                } else {
                    integral.set_value(row, col, sum);
                    integral2.set_value(row, col, sum_sqr);
                    integral_n.set_value(row, col, sum_n);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating integral images: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let bd = Arc::new(binned_data); // wrap binned_data in an Arc
        let i = Arc::new(integral); // wrap integral in an Arc
        let i2 = Arc::new(integral2); // wrap integral2 in an Arc
        let i_n = Arc::new(integral_n); // wrap integral_n in an Arc
        let mut output = Raster::initialize_using_file(&output_file, &input);
        ending_row = 0;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_row < rows {
            let binned_data = bd.clone();
            let i = i.clone();
            let i2 = i2.clone();
            let i_n = i_n.clone();
            let rows = rows.clone();
            let columns = columns.clone();
            let nodata = nodata.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                let mut n: i32;
                let (mut sum, mut sum_sqr): (f64, f64);
                let (mut v, mut s, mut mean): (f64, f64, f64);
                let mut z: i32;
                for row in starting_row..ending_row {
                    y1 = row - midpoint_y - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    if y1 >= rows {
                        y1 = rows - 1;
                    }

                    y2 = row + midpoint_y;
                    if y2 < 0 {
                        y2 = 0;
                    }
                    if y2 >= rows {
                        y2 = rows - 1;
                    }
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = binned_data.get_value(row, col);
                        if z != bin_nodata32 {
                            x1 = col - midpoint_x - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }
                            if x1 >= columns {
                                x1 = columns - 1;
                            }

                            x2 = col + midpoint_x;
                            if x2 < 0 {
                                x2 = 0;
                            }
                            if x2 >= columns {
                                x2 = columns - 1;
                            }
                            n = i_n.get_value(y2, x2) + i_n.get_value(y1, x1) - i_n.get_value(y1, x2) -
                                i_n.get_value(y2, x1);
                            if n > 0 {
                                sum = i.get_value(y2, x2) + i.get_value(y1, x1) - i.get_value(y1, x2) -
                                    i.get_value(y2, x1);
                                sum_sqr = i2.get_value(y2, x2) + i2.get_value(y1, x1) -
                                        i2.get_value(y1, x2) -
                                        i2.get_value(y2, x1);
                                v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                if v > 0f64 {
                                    s = v.sqrt();
                                    mean = sum / n as f64;
                                    data[col as usize] = (z as f64 - mean) / s;
                                } else {
                                    data[col as usize] = 0f64;
                                }
                            } else {
                                data[col as usize] = 0f64;
                            }
                        }
                    }

                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.display_min = -2.0;
        output.configs.display_max = 2.0;
        output.configs.palette = "blue_white_red.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                        self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
        output
            .add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT",
                                                                                                ""));

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
            println!("{}",
                    &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}