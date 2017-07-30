/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 6, 2017
Last Modified: July 6, 2017
License: MIT

NOTE: At the moment this tool determines input/output raster formats based on extensions, but due to file 
extension naming collisions, it would be good to add user hints. For example, the extension 'grd' could
belong to a SurferAscii or a Surfer7BinaryCollisions. This is more important for distinguishing output 
files since input files can be read and distiguishing feasture idenfitied from the file structure.
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::io::{Error, ErrorKind};
use raster::*;
use tools::WhiteboxTool;

pub struct ConvertRasterFormat {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ConvertRasterFormat {
    pub fn new() -> ConvertRasterFormat {
        // public constructor
        let name = "ConvertRasterFormat".to_string();

        let description = "Converts raster data from one format to another.".to_string();

        let mut parameters = "-i, --input   Input raster file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --input=DEM.dep -o=output.dep",
                            short_exe,
                            name)
                .replace("*", &sep);

        ConvertRasterFormat {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ConvertRasterFormat {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" ||
               vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Raster::new(&input_file, "r")?;

        // println!("config info {:?}", input.configs);

        let start = time::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        println!("Initializing the output raster...");
        match output.set_data_from_raster(&input) {
            Ok(_) => (), // do nothings
            Err(err) => return Err(err),
        }
        // for row in 0..input.configs.rows as isize {
        //     for col in 0..input.configs.columns as isize {
        //         output[(row, col)] = input[(row, col)];
        //         if row % 1000 == 0 && col % 1000 == 0 {
        //             println!("cell({}, {}) = {}", row, col, input[(row, col)]);
        //         }
        //     }
        // }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}