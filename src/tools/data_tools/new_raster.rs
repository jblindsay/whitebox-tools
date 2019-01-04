/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 11, 2017
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

pub struct NewRasterFromBase {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NewRasterFromBase {
    pub fn new() -> NewRasterFromBase {
        // public constructor
        let name = "NewRasterFromBase".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Creates a new raster using a base image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["-i".to_owned(), "--base".to_owned()],
            description: "Input base raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Constant Value".to_owned(),
            flags: vec!["--value".to_owned()],
            description: "Constant value to fill raster with; either 'nodata' or numeric value."
                .to_owned(),
            parameter_type: ParameterType::StringOrNumber,
            default_value: Some("nodata".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Data Type".to_owned(), 
            flags: vec!["--data_type".to_owned()], 
            description: "Output raster data type; options include 'double' (64-bit), 'float' (32-bit), and 'integer' (signed 16-bit) (default is 'float').".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["double".to_owned(), "float".to_owned(), "integer".to_owned()]),
            default_value: Some("float".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=base.tif -o=NewRaster.tif --value=0.0 --data_type=integer
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=base.tif -o=NewRaster.tif --value=nodata", short_exe, name).replace("*", &sep);

        NewRasterFromBase {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NewRasterFromBase {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut base_file = String::new();
        let mut output_file = String::new();
        let mut out_val_str = String::new();
        let mut data_type = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
            ));
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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-base" || flag_val == "-i" || flag_val == "-input" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-value" {
                out_val_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-data_type" || flag_val == "-datatype" {
                data_type = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !base_file.contains(&sep) && !base_file.contains("/") {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let base = Raster::new(&base_file, "r")?;

        let start = Instant::now();

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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Base raster file: {}", base_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
