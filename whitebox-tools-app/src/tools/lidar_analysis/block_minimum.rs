/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/07/2017
Last Modified: 19/05/2020
License: MIT
*/

use whitebox_lidar::*;
use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

pub struct LidarBlockMinimum {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarBlockMinimum {
    pub fn new() -> LidarBlockMinimum {
        // public constructor
        let name = "LidarBlockMinimum".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates a block-minimum raster from an input LAS file. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        // parameters.push(ToolParameter{
        //     name: "Palette Name (Whitebox raster outputs only)".to_owned(),
        //     flags: vec!["--palette".to_owned()],
        //     description: "Optional palette name (for use with Whitebox raster files).".to_owned(),
        //     parameter_type: ParameterType::String,
        //     default_value: None,
        //     optional: true
        // });

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=5.0 --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        LidarBlockMinimum {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarBlockMinimum {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut palette = "default".to_string();

        // read the arguments
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
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-palette" {
                palette = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let start = Instant::now();

        let mut inputs = vec![];
        let mut outputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            // match fs::read_dir(working_directory) {
            //     Err(why) => println!("! {:?}", why.kind()),
            //     Ok(paths) => {
            //         for path in paths {
            //             let s = format!("{:?}", path.unwrap().path());
            //             if s.replace("\"", "").to_lowercase().ends_with(".las") {
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 outputs.push(
            //                     inputs[inputs.len() - 1]
            //                         .replace(".las", ".tif")
            //                         .replace(".LAS", ".tif"),
            //                 )
            //             }
            //         }
            //     }
            // }
            if std::path::Path::new(&working_directory).is_dir() {
                for entry in fs::read_dir(working_directory.clone())? {
                    let s = entry?
                        .path()
                        .into_os_string()
                        .to_str()
                        .expect("Error reading path string")
                        .to_string();
                    if s.to_lowercase().ends_with(".las") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".las", ".tif")
                                .replace(".LAS", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".laz") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".laz", ".tif")
                                .replace(".LAZ", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zlidar", ".tif")
                                .replace(".ZLIDAR", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zip", ".tif")
                                .replace(".ZIP", ".tif"),
                        )
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("The input directory ({}) is incorrect.", working_directory),
                ));
            }
        } else {
            inputs.push(input_file.clone());
            if output_file.is_empty() {
                output_file = input_file
                    .clone()
                    .replace(".las", ".tif")
                    .replace(".LAS", ".tif")
                    .replace(".laz", ".tif")
                    .replace(".LAZ", ".tif")
                    .replace(".zlidar", ".tif");
            }
            if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }
            outputs.push(output_file);
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

        for k in 0..inputs.len() {
            input_file = inputs[k].replace("\"", "").clone();
            output_file = outputs[k].replace("\"", "").clone();

            if verbose && inputs.len() > 1 {
                println!(
                    "Gridding {} of {} ({})",
                    k + 1,
                    inputs.len(),
                    input_file.clone()
                );
            }

            if !input_file.contains(path::MAIN_SEPARATOR) {
                input_file = format!("{}{}", working_directory, input_file);
            }
            if !output_file.contains(path::MAIN_SEPARATOR) {
                output_file = format!("{}{}", working_directory, output_file);
            }

            if verbose && inputs.len() == 1 {
                println!("reading input LiDAR file...");
            }
            let input = match LasFile::new(&input_file, "r") {
                Ok(lf) => lf,
                Err(err) => panic!("Error reading file {}: {}", input_file, err),
            };

            let start_run = Instant::now();

            if verbose && inputs.len() == 1 {
                println!("Performing analysis...");
            }

            let n_points = input.header.number_of_points as usize;
            let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

            let west: f64 = input.header.min_x; // - 0.5 * grid_res;
            let north: f64 = input.header.max_y; // + 0.5 * grid_res;
            let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
            let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;
            let south: f64 = north - rows as f64 * grid_res;
            let east = west + columns as f64 * grid_res;
            let nodata = -32768.0f64;
            let half_grid_res = grid_res / 2.0;
            let ns_range = north - south;
            let ew_range = east - west;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows;
            configs.columns = columns;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = grid_res;
            configs.resolution_y = grid_res;
            configs.nodata = nodata;
            configs.data_type = DataType::F64;
            configs.photometric_interp = PhotometricInterpretation::Continuous;
            configs.palette = palette.clone();

            let mut output = Raster::initialize_using_config(&output_file, &configs);

            let input = Arc::new(input); // wrap input in an Arc
            let num_procs = num_cpus::get();
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut col: isize;
                    let mut row: isize;
                    for i in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                        // let p: PointData = input.get_point_info(i);
                        let p = input.get_transformed_coords(i);
                        col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range)
                            .floor()) as isize;
                        row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range)
                            .floor()) as isize;
                        tx.send((row, col, p.z)).unwrap();
                    }
                });
            }

            let mut col: isize;
            let mut row: isize;
            let mut z: f64;
            let mut progress: i32;
            let mut old_progress: i32 = 1;
            for i in 0..n_points {
                let data = rx.recv().expect("Error receiving data from thread.");
                row = data.0;
                col = data.1;
                z = data.2;
                if output[(row, col)] == nodata || z < output[(row, col)] {
                    output.set_value(row, col, z);
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time_run = get_formatted_elapsed_time(start_run);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("Input file: {}", input_file));
            output.add_metadata_entry(format!(
                "Elapsed Time (excluding I/O): {}",
                elapsed_time_run
            ));

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
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
