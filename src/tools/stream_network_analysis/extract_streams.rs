/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 28, 2017
Last Modified: June 28, 2017
License: MIT

NOTES: This tool should be updated to incorporate the option for an area-slope based threshold.
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

pub struct ExtractStreams {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ExtractStreams {
    pub fn new() -> ExtractStreams { // public constructor
        let name = "ExtractStreams".to_string();
        
        let description = "Extracts stream grid cells from a flow accumulation raster.".to_string();
        
        let mut parameters = "--flow_accum       Input D8 flow accumulation raster file.\n".to_owned();
        parameters.push_str("-o, --output       Output raster file.\n");
        parameters.push_str("--threshold        Threshold in flow accumulation values for channelization.\n");
        parameters.push_str("--zero_background  Flag indicating whether the background value of zero should be used.\n");
       
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --flow_accum='d8accum.dep' -o='output.dep' --threshold=100.0  --zero_background", short_exe, name).replace("*", &sep);
    
        ExtractStreams { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for ExtractStreams {
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
        let mut flow_accum_file = String::new();
        let mut output_file = String::new();
        let mut fa_threshold = 0.0;
        let mut background_val = f64::NEG_INFINITY;
        
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
            if vec[0].to_lowercase() == "-flow_accum" || vec[0].to_lowercase() == "--flow_accum" {
                if keyval {
                    flow_accum_file = vec[1].to_string();
                } else {
                    flow_accum_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-threshold" || vec[0].to_lowercase() == "--threshold" {
                if keyval {
                    fa_threshold = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    fa_threshold = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-zero_background" || vec[0].to_lowercase() == "--zero_background" || vec[0].to_lowercase() == "--esri_style" {
                background_val = 0f64;
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

        if !flow_accum_file.contains(&sep) {
            flow_accum_file = format!("{}{}", working_directory, flow_accum_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let flow_accum = Arc::new(Raster::new(&flow_accum_file, "r")?);
        
        let start = time::now();

        let rows = flow_accum.configs.rows as isize;
        let columns = flow_accum.configs.columns as isize;
        let nodata = flow_accum.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }

        let mut output = Raster::initialize_using_file(&output_file, &flow_accum);

        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut starting_row;
        let mut ending_row = 0;
        let mut id = 0;
        while ending_row < rows {
            let flow_accum = flow_accum.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in starting_row..ending_row {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = flow_accum[(row, col)];
                        if z != nodata && z > fa_threshold {
                            data[col as usize] = 1.0;
                        } else if z != nodata {
                            data[col as usize] = background_val;
                        } else {
                            data[col as usize] = nodata;
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
        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Flow accumulation file: {}", flow_accum_file));
        output.add_metadata_entry(format!("Threshold: {}", fa_threshold));
        output.add_metadata_entry(format!("Background value: {}", background_val));
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