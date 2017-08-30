/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: August 10, 2017
Last Modified: August 28, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct RasterCellAssignment {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RasterCellAssignment {
    /// public constructor
    pub fn new() -> RasterCellAssignment {
        let name = "RasterCellAssignment".to_string();

        let description = "Assign row or column number to cells.".to_string();

        let mut parameters = "-i, --input      Input raster file.".to_owned();
        parameters.push_str("-o, --output      Output raster file.\n");
        parameters.push_str("-a, --assign      Which variable would you like to assign to grid cells? Options include 'column', 'row', 'x', and 'y'.\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i='input.dep' -o=output.dep --assign='column'", short_exe, name).replace("*", &sep);

        RasterCellAssignment { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RasterCellAssignment {
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
        let mut what_to_assign = String::from("column");

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
            } else if vec[0].to_lowercase() == "-a" || vec[0].to_lowercase() == "--assign" {
                if keyval {
                    what_to_assign = vec[1].to_string();
                } else {
                    what_to_assign = args[i+1].to_string();
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

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let num_procs = num_cpus::get() as isize;
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
            },
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
            },
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
            },
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
            },
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
