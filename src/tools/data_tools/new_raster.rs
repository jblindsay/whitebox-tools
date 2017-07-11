/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 11, 2017
Last Modified: July 11, 2017
License: MIT
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct NewRasterFromBase {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl NewRasterFromBase {
    pub fn new() -> NewRasterFromBase {
        // public constructor
        let name = "NewRasterFromBase".to_string();

        let description = "Creates a new raster using a base image.".to_string();

        let mut parameters = "--base          Input base raster file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--value         Constant value to fill raster with; either 'nodata' or numberic value (default is nodata).\n");
        parameters.push_str("--data_type     Output raster data type; options include 'double' (64-bit), 'float' (32-bit), and 'integer' (signed 16-bit) (default is 'float').\n");

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
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --base=base.dep -o=NewRaster.dep --value=0.0 --data_type=integer
>>.*{0} -r={1} --wd=\"*path*to*data*\" --base=base.dep -o=NewRaster.dep --value=nodata", short_exe, name).replace("*", &sep);

        NewRasterFromBase {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NewRasterFromBase {
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
        let mut base_file = String::new();
        let mut output_file = String::new();
        let mut out_val_str = String::new();
        let mut data_type = String::new();

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
            if vec[0].to_lowercase() == "-base" || vec[0].to_lowercase() == "--base" {
                if keyval {
                    base_file = vec[1].to_string();
                } else {
                    base_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-value" || vec[0].to_lowercase() == "--value" {
                if keyval {
                    out_val_str = vec[1].to_string();
                } else {
                    out_val_str = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-data_type" ||
                      vec[0].to_lowercase() == "--data_type" {
                if keyval {
                    data_type = vec[1].to_string();
                } else {
                    data_type = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !base_file.contains(&sep) {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let base = Raster::new(&base_file, "r")?;

        let start = time::now();

        let nodata = -32768.0;
        let mut out_val = nodata;
        if out_val_str.to_lowercase() != "nodata" {
            // try to parse the value
            out_val = out_val_str.parse::<f64>().unwrap();
        }

        let mut output = Raster::initialize_using_file(&output_file, &base);
        if base.configs.nodata != nodata || out_val != nodata {
            output.configs.nodata = nodata;
            output.reinitialize_values(out_val);
        }

        if data_type.to_lowercase().contains("i") {
            output.configs.data_type = DataType::I16;
        } else if data_type.to_lowercase().contains("d") {
            output.configs.data_type = DataType::F64;
        } else {
            output.configs.data_type = DataType::F32;
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Base raster file: {}", base_file));
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
