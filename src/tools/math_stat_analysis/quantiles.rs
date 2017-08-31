/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 1, 2017
Last Modified: July 1, 2017
License: MIT
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct Quantiles {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Quantiles {
    pub fn new() -> Quantiles { // public constructor
        let name = "Quantiles".to_string();
        
        let description = "Tranforms raster values into quantiles.".to_string();
        
        let mut parameters = "-i, --input      Input raster file.\n".to_owned();
        parameters.push_str("-o, --output     Output raster file.\n");
        parameters.push_str("--num_quantiles  Number of quantiles (default 4)");
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep --num_quantiles=5", short_exe, name).replace("*", &sep);
    
        Quantiles { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Quantiles {
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
        let mut num_quantiles = 5;
        
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
            } else if vec[0].to_lowercase() == "-num_quantiles" || vec[0].to_lowercase() == "--num_quantiles" {
                if keyval {
                    num_quantiles = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    num_quantiles = args[i+1].to_string().parse::<isize>().unwrap();
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
        let out_palette = input.configs.palette.clone();

        let start = time::now();

        let min_value = input.configs.minimum;
        let max_value = input.configs.maximum;
        let value_range = (max_value - min_value).ceil();

        let highres_num_bins = 10000isize;
	    let highres_bin_size = value_range / highres_num_bins as f64;

	    let mut primary_histo = vec![0.0; highres_num_bins as usize];
	    let mut num_valid_cells = 0;
	
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        
        let mut z: f64;
        let mut bin: isize;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    bin = ((z - min_value) / highres_bin_size).floor() as isize;
                    if bin >= highres_num_bins {
                        bin = highres_num_bins - 1;
                    }
                    primary_histo[bin as usize] += 1.0;
                    num_valid_cells += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for i in 1..highres_num_bins as usize {
            primary_histo[i] += primary_histo[i-1];
        }

        let mut cdf = vec![0.0; highres_num_bins as usize];
        for i in 0..highres_num_bins as usize {
            cdf[i] = 100.0 * primary_histo[i] as f64 / num_valid_cells as f64
        }

        let quantile_proportion = 100.0 / num_quantiles as f64;

        for i in 0..highres_num_bins as usize {
            primary_histo[i] = (cdf[i] / quantile_proportion).floor();
            if primary_histo[i] == num_quantiles as f64 {
                primary_histo[i] = num_quantiles as f64 - 1.0;
            }
        }

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    let mut i = ((z - min_value) / highres_bin_size).floor() as usize;
                    if i >= highres_num_bins as usize {
                        i = highres_num_bins as usize - 1;
                    }
                    let bin = primary_histo[i];

                    output[(row, col)] = bin + 1.0;
                }
            }
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
        output.configs.palette = out_palette;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Num. quantiles: {}", num_quantiles));
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