/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 28, 2017
Last Modified: June 28, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use structures::Array2D;
use tools::WhiteboxTool;

pub struct Closing {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Closing {
    pub fn new() -> Closing { // public constructor
        let name = "Closing".to_string();
        
        let description = "A closing is a mathematical morphology operating involving an erosion (min filter) of a dilation (max filter) set.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.".to_owned();
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
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --filter=25", short_exe, name).replace("*", &sep);
    
        Closing { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Closing {
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
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i+1].to_string().parse::<usize>().unwrap();
                }
                filter_size_y = filter_size_x;
            } else if vec[0].to_lowercase() == "-filterx" || vec[0].to_lowercase() == "--filterx" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i+1].to_string().parse::<usize>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-filtery" || vec[0].to_lowercase() == "--filtery" {
                if keyval {
                    filter_size_y = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_y = args[i+1].to_string().parse::<usize>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if filter_size_x < 3 { filter_size_x = 3; }
        if filter_size_y < 3 { filter_size_y = 3; }

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

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        // first perform the dilation
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let mut starting_row;
        let mut ending_row = 0;
        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_row < rows {
            let input = input.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut z_n, mut z) : (f64, f64);
                let mut max_val: f64;
                let (mut start_col, mut end_col, mut start_row, mut end_row): (isize, isize, isize, isize);
                for row in starting_row..ending_row {
                    let mut filter_max_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if col > 0 {
                            filter_max_vals.pop_front();
                            max_val = f64::NEG_INFINITY;
                            for row2 in start_row..end_row+1 {
                                z_n = input.get_value(row2, col + midpoint_x);
                                if z_n != nodata {
                                    if z_n > max_val { max_val = z_n; }
                                }
                            }
                            filter_max_vals.push_back(max_val);
                        } else {
                            // initialize the filter_vals
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            for col2 in start_col..end_col+1 {
                                max_val = f64::NEG_INFINITY;
                                for row2 in start_row..end_row+1 {
                                    z_n = input.get_value(row2, col2);
                                    if z_n != nodata {
                                        if z_n > max_val { max_val = z_n; }
                                    }
                                }
                                filter_max_vals.push_back(max_val);
                            }
                        }
                        z = input.get_value(row, col);
                        if z != nodata {
                            max_val = f64::NEG_INFINITY;
                            for i in 0..filter_size_x {
                                if filter_max_vals[i] > max_val { max_val = filter_max_vals[i]; }
                            }
                            if max_val > f64::NEG_INFINITY {
                                data[col as usize] = max_val;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut dilation: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            dilation.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now perform the erosion
        let dilation = Arc::new(dilation); // wrap the erosion result in an Arc
        ending_row = 0;
        let (tx, rx) = mpsc::channel();
        id = 0;
        while ending_row < rows {
            let input = dilation.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut z_n, mut z) : (f64, f64);
                let mut min_val: f64;
                let (mut start_col, mut end_col, mut start_row, mut end_row): (isize, isize, isize, isize);
                for row in starting_row..ending_row {
                    let mut filter_min_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if col > 0 {
                            filter_min_vals.pop_front();
                            min_val = f64::INFINITY;
                            for row2 in start_row..end_row+1 {
                                z_n = input.get_value(row2, col + midpoint_x);
                                if z_n != nodata {
                                    if z_n < min_val { min_val = z_n; }
                                }
                            }
                            filter_min_vals.push_back(min_val);
                        } else {
                            // initialize the filter_vals
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            for col2 in start_col..end_col+1 {
                                min_val = f64::INFINITY;
                                for row2 in start_row..end_row+1 {
                                    z_n = input.get_value(row2, col2);
                                    if z_n != nodata {
                                        if z_n < min_val { min_val = z_n; }
                                    }
                                }
                                filter_min_vals.push_back(min_val);
                            }
                        }
                        z = input.get_value(row, col);
                        if z != nodata {
                            min_val = f64::INFINITY;
                            for i in 0..filter_size_x {
                                if filter_min_vals[i] < min_val { min_val = filter_min_vals[i]; }
                            }
                            if min_val < f64::INFINITY {
                                data[col as usize] = min_val;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
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

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}