/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 13, 2017
Last Modified: July 13, 2017
License: MIT

NOTES: The tool should be updated to take multiple file inputs.
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

pub struct SigmoidalContrastStretch {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl SigmoidalContrastStretch {
    pub fn new() -> SigmoidalContrastStretch { // public constructor
        let name = "SigmoidalContrastStretch".to_string();
        
        let description = "Performs a sigmoidal contrast stretch on input images.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--cutoff      Cutoff value between 0.0 and 0.95 (default is 0.0).\n");
        parameters.push_str("--gain        Gain value between 0.0 and 0.95 (default is 1.0).\n");
        parameters.push_str("--num_tones   Number of tones in the output image (default is 256).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=input.dep -o=output.dep --cutoff=0.1 --gain=2.0 --num_tones=1024", short_exe, name).replace("*", &sep);
    
        SigmoidalContrastStretch { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for SigmoidalContrastStretch {
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
        let mut cutoff = 0.0;
        let mut gain = 1.0;
        let mut num_tones = 256f64;
        
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
            } else if vec[0].to_lowercase() == "-cutoff" || vec[0].to_lowercase() == "--cutoff" {
                if keyval {
                    cutoff = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    cutoff = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-gain" || vec[0].to_lowercase() == "--gain" {
                if keyval {
                    gain = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    gain = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-num_tones" || vec[0].to_lowercase() == "--num_tones" {
                if keyval {
                    num_tones = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    num_tones = args[i + 1].to_string().parse::<f64>().unwrap();
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
            input.configs.data_type == DataType::RGB48 ||
            input.configs.data_type == DataType::RGBA32 {

            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool is intended to be applied to single-band greyscale rasters and not RGB colour-composite images."));
        }
        
        let start = time::now();

        if cutoff < 0.0 { cutoff = 0f64; }
        if cutoff > 0.95 { cutoff = 0.95; }
        
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        let value_range = max_val - min_val;
        
        let a = 1f64/(1f64+(gain * cutoff).exp());
        let b = 1f64/(1f64+(gain*(cutoff-1f64)).exp()) - 1f64/(1f64+(gain*cutoff).exp());
        
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
                            z_out = (z_in - min_val) / value_range;
                            z_out = (1f64/(1f64+(gain*(cutoff-z_out)).exp()) - a ) / b;
                            z_out = (z_out * num_tones).floor();
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
        output.add_metadata_entry(format!("Cutoff value: {}", cutoff));
        output.add_metadata_entry(format!("Gain value: {}", gain));
        output.add_metadata_entry(format!("Number of tones: {}", num_tones));
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