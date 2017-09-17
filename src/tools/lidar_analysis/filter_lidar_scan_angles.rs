/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 17, 2017
Last Modified: September 17, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::f64;
use std::path;
use std::io::{Error, ErrorKind};
use lidar::*;
use tools::WhiteboxTool;

pub struct FilterLidarScanAngles {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl FilterLidarScanAngles {
    pub fn new() -> FilterLidarScanAngles { // public constructor
        let name = "FilterLidarScanAngles".to_string();
        
        let description = "Removes points in a LAS file with scan angles greater than a threshold.".to_string();
        
        let mut parameters = "-i, --input    Input LAS file.\n".to_owned();
        parameters.push_str("-o, --output   Output LAS file.\n");
        parameters.push_str("--threshold    Scan angle threshold.\n");
  
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --threshold=10.0", short_exe, name).replace("*", &sep);
    
        FilterLidarScanAngles { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for FilterLidarScanAngles {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut threshold = 0i8;
        
        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 { keyval = true; }
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
            } else if vec[0].to_lowercase() == "-threshold" || vec[0].to_lowercase() == "--threshold" {
                if keyval {
                    threshold = vec[1].to_string().parse::<i8>().unwrap().abs();
                } else {
                    threshold = args[i+1].to_string().parse::<i8>().unwrap().abs();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading input LAS file..."); }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let start = time::now();

        if verbose { println!("Performing analysis..."); }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        for i in 0..n_points {
            if input[i].scan_angle.abs() <= threshold {
                let pr = input.get_record(i);
                let pr2 = match pr {
                    LidarPointRecord::PointRecord0 { point_data }  => {
                        LidarPointRecord::PointRecord0 { point_data: point_data }
                    },
                    LidarPointRecord::PointRecord1 { point_data, gps_data } => {
                        LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data }
                    },
                    LidarPointRecord::PointRecord2 { point_data, rgb_data } => {
                        LidarPointRecord::PointRecord2 { point_data: point_data, rgb_data: rgb_data }
                    },
                    LidarPointRecord::PointRecord3 { point_data, gps_data, rgb_data } => {
                        LidarPointRecord::PointRecord3 { point_data: point_data, gps_data: gps_data, rgb_data: rgb_data }
                    },
                };
                output.add_point_record(pr2);
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;

        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}