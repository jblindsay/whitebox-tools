/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 18, 2017
Last Modified: July 19, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

// use std::mem;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

/// Tool struct containing the essential descriptors required to interact with the tool.
pub struct WriteFunctionMemoryInsertion {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl WriteFunctionMemoryInsertion {

    /// Public constructor.
    pub fn new() -> WriteFunctionMemoryInsertion {
        let name = "WriteFunctionMemoryInsertion".to_string();
        
        let description = "Performs a write function memory insertion for single-band multi-date change detection.".to_string();
        
        let mut parameters = "--i1, --input1   Input raster file associated with the first date.\n".to_owned();
        parameters.push_str("--i2, --input2   Input raster file associated with the second date.\n");
        parameters.push_str("--i3, --input3   Optional input raster file associated with the third date.\n");
        parameters.push_str("-o, --output  Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i1=input1.dep -i2=input2.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        WriteFunctionMemoryInsertion { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for WriteFunctionMemoryInsertion {
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
        let mut input1_file = String::new();
        let mut input2_file = String::new();
        let mut input3_file = String::new();
        let mut input3_used = false;
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
            if vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input1_file = vec[1].to_string();
                } else {
                    input1_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input2_file = vec[1].to_string();
                } else {
                    input2_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "--i3" || vec[0].to_lowercase() == "--input3" {
                if keyval {
                    input3_file = vec[1].to_string();
                } else {
                    input3_file = args[i+1].to_string();
                }
                input3_used = true;
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

        if !input1_file.contains(&sep) {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) {
            input2_file = format!("{}{}", working_directory, input2_file);
        }
        if input3_used {
            if !input3_file.contains(&sep) {
                input3_file = format!("{}{}", working_directory, input3_file);
            }
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let input_r = Arc::new(Raster::new(&input1_file, "r")?);
        let input_g = Arc::new(Raster::new(&input2_file, "r")?);
        let input_b = match input3_used {
            true => Arc::new(Raster::new(&input3_file, "r")?),
            false => Arc::new(Raster::new(&input2_file, "r")?),
        };

        let start = time::now();

        // make sure the input files have the same size
        if input_r.configs.rows != input_g.configs.rows || input_r.configs.columns != input_g.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }
        if input_r.configs.rows != input_b.configs.rows || input_r.configs.columns != input_b.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }
        
        let rows = input_r.configs.rows as isize;
        let columns = input_r.configs.columns as isize;
        let nodata_r = input_r.configs.nodata;
        let nodata_g = input_g.configs.nodata;
        let nodata_b = input_b.configs.nodata;
        let red_min = input_r.configs.display_min;
        let green_min = input_g.configs.display_min;
        let blue_min = input_b.configs.display_min;
        let red_range = input_r.configs.display_max - red_min;
        let green_range = input_g.configs.display_max - green_min;
        let blue_range = input_b.configs.display_max - blue_min;
        

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input_r = input_r.clone();
            let input_g = input_g.clone();
            let input_b = input_b.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut red_val: f64;
                let mut green_val: f64;
                let mut blue_val: f64;
                let (mut r, mut g, mut b): (u32, u32, u32);
                let alpha_mask = (255 << 24) as u32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata_r; columns as usize];
                    for col in 0..columns {
                        red_val = input_r[(row, col)];
                        green_val = input_g[(row, col)];
                        blue_val = input_b[(row, col)];
                        if red_val != nodata_r && green_val != nodata_g && blue_val != nodata_b {
                            red_val = (red_val - red_min) / red_range * 255f64;
                            if red_val < 0f64 {
                                red_val = 0f64;
                            }
                            if red_val > 255f64 {
                                red_val = 255f64;
                            }
                            r = red_val as u32;

                            green_val = (green_val - green_min) / green_range * 255f64;
                            if green_val < 0f64 {
                                green_val = 0f64;
                            }
                            if green_val > 255f64 {
                                green_val = 255f64;
                            }
                            g = green_val as u32;

                            blue_val = (blue_val - blue_min) / blue_range * 255f64;
                            if blue_val < 0f64 {
                                blue_val = 0f64;
                            }
                            if blue_val > 255f64 {
                                blue_val = 255f64;
                            }
                            b = blue_val as u32;
                            data[col as usize] = (alpha_mask | (b << 16) | (g << 8) | r) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input_r);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGB24;
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

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file Date 1: {}", input1_file));
        output.add_metadata_entry(format!("Input file Date 2: {}", input2_file));
        if input3_used {
            output.add_metadata_entry(format!("Input file Date 3: {}", input3_file));
        }
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