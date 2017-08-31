/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 22 2017
Last Modified: July 22, 2017
License: MIT

NOTES: Will need to add support for vector polygons eventually.
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct Centroid {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Centroid {
    pub fn new() -> Centroid { // public constructor
        let name = "Centroid".to_string();
        
        let description = "Calclates the centroid, or average location, of raster polygon objects.".to_string();
        
        let mut parameters = "-i, --input    Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--text_output  Optional text output.\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep
>>.*{0} -r={1} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep --text_output", short_exe, name).replace("*", &sep);
    
        Centroid { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Centroid {
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
        let mut text_output = false;
        
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
            } else if vec[0].to_lowercase() == "-text_output" || vec[0].to_lowercase() == "--text_output" {
                text_output = true;
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

        let min_val = input.configs.minimum.floor() as usize;
        let max_val = input.configs.maximum.ceil() as usize;
        let range = max_val - min_val;
        
        let mut total_columns = vec![0usize; range + 1];
        let mut total_rows = vec![0usize; range + 1];
        let mut total_n = vec![0usize; range + 1];
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut z: f64;
        let mut a: usize;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z > 0f64 && z != nodata {
                    a = (z - min_val as f64) as usize;
                    total_columns[a] += col as usize;
                    total_rows[a] += row as usize;
                    total_n[a] += 1usize;
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

        let mut col: isize;
        let mut row: isize;
        for a in 0..range+1 {
            if total_n[a] > 0 {
                col = (total_columns[a] / total_n[a]) as isize;
                row = (total_rows[a] / total_n[a]) as isize;
                output.set_value(row, col, (a + min_val) as f64);
            }
        }

        if text_output {
            let mut col: f64;
            let mut row: f64;
            println!("Patch Centroid\nPatch ID\tColumn\tRow");
            for a in 0..range+1 {
                if total_n[a] > 0 {
                    col = total_columns[a] as f64 / total_n[a] as f64;
                    row = total_rows[a] as f64 / total_n[a] as f64;
                    println!("{}\t{}\t{}", (a + min_val), col, row);
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