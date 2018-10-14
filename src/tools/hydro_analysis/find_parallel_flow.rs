/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 11, 2017
Last Modified: 12/10/2018
License: MIT
*/

use num_cpus;
use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use structures::Array2D;
use tools::*;

pub struct FindParallelFlow {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FindParallelFlow {
    pub fn new() -> FindParallelFlow {
        // public constructor
        let name = "FindParallelFlow".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Finds areas of parallel flow in D8 flow direction rasters.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input D8 pointer raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=pointer.tif -o=out.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=pointer.tif -o=out.tif --streams='streams.tif'",
            short_exe, name
        ).replace("*", &sep);

        FindParallelFlow {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FindParallelFlow {
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
        let mut d8_file = String::new();
        let mut use_streams = false;
        let mut streams_file = String::new();
        let mut output_file = String::new();

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
            if vec[0].to_lowercase() == "-d8_pntr" || vec[0].to_lowercase() == "--d8_pntr" {
                if keyval {
                    d8_file = vec[1].to_string();
                } else {
                    d8_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-streams" || vec[0].to_lowercase() == "--streams" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i + 1].to_string();
                }
                if !streams_file.is_empty() {
                    use_streams = true;
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

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let pntr = Arc::new(Raster::new(&d8_file, "r")?);

        let start = Instant::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = pntr.configs.nodata;

        let streams_nodata: f64;
        let streams: Array2D<f64> = match use_streams {
            false => {
                streams_nodata = -32768.0;
                Array2D::new(1, 1, 1f64, 1f64)?
            }
            true => {
                if verbose {
                    println!("Reading streams data...")
                };
                let r = Raster::new(&streams_file, "r")?;
                if r.configs.rows != rows as usize || r.configs.columns != columns as usize {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                        "The input files must have the same number of rows and columns and spatial extent."));
                }
                streams_nodata = r.configs.nodata;
                r.get_data_as_array2d()
            }
        };

        let mut output = Raster::initialize_using_file(&output_file, &pntr);
        let streams = Arc::new(streams);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let pntr = pntr.clone();
            let streams = streams.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut zn: f64;
                let mut stream_val: f64;
                let mut stream_valn: f64;
                let mut is_parallel: bool;
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
                let outflowing_vals = [1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64, 128f64];
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = pntr[(row, col)];
                        stream_val = streams[(row, col)];
                        if z != nodata && stream_val != streams_nodata && stream_val > 0f64 {
                            is_parallel = false;
                            for n in 0..8 {
                                if z != outflowing_vals[n] {
                                    zn = pntr[(row + dy[n], col + dx[n])];
                                    stream_valn = streams[(row + dy[n], col + dx[n])];
                                    if zn == z
                                        && zn != inflowing_vals[n]
                                        && stream_valn > 0f64
                                        && stream_valn != streams_nodata
                                    {
                                        is_parallel = true;
                                        break;
                                    }
                                }
                            }
                            if is_parallel {
                                data[col as usize] = 1f64;
                            } else {
                                data[col as usize] = 0f64;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
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
        output.add_metadata_entry(format!("Input D8 pointer file: {}", d8_file));
        if use_streams {
            output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        }
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
