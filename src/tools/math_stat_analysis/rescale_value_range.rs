/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 10, 2017
Last Modified: September 10, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use tools::WhiteboxTool;

pub struct RescaleValueRange {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RescaleValueRange {
    pub fn new() -> RescaleValueRange { // public constructor
        let name = "RescaleValueRange".to_string();
        
        let description = "Performs a min-max contrast stretch on an input greytone image.".to_string();
        
        let mut parameters = "-i, --input    Input raster file.\n".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--out_min_val  New minimum value in output image.\n");
        parameters.push_str("--out_max_val  New maximum value in output image.\n");
        parameters.push_str("--clip_min     Optional lower tail clip value.\n");
        parameters.push_str("--clip_max     Optional upper tail clip value.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=input.dep -o=output.dep --out_min_val=0.0 --out_max_val=1.0
>>.*{0} -r={1} --wd=\"*path*to*data*\" -i=input.dep -o=output.dep --out_min_val=0.0 --out_max_val=1.0 --clip_min=45.0 --clip_max=200.0 ", short_exe, name).replace("*", &sep);
    
        RescaleValueRange { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RescaleValueRange {
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
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut out_min_val = f64::INFINITY;
        let mut out_max_val = f64::NEG_INFINITY;
        
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
            } else if vec[0].to_lowercase() == "-clip_min" || vec[0].to_lowercase() == "--clip_min" {
                if keyval {
                    min_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    min_val = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-clip_max" || vec[0].to_lowercase() == "--clip_max" {
                if keyval {
                    max_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_val = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-out_min_val" || vec[0].to_lowercase() == "--out_min_val" {
                if keyval {
                    out_min_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    out_min_val = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-out_max_val" || vec[0].to_lowercase() == "--out_max_val" {
                if keyval {
                    out_max_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    out_max_val = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if verbose { println!("Reading input data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        if input.configs.data_type == DataType::RGB24 ||
            input.configs.data_type == DataType::RGBA32 ||
            input.configs.data_type == DataType::RGB48 ||
            input.configs.photometric_interp == PhotometricInterpretation::RGB {
            return Err(Error::new(ErrorKind::InvalidInput,
            "This tool cannot be applied to RGB colour-composite images."));
        }
        
        let start = time::now();

        if out_min_val == f64::INFINITY && out_max_val == f64::NEG_INFINITY {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Error reading the output minimum and maximum values."));
        }

        if min_val == f64::INFINITY {
            min_val = input.configs.minimum;
        }
        
        if max_val == f64::NEG_INFINITY {
            max_val = input.configs.maximum;
        }

        let value_range = max_val - min_val;
        if value_range < 0f64 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input minimum and maximum clip values are incorrect."));
        }

        let out_range = out_max_val - out_min_val;
        if out_range < 0f64 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The output minimum and maximum values are incorrect."));
        }
        
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z_in: f64;
                let mut z_out: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = input[(row, col)];
                        if z_in != nodata {
                            if z_in < min_val { z_in = min_val; }
  						    if z_in > max_val { z_in = max_val; }
	  					    z_out = out_min_val + ((z_in - min_val) / value_range) * out_range;
                            data[col as usize] = z_out;
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
        output.add_metadata_entry(format!("Minimum clip value: {}", min_val));
        output.add_metadata_entry(format!("Maximum clip value: {}", max_val));
        output.add_metadata_entry(format!("Output minimum value: {}", out_min_val));
        output.add_metadata_entry(format!("Output maximum value: {}", out_max_val));
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