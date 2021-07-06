/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/02/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::isize;
use std::path;

/// Thins points from high density areas within a LiDAR point cloud.
pub struct LidarThinHighDensity {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarThinHighDensity {
    pub fn new() -> LidarThinHighDensity {
        // public constructor
        let name = "LidarThinHighDensity".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Thins points from high density areas within a LiDAR point cloud.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Max. Point Density (pts/m^2)".to_owned(),
            flags: vec!["--density".to_owned()],
            description: "Max. point density (points / m^3).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Save filtered points to seperate file?".to_owned(),
            flags: vec!["--save_filtered".to_owned()],
            description: "Save filtered points to seperate file?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --resolution=1.0 --density=100.0 --save_filtered", short_exe, name).replace("*", &sep);

        LidarThinHighDensity {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarThinHighDensity {
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
        let mut density: f64 = f64::MAX;
        let mut save_filtered = false;

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
            } else if flag_val == "-density" {
                density = if keyval {
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
            } else if flag_val == "-save_filtered" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    save_filtered = true;
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

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let start = Instant::now();

        if verbose {
            println!("Performing analysis...");
        }

        let west: f64 = input.header.min_x;
        let north: f64 = input.header.max_y;
        let rows = (((north - input.header.min_y) / grid_res).ceil()) as isize;
        let columns = (((input.header.max_x - west) / grid_res).ceil()) as isize;
        let south: f64 = north - rows as f64 * grid_res;
        let east = west + columns as f64 * grid_res;
        let half_grid_res = grid_res / 2.0;
        let ns_range = north - south;
        let ew_range = east - west;

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut map = HashMap::new();
        let (mut row, mut col): (isize, isize);
        for i in 0..n_points {
            // let p: PointData = input.get_point_info(i);
            let p = input.get_transformed_coords(i);
            col =
                (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round()) as isize;
            row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round()) as isize;
            let gc = GridCell {
                column: col,
                row: row,
            };
            let val = match map.entry(gc) {
                Vacant(entry) => entry.insert(vec![]),
                Occupied(entry) => entry.into_mut(),
            };
            val.push(SearchEntry { z: p.z, index: i });
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let threshold = grid_res * grid_res * density;
        let mut filtered = vec![false; n_points];
        let mut num_solved_points = 0;
        for row in 0..rows {
            for col in 0..columns {
                if let Some(vals) = map.get(&GridCell {
                    column: col,
                    row: row,
                }) {
                    if vals.len() as f64 > threshold {
                        let mut minz = f64::INFINITY;
                        let mut maxz = f64::NEG_INFINITY;
                        for val in vals {
                            if val.z < minz {
                                minz = val.z;
                            }
                            if val.z > maxz {
                                maxz = val.z;
                            }
                        }
                        let mut num_bins = ((maxz - minz) / grid_res).ceil() as usize;
                        if (maxz - minz) % grid_res == 0f64 {
                            num_bins += 1;
                        }
                        let mut histo = vec![0f64; num_bins];
                        let mut bin: usize;
                        for val in vals {
                            bin = ((val.z - minz) / grid_res).floor() as usize;
                            histo[bin] += 1f64;
                        }
                        let mut skip_factor = vec![1usize; num_bins];
                        for i in 0..num_bins {
                            if histo[i] > threshold {
                                skip_factor[i] = (histo[i] / threshold).floor() as usize;
                            }
                        }
                        let mut skipped = vec![0usize; num_bins];
                        for val in vals {
                            bin = ((val.z - minz) / grid_res).floor() as usize;
                            if histo[bin] > threshold {
                                skipped[bin] += 1usize;
                                if skipped[bin] <= skip_factor[bin] {
                                    filtered[val.index] = true;
                                } else {
                                    skipped[bin] = 0usize;
                                }
                            }
                        }
                    }

                    num_solved_points += vals.len();
                    if verbose {
                        progress = (100.0_f64 * num_solved_points as f64 / num_points) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        let end;

        if !save_filtered {
            for i in 0..n_points {
                if !filtered[i] {
                    output.add_point_record(input.get_record(i));
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            end = get_formatted_elapsed_time(start);
        } else {
            let p = path::Path::new(&output_file);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            let filtered_output_file = output_file.replace(&extension, "_filtered_points.las");
            let mut filtered_output = LasFile::initialize_using_file(&filtered_output_file, &input);
            filtered_output.header.system_id = "EXTRACTION".to_string();

            for i in 0..n_points {
                if !filtered[i] {
                    output.add_point_record(input.get_record(i));
                } else {
                    filtered_output.add_point_record(input.get_record(i));
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            end = get_formatted_elapsed_time(start);

            let _ = match filtered_output.write() {
                Ok(_) => println!("Filtered points LAS file saved"),
                Err(e) => println!("error while writing: {:?}", e),
            };
        }

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Complete!")
                }
            }
            Err(e) => println!("error while writing: {:?}", e),
        };
        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", end));
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct GridCell {
    column: isize,
    row: isize,
}

#[derive(Clone, Copy)]
struct SearchEntry {
    z: f64,
    index: usize,
}
