/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 21, 2017
Last Modified: July 17, 2017
License: MIT
*/

use std;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use lidar::*;
use tools::WhiteboxTool;

pub struct LidarJoin {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarJoin {
    pub fn new() -> LidarJoin { // public constructor
        let name = "LidarJoin".to_string();
        
        let description = "Joins multiple LiDAR (LAS) files into a single LAS file.".to_string();
        
        let mut parameters = "-i, --inputs      Input LAS files, separated by commas.\n".to_owned();
        parameters.push_str("-o, --output  Output LAS file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.las, file2.las, file3.las\" -o=outfile.las\"", short_exe, name).replace("*", &sep);
    
        LidarJoin { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarJoin {
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
        let mut input_files: String = String::new();
        let mut output_file = String::new();

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
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;
        // if !working_directory.ends_with(sep) {
        //     working_directory.push_str(&(sep.to_string()));
        // }

        if !output_file.contains(sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut output: LasFile = LasFile::new(&output_file, "w")?;

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let mut i = 0;
        let num_files = vec.len();
        let mut file_format = -1i32;
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(sep) {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                let input = match LasFile::new(&input_file, "r") {
                    Ok(lf) => lf,
                    Err(_) => return Err(Error::new(ErrorKind::NotFound, format!("No such file or directory ({})", input_file))),
                };

                if file_format == -1 {
                    file_format = input.header.point_format as i32;
                } else {
                    if input.header.point_format as i32 != file_format {
                        return Err(Error::new(ErrorKind::InvalidData, "All input files must be of the same LAS Point Format."));
                    }
                }

                if i == 0 {
                    output = LasFile::initialize_using_file(&output_file, &input);
                }

                let n_points = input.header.number_of_points as usize;

                let mut pr: LidarPointRecord;
                for i in 0..n_points {
                    pr = input.get_record(i);
                    output.add_point_record(pr);
                }
            }
            i += 1;
            if verbose { println!("Adding file: {} of {}", i, num_files); }
        }

        if verbose { println!("Writing output LAS file..."); }
        output.write()?;

        Ok(())
    }
}
