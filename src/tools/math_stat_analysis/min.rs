/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 6, 2017
Last Modified: July 6, 2017
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
use tools::WhiteboxTool;

pub struct Min {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Min {
    /// public constructor
    pub fn new() -> Min { 
        let name = "Min".to_string();
        
        let description = "Performs a MIN operation on two rasters or a raster and a constant value.".to_string();
        
        let mut parameters = "--input1       Input raster file or constant value.".to_owned();
        parameters.push_str("--input2       Input raster file or constant value.\n");
        parameters.push_str("-o, --output   Output raster file.\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --input1='in1.dep' --input2='in2.dep' -o=output.dep", short_exe, name).replace("*", &sep);
    
        Min { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Min {
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
        let mut input1 = String::new();
        let mut input2 = String::new();
        let mut output_file = String::new();
         
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
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input1 = vec[1].to_string();
                } else {
                    input1 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input2 = vec[1].to_string();
                } else {
                    input2 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
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

        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // Are either of the inputs constants?
        let mut input1_constant = f64::NEG_INFINITY;
        let input1_is_constant = match input1.parse::<f64>() {
            Ok(val) => { 
                input1_constant = val;
                true
            },
            Err(_) => false,
        };
        if !input1_is_constant {
            if !input1.contains(&sep) {
                input1 = format!("{}{}", working_directory, input1);
            }
        }

        let mut input2_constant = f64::NEG_INFINITY;
        let input2_is_constant = match input2.parse::<f64>() {
            Ok(val) => { 
                input2_constant = val;
                true
            },
            Err(_) => false,
        };
        if !input2_is_constant {
            if !input2.contains(&sep) {
                input2 = format!("{}{}", working_directory, input2);
            }
        }

        if input1_is_constant && input2_is_constant {
            // return Err(Error::new(ErrorKind::InvalidInput,
            //                     "At least one of the inputs must be a raster."));
            println!("{}", input1_constant.min(input2_constant));
            return Ok(());
        } else if input1_is_constant && !input2_is_constant {
            if verbose { println!("Reading data...") };
            let in2 = Arc::new(Raster::new(&input2, "r")?);

            let start = time::now();
            let rows = in2.configs.rows as isize;
            let columns = in2.configs.columns as isize;
            let nodata2 = in2.configs.nodata;

            let mut starting_row;
            let mut ending_row = 0;
            let num_procs = num_cpus::get() as isize;
            let row_block_size = rows / num_procs;
            let (tx, rx) = mpsc::channel();
            let mut id = 0;
            while ending_row < rows {
                let in2 = in2.clone();
                starting_row = id * row_block_size;
                ending_row = starting_row + row_block_size;
                if ending_row > rows {
                    ending_row = rows;
                }
                id += 1;
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z2: f64;
                    for row in starting_row..ending_row {
                        let mut data: Vec<f64> = vec![nodata2; columns as usize];
                        for col in 0..columns {
                            z2 = in2[(row, col)];
                            if z2 != nodata2 {
                                data[col as usize] = input1_constant.min(z2);
                            } else {
                                data[col as usize] = nodata2;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&output_file, &in2);
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
                
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let end = time::now();
            let elapsed_time = end - start;
            output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        } else if !input1_is_constant && input2_is_constant {
            if verbose { println!("Reading data...") };
            let in1 = Arc::new(Raster::new(&input1, "r")?);
            
            let start = time::now();
            let rows = in1.configs.rows as isize;
            let columns = in1.configs.columns as isize;
            let nodata1 = in1.configs.nodata;
            
            let mut starting_row;
            let mut ending_row = 0;
            let num_procs = num_cpus::get() as isize;
            let row_block_size = rows / num_procs;
            let (tx, rx) = mpsc::channel();
            let mut id = 0;
            while ending_row < rows {
                let in1 = in1.clone();
                starting_row = id * row_block_size;
                ending_row = starting_row + row_block_size;
                if ending_row > rows {
                    ending_row = rows;
                }
                id += 1;
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z1: f64;
                    for row in starting_row..ending_row {
                        let mut data: Vec<f64> = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            z1 = in1[(row, col)];
                            if z1 != nodata1 {
                                data[col as usize] = z1.min(input2_constant);
                            } else {
                                data[col as usize] = nodata1;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&output_file, &in1);
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
                
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let end = time::now();
            let elapsed_time = end - start;
            output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        } else { // !input1_is_constant && !input2_is_constant
            if verbose { println!("Reading data...") };
            let in1 = Arc::new(Raster::new(&input1, "r")?);
            let in2 = Arc::new(Raster::new(&input2, "r")?);

            let start = time::now();
            let rows = in1.configs.rows as isize;
            let columns = in1.configs.columns as isize;
            let nodata1 = in1.configs.nodata;
            let nodata2 = in2.configs.nodata;

            // make sure the input files have the same size
            if in1.configs.rows != in2.configs.rows || in1.configs.columns != in2.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                                    "The input files must have the same number of rows and columns and spatial extent."));
            }
            
            let mut starting_row;
            let mut ending_row = 0;
            let num_procs = num_cpus::get() as isize;
            let row_block_size = rows / num_procs;
            let (tx, rx) = mpsc::channel();
            let mut id = 0;
            while ending_row < rows {
                let in1 = in1.clone();
                let in2 = in2.clone();
                starting_row = id * row_block_size;
                ending_row = starting_row + row_block_size;
                if ending_row > rows {
                    ending_row = rows;
                }
                id += 1;
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z1: f64;
                    let mut z2: f64;
                    for row in starting_row..ending_row {
                        let mut data: Vec<f64> = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            z1 = in1[(row, col)];
                            z2 = in2[(row, col)];
                            if z1 != nodata1 && z2 != nodata2 {
                                data[col as usize] = z1.min(z2);
                            } else {
                                data[col as usize] = nodata1;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&output_file, &in1);
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
                
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let end = time::now();
            let elapsed_time = end - start;
            output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        }
        
        Ok(())
    }
}