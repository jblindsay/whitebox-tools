/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: June 27, 2017
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

pub struct LaplacianFilter {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LaplacianFilter {
    pub fn new() -> LaplacianFilter { // public constructor
        let name = "LaplacianFilter".to_string();
        
        let description = "Performs a Laplacian filter on an image.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--variant     Optional variant value. Options include 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), and 5x5(2) (default is 3x3(1)).\n");
        parameters.push_str("--clip        Optional amount to clip the distribution tails by, in percent (default is 0.0).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --variant='3x3(1)' --clip=1.0", short_exe, name).replace("*", &sep);
    
        LaplacianFilter { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LaplacianFilter {
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
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut variant = "3x3(1)".to_string();
        let mut clip_amount = 0.0;
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
            } else if vec[0].to_lowercase() == "-variant" || vec[0].to_lowercase() == "--variant" {
                if keyval {
                    variant = vec[1].to_string();
                } else {
                    variant = args[i + 1].to_string();
                }
                // if variant.contains("5") {
                //     variant = "5x5".to_string();
                // } else {
                //     variant = "3x3".to_string();
                // }
            } else if vec[0].to_lowercase() == "-clip" || vec[0].to_lowercase() == "--clip" {
                if keyval {
                    clip_amount = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    clip_amount = args[i + 1].to_string().parse::<f64>().unwrap();
                }
                if clip_amount < 0.0 { clip_amount == 0.0; }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
    
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut starting_row;
        let mut ending_row = 0;
        let mut id = 0;
        while ending_row < rows {
            let input = input.clone();
            let variant = variant.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut zn: f64;
                let weights: Vec<f64>;
                let dx: Vec<isize>;
                let dy: Vec<isize>; 

                if variant.contains("3x3(1)") {
                    weights = vec![ 0.0, -1.0, 0.0, -1.0, 4.0, -1.0, 0.0, -1.0, 0.0 ];
                    dx = vec![ -1, 0, 1, -1, 0, 1, -1, 0, 1 ];
                    dy = vec![ -1, -1, -1, 0, 0, 0, 1, 1, 1 ];
                } else if variant.contains("3x3(2)") {
                    weights = vec![ 0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0 ];
                    dx = vec![ -1, 0, 1, -1, 0, 1, -1, 0, 1 ];
                    dy = vec![ -1, -1, -1, 0, 0, 0, 1, 1, 1 ];
                } else if variant.contains("3x3(3)") {
                    weights = vec![ -1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0 ];
                    dx = vec![ -1, 0, 1, -1, 0, 1, -1, 0, 1 ];
                    dy = vec![ -1, -1, -1, 0, 0, 0, 1, 1, 1 ];
                } else if variant.contains("3x3(4)") {
                    weights = vec![ 1.0, -2.0, 1.0, -2.0, 4.0, -2.0, 1.0, -2.0, 1.0 ];
                    dx = vec![ -1, 0, 1, -1, 0, 1, -1, 0, 1 ];
                    dy = vec![ -1, -1, -1, 0, 0, 0, 1, 1, 1 ];
                } else if variant.contains("5x5(1)") {
                    weights = vec![ 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 
                        17.0, -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0 ];
                    dx = vec![ -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 
                        2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2 ];
                    dy = vec![ -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 
                        0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2 ];
                } else { // 5 x 5 (2)
                    weights = vec![ 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 
                        16.0, -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0 ];
                    dx = vec![ -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 
                        2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2 ];
                    dy = vec![ -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 
                        0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2 ];
                }

                let num_pixels_in_filter = dx.len();
                let mut sum: f64;
                for row in starting_row..ending_row {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            sum = 0.0;
                            for i in 0..num_pixels_in_filter {
                                zn = input[(row + dy[i], col + dx[i])];
                                if zn == nodata {
                                    zn = z; // replace it with z
                                }
                                sum += zn * weights[i];
                            }
                            data[col as usize] = sum;
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
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if clip_amount > 0.0 {
            println!("Clipping output...");
            output.clip_min_and_max_by_percent(clip_amount);
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Variant: {}", variant));
        output.add_metadata_entry(format!("Clip amount: {}", clip_amount));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

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