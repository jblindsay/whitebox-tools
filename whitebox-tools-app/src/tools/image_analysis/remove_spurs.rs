/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 05/07/2017
Last Modified: 16/02/2019
License: MIT

NOTE: This algorithm can't easily be parallelized because the output raster must be read
and written to during the same loop. Doing so would involve using a mutex.
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

/// This image processing tool removes small irregularities (i.e. spurs) on the boundaries of objects in a
/// Boolean input raster image (`--input`). This operation is sometimes called *pruning*. Remove Spurs is a useful tool
/// for cleaning an image before performing a line thinning operation. In fact, the input image need not be truly
/// Boolean (i.e. contain only 1's and 0's). All non-zero, positive values are considered to be foreground pixels
/// while all zero valued cells are considered background pixels.
///
/// Note: Unlike other filter-based operations in *WhiteboxTools*, this algorithm can't easily be parallelized because
/// the output raster must be read and written to during the same loop.
///
/// # See Also
/// `LineThinning`
pub struct RemoveSpurs {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RemoveSpurs {
    pub fn new() -> RemoveSpurs {
        // public constructor
        let name = "RemoveSpurs".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Removes the spurs (pruning operation) from a Boolean line image; intended to be used on the output of the LineThinning tool.".to_string();

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
            name: "Maximum Iterations".to_owned(),
            flags: vec!["--iterations".to_owned()],
            description: "Maximum number of iterations".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("10".to_owned()),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif --iterations=10",
            short_exe, name
        )
        .replace("*", &sep);

        RemoveSpurs {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RemoveSpurs {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut max_iterations = 10;

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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
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
            } else if flag_val == "-iterations" {
                max_iterations = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
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
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let start = Instant::now();

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if input[(row, col)] > 0.0 && input[(row, col)] != nodata {
                            data[col as usize] = 1.0;
                        } else if input[(row, col)] == 0.0 {
                            data[col as usize] = 0.0;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing output: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut did_something: bool;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let elements = vec![
            vec![0, 1, 4, 5, 6, 7],
            vec![0, 1, 2, 5, 6, 7],
            vec![0, 1, 2, 3, 6, 7],
            vec![0, 1, 2, 3, 4, 7],
            vec![0, 1, 2, 3, 4, 5],
            vec![1, 2, 3, 4, 5, 6],
            vec![2, 3, 4, 5, 6, 7],
            vec![0, 3, 4, 5, 6, 7],
        ];

        let mut neighbours = [0.0; 8];
        let mut pattern_match: bool;
        let mut z: f64;
        for loop_num in 0..max_iterations {
            did_something = false;
            if loop_num % 2 == 1 {
                for a in 0..8 {
                    for row in 0..rows {
                        for col in 0..columns {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }

                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as usize;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num + 1, progress);
                            old_progress = progress;
                        }
                    }
                }
            } else {
                for a in 0..8 {
                    for row in (0..rows).rev() {
                        for col in (0..columns).rev() {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }

                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != 0.0 {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as usize;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num + 1, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            if !did_something {
                break;
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Max iterations: {}", max_iterations));
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
