/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 22 2017
Last Modified: July 22, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct CumulativeDistribution {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl CumulativeDistribution {
    pub fn new() -> CumulativeDistribution { // public constructor
        let name = "CumulativeDistribution".to_string();
        
        let description = "Converts a raster image to its cumulative distribution function.".to_string();
        
        let mut parameters = "-i, --input    Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        CumulativeDistribution { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for CumulativeDistribution {
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

        let input = Raster::new(&input_file, "r")?;
        let start = time::now();
        
        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;

        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        let range = max_val - min_val;
        let num_bins = 50000;
        let bin_size = range / num_bins as f64;
        let mut histogram = vec![0usize; num_bins];
        let mut bin_num: usize;
        let num_bins_less_one = num_bins - 1;
        let mut num_cells = 0usize;
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    num_cells += 1;
                    bin_num = ((z - min_val) / bin_size) as usize;
                    if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                    histogram[bin_num] += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut cdf = vec![0f64; num_bins];
        cdf[0] = histogram[0] as f64; 
        for i in 1..histogram.len() {
            cdf[i] = cdf[i - 1] + histogram[i] as f64;
        }

        for i in 0..histogram.len() {
            cdf[i] = cdf[i] / num_cells as f64;
        }

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    bin_num = ((z - min_val) / bin_size) as usize;
                    if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                    output[(row, col)] = cdf[bin_num];
                }
            }
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