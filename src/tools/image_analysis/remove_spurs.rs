/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 5, 2017
Last Modified: July 5, 2017
License: MIT

NOTE: This algorithm can't easily be parallelized because the output raster must be read 
and written to during the same loop. Doing so would involve using a mutex.
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::WhiteboxTool;

pub struct RemoveSpurs {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RemoveSpurs {
    pub fn new() -> RemoveSpurs { // public constructor
        let name = "RemoveSpurs".to_string();
        
        let description = "Removes the spurs (prunning operation) from a Boolean line image.; intended to be used on the output of the LineThinning tool.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--iterations  Maximum number of iterations (default is 10).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" --input=DEM.dep -o=output.dep --iterations=10", short_exe, name).replace("*", &sep);
    
        RemoveSpurs { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RemoveSpurs {
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
        let mut max_iterations = 10;
        
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
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
            } else if vec[0].to_lowercase() == "-iterations" || vec[0].to_lowercase() == "--iterations" {
                if keyval {
                    max_iterations = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    max_iterations = args[i + 1].to_string().parse::<usize>().unwrap();
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
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
                
        let start = time::now();

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
                for row in starting_row..ending_row {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if input[(row, col)] > 0.0 && input[(row, col)] != nodata {
                            data[col as usize] = 1.0;
                        } else if input[(row, col)] == 0.0 {
                            data[col as usize] = 0.0;
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
                    println!("Initializing output: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut did_something: bool;
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        
        let elements = vec![ vec![ 0, 1, 4, 5, 6, 7 ], vec![ 0, 1, 2, 5, 6, 7 ], 
            vec![ 0, 1, 2, 3, 6, 7 ], vec![ 0, 1, 2, 3, 4, 7 ], 
            vec![ 0, 1, 2, 3, 4, 5 ], vec![ 1, 2, 3, 4, 5, 6 ], 
            vec![ 2, 3, 4, 5, 6, 7 ], vec![ 0, 3, 4, 5, 6, 7 ] ];

        let mut neighbours = [0.0; 8];
        let mut pattern_match: bool;
        let mut z: f64;
        for loop_num in 0..max_iterations {
            did_something = false;
            if loop_num % 2 == 1 {
                for a in 0..8 {
                    for row in 0..rows {
                        for col in 0..columns {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }
                                
                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as usize;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num + 1, progress);
                            old_progress = progress;
                        }
                    }
                }
            } else {
                for a in 0..8 {
                    for row in (0..rows).rev() {
                        for col in (0..columns).rev() {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }
                                
                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as usize;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num + 1, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            if !did_something {
                break;
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Max iterations: {}", max_iterations));
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