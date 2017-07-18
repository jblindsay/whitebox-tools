/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 16, 2017
Last Modified: July 17, 2017
License: MIT
*/
extern crate time;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use lidar::*;
use tools::WhiteboxTool;

pub struct LasToAscii {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LasToAscii {
    pub fn new() -> LasToAscii { // public constructor
        let name = "LasToAscii".to_string();
        
        let description = "Converts one or more LAS files into ASCII text files.".to_string();
        
        let parameters = "-i, --inputs      Input LAS files, separated by commas.\n".to_owned();
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.las, file2.las, file3.las\" -o=outfile.las\"", short_exe, name).replace("*", &sep);
    
        LasToAscii { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LasToAscii {
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
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;
        
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = time::now();

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let mut i = 1;
        let num_files = vec.len();
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(sep) {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                let input: LasFile = match LasFile::new(&input_file, "r") {
                    Ok(lf) => lf,
                    Err(_) => return Err(Error::new(ErrorKind::NotFound, format!("No such file or directory ({})", input_file))),
                };
                
                let output_file = if input_file.to_lowercase().ends_with(".las") {
                    input_file.replace(".las", ".txt")
                } else if input_file.to_lowercase().ends_with(".zip") {
                    input_file.replace(".zip", ".txt")
                } else {
                    return Err(Error::new(ErrorKind::NotFound, format!("No such file or directory ({})", input_file)));
                };

                let f = File::create(output_file)?;
                let mut writer = BufWriter::new(f);

                
                let n_points = input.header.number_of_points as usize;

                writer.write_all("X Y Z Intensity Class Return Num_returns\n".as_bytes())?;
                for k in 0..n_points {
                    let pd = input[k];
                    let s = format!("{} {} {} {} {} {} {}\n", pd.x, pd.y, pd.z, pd.intensity, pd.classification(), pd.return_number(), pd.number_of_returns());
                    writer.write_all(s.as_bytes())?;

                    if verbose {
                        progress = (100.0_f64 * k as f64 / (n_points - 1) as f64) as usize;
                        if progress != old_progress {
                            if num_files > 1 {
                                println!("Creating file: {} of {}: {}%", i, num_files, progress);
                            } else {
                                println!("Progress: {}%", progress);
                            }
                            old_progress = progress;
                        }
                    }
                }
                let _ = writer.flush();
            }
            i += 1;
        }

        let end = time::now();
        let elapsed_time = end - start;
        println!("{}", &format!("Elapsed Time: {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
