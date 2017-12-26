/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/12/2017
Last Modified: 15/12/2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::*;

pub struct FindRidges {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FindRidges {
    pub fn new() -> FindRidges {
        // public constructor
        let name = "FindRidges".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Identifies potential ridge and peak grid cells."
            .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Perform line-thinning?".to_owned(), 
            flags: vec!["--line_thin".to_owned()], 
            description: "Optional flag indicating whether post-processing line-thinning should be performed.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true
        });

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=pointer.dep -o=out.dep --line_thin", short_exe, name).replace("*", &sep);

        FindRidges {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FindRidges {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut line_thin = false;
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-line_thin" || vec[0].to_lowercase() == "--line_thin" {
                line_thin = true;
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        
        output.reinitialize_values(0f64);
                
        // This one can be performed conccurently.
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = vec![0, 0, -1, 1];
                let dy = vec![-1, 1, 0, 0];
                let mut z: f64;
                let (mut zn1, mut zn2): (f64, f64);
                
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata  {
                            zn1 = input.get_value(row + dy[0], col + dx[0]);
                            zn2 = input.get_value(row + dy[1], col + dx[1]);
                            if zn1 != nodata && zn2 != nodata && zn1 < z && zn2 < z {
                                data[col as usize] = 1f64;
                            } else {
                                zn1 = input.get_value(row + dy[2], col + dx[2]);
                                zn2 = input.get_value(row + dy[3], col + dx[3]);
                                if zn1 != nodata && zn2 != nodata && zn1 < z && zn2 < z {
                                    data[col as usize] = 1f64;
                                }
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
        
        if line_thin {
            println!("Line thinning operation...");
            let mut did_something = true;
            let mut loop_num = 0;
            let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
            let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
            let elements = vec![ vec![ 6, 7, 0, 4, 3, 2 ], vec![ 7, 0, 1, 3, 5 ], 
                vec![ 0, 1, 2, 4, 5, 6 ], vec![ 1, 2, 3, 5, 7 ], 
                vec![ 2, 3, 4, 6, 7, 0 ], vec![ 3, 4, 5, 7, 1 ], 
                vec![ 4, 5, 6, 0, 1, 2 ], vec![ 5, 6, 7, 1, 3 ] ];

            let vals = vec![ vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ], 
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ],
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ],
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ] ];
            
            let mut neighbours = [0.0; 8];
            let mut pattern_match: bool;
            let mut z: f64;
            while did_something {
                loop_num += 1;
                did_something = false;
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
                                    if neighbours[elements[a][i]] != vals[a][i] {
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
                        progress = (100.0_f64 * a as f64 / 7.0) as i32;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("Line thinning: {}", line_thin));
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
