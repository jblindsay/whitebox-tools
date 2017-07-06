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

pub struct ReclassEqualInterval {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ReclassEqualInterval {
    /// public constructor
    pub fn new() -> ReclassEqualInterval { 
        let name = "ReclassEqualInterval".to_string();
        
        let description = "Reclassifies the values in a raster image based on equal-ranges.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--interval    Class interval size (default is 10.0).\n");
        parameters.push_str("--start_val   Optional starting value (default is input minimum value).\n");
        parameters.push_str("--end_val     Optional ending value (default is input maximum value).\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i='input.dep' -o=output.dep --interval=10.0 --start_val=0.0", short_exe, name).replace("*", &sep);
    
        ReclassEqualInterval { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for ReclassEqualInterval {
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
        let mut interval_size = 10.0;
        let mut start_val = f64::NEG_INFINITY;
        let mut end_val = f64::INFINITY;
         
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
            } else if vec[0].to_lowercase() == "-interval" || vec[0].to_lowercase() == "--interval" {
                if keyval {
                    interval_size = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    interval_size = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-start_val" || vec[0].to_lowercase() == "--start_val" {
                if keyval {
                    start_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    start_val = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-end_val" || vec[0].to_lowercase() == "--end_val" {
                if keyval {
                    end_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    end_val = args[i+1].to_string().parse::<f64>().unwrap();
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

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        if start_val == f64::NEG_INFINITY {
            start_val = input.configs.minimum;
        }
        if end_val == f64::INFINITY {
            end_val = input.configs.maximum;
        }

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
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in starting_row..ending_row {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            if z >= start_val && z <= end_val {
                                z = (z / interval_size).floor() * interval_size;
                            }
                            data[col as usize] = z;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
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
        output.add_metadata_entry(format!("Input file: {}", input_file));
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