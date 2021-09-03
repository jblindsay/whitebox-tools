/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: August 10, 2017
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

/// This tool can be used to create a new raster with the same coordinates and dimensions
/// (i.e. rows and columns) as an existing base image. Grid cells in the new raster will be
/// assigned either the row or column number or the x- or y-coordinate, depending on the
/// selected option (`--assign` flag). The user must also specify the name of the base
/// image (`--input`).
///
/// # See Also
/// `NewRasterFromBase`
pub struct RasterCellAssignment {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterCellAssignment {
    /// public constructor
    pub fn new() -> RasterCellAssignment {
        let name = "RasterCellAssignment".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Assign row or column number to cells.".to_string();

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

        parameters.push(ToolParameter{
            name: "Which spatial variable should be assigned?".to_owned(), 
            flags: vec!["-a".to_owned(), "--assign".to_owned()], 
            description: "Which variable would you like to assign to grid cells? Options include 'column', 'row', 'x', and 'y'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["column".to_owned(),"row".to_owned(), "x".to_owned(), "y".to_owned()]),
            default_value: Some("column".to_owned()),
            optional: false
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --assign='column'", short_exe, name).replace("*", &sep);

        RasterCellAssignment {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterCellAssignment {
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
        let mut what_to_assign = String::from("column");

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
            } else if vec[0].to_lowercase() == "-a" || vec[0].to_lowercase() == "--assign" {
                if keyval {
                    what_to_assign = vec[1].to_string();
                } else {
                    what_to_assign = args[i + 1].to_string();
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

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();

        match what_to_assign.to_lowercase().as_ref() {
            "column" | "columns" | "col" => {
                for tid in 0..num_procs {
                    let tx = tx.clone();
                    thread::spawn(move || {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data: Vec<f64> = vec![nodata; columns as usize];
                            for col in 0..columns {
                                data[col as usize] = col as f64;
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }
            }
            "row" | "rows" => {
                for tid in 0..num_procs {
                    let tx = tx.clone();
                    thread::spawn(move || {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data: Vec<f64> = vec![nodata; columns as usize];
                            for col in 0..columns {
                                data[col as usize] = row as f64;
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }
            }
            "x" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data: Vec<f64> = vec![nodata; columns as usize];
                            for col in 0..columns {
                                data[col as usize] = input.get_x_from_column(col);
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }
            }
            "y" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data: Vec<f64> = vec![nodata; columns as usize];
                            for col in 0..columns {
                                data[col as usize] = input.get_y_from_row(row);
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }
            }
            _ => {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "Unrecognized 'assign' input parameter. Options include 'column', 'row', 'x', and 'y'."));
            }
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
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
