/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 07/07/2017
Last Modified: 03/09/2020
License: MIT
*/

use whitebox_raster::Raster;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool creates a new raster in which each grid cell is assigned the distance, in meters, to the nearest
/// topographic obstacle in a specified direction. It is a modification of the algorithm described by Lapen and
/// Martz (1993). Unlike the original algorithm, Fetch Analysis is capable of analyzing fetch in any direction
/// from 0-360 degrees. The user must specify the name of an input digital elevation model (DEM) raster file, the
/// output raster name, a hypothetical wind direction, and a value for the height increment parameter. The algorithm
/// searches each grid cell in a path following the specified wind direction until the following condition is met:
///
///  > *Z*<sub>test</sub> >= *Z*<sub>core</sub> + *DI*
///
/// where *Z*<sub>core</sub> is the elevation of the grid cell at which fetch is being determined, *Z*<sub>test</sub>
/// is the elevation of the grid cell being tested as a topographic obstacle, *D* is the distance between the two
/// grid cells in meters, and *I* is the height increment in m/m. Lapen and Martz (1993) suggest values for *I* in
/// the range of 0.025 m/m to 0.1 m/m based on their study of snow re-distribution in low-relief agricultural
/// landscapes of the Canadian Prairies. If the directional search does not identify an obstacle grid cell before the
/// edge of the DEM is reached, the distance between the DEM edge and Zcore is entered. Edge distances are assigned
/// negative values to differentiate between these artificially truncated fetch values and those for which a valid
/// topographic obstacle was identified. Notice that linear interpolation is used to estimate the elevation of the
/// surface where a ray (i.e. the search path) does not intersect the DEM grid precisely at one of its nodes.
///
/// Ray-tracing is a highly computationally intensive task and therefore this tool may take considerable time to
/// operate for larger sized DEMs. This tool is parallelized to aid with computational efficiency. NoData valued
/// grid cells in the input image will be assigned NoData values in the output image. Fetch Analysis images are
/// best displayed using the blue-white-red bipolar palette to distinguish between the positive and negative
/// values that are present in the output.
///
/// # Reference
/// Lapen, D. R., & Martz, L. W. (1993). The measurement of two simple topographic indices of wind sheltering-exposure
/// from raster digital elevation models. Computers & Geosciences, 19(6), 769-779.
///
/// # See Also
/// `DirectionalRelief`, `HorizonAngle`, `RelativeAspect`
pub struct FetchAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FetchAnalysis {
    /// public constructor
    pub fn new() -> FetchAnalysis {
        let name = "FetchAnalysis".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Performs an analysis of fetch or upwind distance to an obstacle.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
            name: "Azimuth (degrees)".to_owned(),
            flags: vec!["--azimuth".to_owned()],
            description: "Wind azimuth in degrees in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Height Increment Value".to_owned(),
            flags: vec!["--hgt_inc".to_owned()],
            description: "Height increment value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.05".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --azimuth=315.0", short_exe, name).replace("*", &sep);

        FetchAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FetchAnalysis {
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
        let mut azimuth = 0.0;
        let mut height_increment = 0.05;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-azimuth" {
                if keyval {
                    azimuth = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    azimuth = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-hgt_inc" {
                if keyval {
                    height_increment = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    height_increment = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
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

        if azimuth > 360f64 || azimuth < 0f64 {
            azimuth = 0.1;
        }
        if azimuth == 0f64 {
            azimuth = 0.1;
        }
        if azimuth == 180f64 {
            azimuth = 179.9;
        }
        if azimuth == 360f64 {
            azimuth = 359.9;
        }
        let line_slope: f64;
        if azimuth < 180f64 {
            line_slope = (90f64 - azimuth).to_radians().tan();
        } else {
            line_slope = (270f64 - azimuth).to_radians().tan();
        }

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        if input.is_in_geographic_coordinates() {
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                cell_size = cell_size * (111320.0 * mid_lat.cos());
            }
        }

        let x_step: isize;
        let y_step: isize;
        if azimuth > 0f64 && azimuth <= 90f64 {
            x_step = 1;
            y_step = 1;
        } else if azimuth <= 180f64 {
            x_step = 1;
            y_step = -1;
        } else if azimuth <= 270f64 {
            x_step = -1;
            y_step = -1;
        } else {
            x_step = -1;
            y_step = 1;
        }

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
                let mut z: f64;
                let mut current_val: f64;
                let mut y_intercept: f64;
                let mut flag: bool;
                let mut max_val_dist: f64;
                let (mut delta_x, mut delta_y): (f64, f64);
                let (mut x, mut y): (f64, f64);
                let (mut x1, mut y1): (isize, isize);
                let (mut x2, mut y2): (isize, isize);
                let (mut z1, mut z2): (f64, f64);
                let mut dist: f64;
                let mut old_dist: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        current_val = input[(row, col)];
                        if current_val != nodata {
                            //calculate the y intercept of the line equation
                            y_intercept = -row as f64 - line_slope * col as f64;

                            //find all of the vertical intersections
                            max_val_dist = 0f64;
                            dist = 0f64;
                            x = col as f64;

                            flag = true;
                            while flag {
                                x = x + x_step as f64;
                                if x < 0.0 || x >= columns as f64 {
                                    flag = false;
                                // break;
                                } else {
                                    //calculate the Y value
                                    y = (line_slope * x + y_intercept) * -1f64;
                                    if y < 0f64 || y >= rows as f64 {
                                        flag = false;
                                    // break;
                                    } else {
                                        //calculate the distance
                                        delta_x = (x - col as f64) * cell_size;
                                        delta_y = (y - row as f64) * cell_size;

                                        dist = (delta_x * delta_x + delta_y * delta_y).sqrt();
                                        //estimate z
                                        y1 = y as isize;
                                        y2 = y1 + y_step * -1isize;
                                        z1 = input[(y1, x as isize)];
                                        z2 = input[(y2, x as isize)];
                                        z = z1 + (y - y1 as f64) * (z2 - z1);

                                        if z >= current_val + dist * height_increment {
                                            max_val_dist = dist;
                                            flag = false;
                                        }
                                    }
                                }
                            }

                            old_dist = dist;

                            //find all of the horizontal intersections
                            y = -row as f64;
                            flag = true;
                            while flag {
                                y = y + y_step as f64;
                                if -y < 0f64 || -y >= rows as f64 {
                                    flag = false;
                                // break;
                                } else {
                                    //calculate the X value
                                    x = (y - y_intercept) / line_slope;
                                    if x < 0f64 || x >= columns as f64 {
                                        flag = false;
                                    //break;
                                    } else {
                                        //calculate the distance
                                        delta_x = (x - col as f64) * cell_size;
                                        delta_y = (-y - row as f64) * cell_size;
                                        dist = (delta_x * delta_x + delta_y * delta_y).sqrt();
                                        //estimate z
                                        x1 = x as isize;
                                        x2 = x1 + x_step;
                                        if x2 < 0 || x2 >= columns {
                                            flag = false;
                                        // break;
                                        } else {
                                            z1 = input[(-y as isize, x1)];
                                            z2 = input[(y as isize, x2)];
                                            z = z1 + (x - x1 as f64) * (z2 - z1);

                                            if z >= current_val + dist * height_increment {
                                                if dist < max_val_dist || max_val_dist == 0f64 {
                                                    max_val_dist = dist;
                                                }
                                                flag = false;
                                            }
                                        }
                                    }
                                }
                            }

                            if max_val_dist == 0f64 {
                                //find the larger of dist and olddist
                                if dist > old_dist {
                                    max_val_dist = -dist;
                                } else {
                                    max_val_dist = -old_dist;
                                }
                            }
                            data[col as usize] = max_val_dist;
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
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Azimuth: {}", azimuth));
        output.add_metadata_entry(format!("Height increment: {}", height_increment));
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
