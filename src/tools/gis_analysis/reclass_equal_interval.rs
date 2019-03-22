/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/07/2017
Last Modified: 13/10/2018
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool reclassifies the values in an input raster (`--input`) file based on an equal-interval scheme, where the 
/// user must specify the reclass interval value (`--interval`), the starting value (`--start_val`), and optionally, 
/// the ending value (`--end_val`). Grid cells containing values that fall outside of the range defined by the starting 
/// and ending values, will be assigned their original values in the output grid. If the user does not specify an ending 
/// value, the tool will assign a very large positive value.
/// 
/// # See Also
/// `Reclass`, `ReclassFromFile`
pub struct ReclassEqualInterval {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ReclassEqualInterval {
    /// public constructor
    pub fn new() -> ReclassEqualInterval {
        let name = "ReclassEqualInterval".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Reclassifies the values in a raster image based on equal-ranges.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            name: "Class Interval Size".to_owned(),
            flags: vec!["--interval".to_owned()],
            description: "Class interval size.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Starting Value".to_owned(),
            flags: vec!["--start_val".to_owned()],
            description: "Optional starting value (default is input minimum value).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Ending Value".to_owned(),
            flags: vec!["--end_val".to_owned()],
            description: "Optional ending value (default is input maximum value).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --interval=10.0 --start_val=0.0", short_exe, name).replace("*", &sep);

        ReclassEqualInterval {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ReclassEqualInterval {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut interval_size = 10.0;
        let mut start_val = f64::NEG_INFINITY;
        let mut end_val = f64::INFINITY;

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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
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
            } else if vec[0].to_lowercase() == "-interval" || vec[0].to_lowercase() == "--interval"
            {
                if keyval {
                    interval_size = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    interval_size = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-start_val"
                || vec[0].to_lowercase() == "--start_val"
            {
                if keyval {
                    start_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    start_val = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-end_val" || vec[0].to_lowercase() == "--end_val" {
                if keyval {
                    end_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    end_val = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        if start_val == f64::NEG_INFINITY {
            start_val = input.configs.minimum;
        }
        if end_val == f64::INFINITY {
            end_val = input.configs.maximum;
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            if z >= start_val && z <= end_val {
                                z = (z / interval_size).floor() * interval_size;
                            }
                            data[col as usize] = z;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
            output.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
