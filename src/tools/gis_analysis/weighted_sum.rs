/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 22 2017
Last Modified: June 22, 2017
License: MIT
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct WeightedSum {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl WeightedSum {
    pub fn new() -> WeightedSum { // public constructor
        let name = "WeightedSum".to_string();
        
        let description = "Performs a weighted-sum overlay on multiple input raster images.".to_string();
        
        let mut parameters = "-i, --inputs     Input raster files, contained in quotes and separated by commas or semicolons.\n".to_owned();
        parameters.push_str("-o, --output     Output raster file.\n");
        parameters.push_str("-w, --weights    Weight values, contained in quotes and separated by commas or semicolons.\n");
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd='*path*to*data*' -i='image1.dep;image2.dep;image3.dep' -o=output.dep --weights='0.3;0.2;0.5'", short_exe, name).replace("*", &sep);
    
        WeightedSum { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for WeightedSum {
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
        let mut input_files = String::new();
        let mut output_file = String::new();
        let mut weights_list = String::new();
        
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--inputs" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-w" || vec[0].to_lowercase() == "--weights" {
                if keyval {
                    weights_list = vec[1].to_string();
                } else {
                    weights_list = args[i+1].to_string();
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

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        let start = time::now();

        // Parse the weights list and convert it into numbers
        cmd = weights_list.split(";");
        let mut weights_str = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = weights_list.split(",");
            weights_str = cmd.collect::<Vec<&str>>();
        }
        let num_weights = weights_str.len();
        if num_weights != num_files {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The number of weights specified must equal the number of input files."));
        }
        let mut weights = vec![];
        for w in weights_str {
            weights.push(w.to_string().parse::<f64>().unwrap());
        }

        // make sure that the weights sum to 1.0
        let mut weight_sum = 0.0f64;
        for i in 0..num_weights {
            weight_sum += weights[i];
        }
        for i in 0..num_weights {
            weights[i] /= weight_sum;
        }

        // We need to initialize output here, but in reality this can't be done
        // until we know the size of rows and columns, which occurs during the first loop.
        let mut output: Raster = Raster::new(&output_file, "w")?;
        let mut rows = 0isize;
        let mut columns = 0isize;
        let mut in_nodata: f64;
        let mut out_nodata: f64 = -32768.0f64;
        let mut in_val: f64;
        let mut read_first_file = false;
        let mut i = 1;
        let mut j = 0usize;
        for value in vec {
            if !value.trim().is_empty() {
                if verbose { println!("Reading data...") };

                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;
                in_nodata = input.configs.nodata;
                if !read_first_file {
                    read_first_file = true;
                    rows = input.configs.rows as isize;
                    columns = input.configs.columns as isize;
                    out_nodata = in_nodata;
                    
                    // initialize the output file and low_val
                    output = Raster::initialize_using_file(&output_file, &input);
                    output.reinitialize_values(0.0);
                }
                // check to ensure that all inputs have the same rows and columns
                if input.configs.rows as isize != rows || input.configs.columns as isize != columns {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
                }

                for row in 0..rows {
                    for col in 0..columns {
                        if output[(row, col)] != out_nodata {
                            in_val = input[(row, col)];
                            if in_val != in_nodata {
                                output.increment(row, col, in_val * weights[j]);
                            } else {
                                output[(row, col)] = out_nodata;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress (loop {} of {}): {}%", i, num_files, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
            j += 1;
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}