/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/07/2017
Last Modified: 13/10/2018
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool is a Boolean **XOR** operator, i.e. it works on *True* or *False* (1 and 0) values. Grid cells for which
/// either the first or second input rasters (`--input1`; `--input2`) have a *True* value but not both are assigned
/// 1 in the output raster, otherwise grid cells are assigned a value of 0. All non-zero values in the input
/// rasters are considered to be *True*, while all zero-valued grid cells are considered to be *False*. Grid
/// cells containing **NoData** values in either of the input rasters will be assigned a **NoData** value in
/// the output raster (`--output`). Notice that the **Not** operator is asymmetrical, and the order of inputs matters.
///
/// # See Also
/// `Or`, `And`, `Not`
pub struct Xor {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Xor {
    pub fn new() -> Xor {
        // public constructor
        let name = "Xor".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs a logical XOR operator on two Boolean raster images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["--input1".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["--input2".to_owned()],
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

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1='in1.tif' --input2='in2.tif' -o=output.tif", short_exe, name).replace("*", &sep);

        Xor {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Xor {
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
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
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
        let mut input1 = String::new();
        let mut input2 = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
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
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input1 = vec[1].to_string();
                } else {
                    input1 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input2 = vec[1].to_string();
                } else {
                    input2 = args[i + 1].to_string();
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
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !input1.contains(&sep) && !input1.contains("/") {
            input1 = format!("{}{}", working_directory, input1);
        }
        if !input2.contains(&sep) && !input2.contains("/") {
            input2 = format!("{}{}", working_directory, input2);
        }

        if verbose {
            println!("Reading data...")
        };
        let in1 = Arc::new(Raster::new(&input1, "r")?);
        let in2 = Arc::new(Raster::new(&input2, "r")?);

        let start = Instant::now();
        let rows = in1.configs.rows as isize;
        let columns = in1.configs.columns as isize;
        let nodata1 = in1.configs.nodata;
        let nodata2 = in2.configs.nodata;

        // make sure the input files have the same size
        if in1.configs.rows != in2.configs.rows || in1.configs.columns != in2.configs.columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        // calculate the number of downslope cells
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let in1 = in1.clone();
            let in2 = in2.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z1: f64;
                let mut z2: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata1; columns as usize];
                    for col in 0..columns {
                        z1 = in1[(row, col)];
                        z2 = in2[(row, col)];
                        if z1 != nodata1 && z2 != nodata2 {
                            if z1 != 0f64 {
                                z1 = 1f64;
                            }
                            if z2 != 0f64 {
                                z2 = 1f64;
                            }
                            if z1 + z2 == 1f64 {
                                //this occurs only when one of the two images has a true value
                                data[col as usize] = 1f64;
                            } else {
                                //this occurs when either neither of the images have a true image (i.e. z1+z2=0) or both do (i.e. z1+z2=2)
                                data[col as usize] = 0f64;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &in1);
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
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
        output.configs.data_type = DataType::F32;
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input1: {}", input1));
        output.add_metadata_entry(format!("Input2: {}", input2));
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
