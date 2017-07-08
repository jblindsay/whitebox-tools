/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 7, 2017
Last Modified: July 7, 2017
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

pub struct DirectionalRelief {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DirectionalRelief {
    /// public constructor
    pub fn new() -> DirectionalRelief { 
        let name = "DirectionalRelief".to_string();
        
        let description = "Calculates relief for cells in an input DEM for a specified direction.".to_string();
        
        let mut parameters = "-i, --dem      Input DEM raster file.".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--azimuth      Wind azimuth in degrees (default is 0.0).\n");
        parameters.push_str("--max_dist     Optional maximum search distance (unspecified if none; in xy units).\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i='input.dep' -o=output.dep --azimuth=315.0", short_exe, name).replace("*", &sep);
    
        DirectionalRelief { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DirectionalRelief {
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
        let mut azimuth = 0.0;
        let mut max_dist = f64::INFINITY;
         
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
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
            } else if vec[0].to_lowercase() == "-azimuth" || vec[0].to_lowercase() == "--azimuth" {
                if keyval {
                    azimuth = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    azimuth = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-max_dist" || vec[0].to_lowercase() == "--max_dist" {
                if keyval {
                    max_dist = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_dist = args[i+1].to_string().parse::<f64>().unwrap();
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
                cell_size = cell_size * (113200.0 * mid_lat.cos());
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

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut current_val: f64;
                let mut y_intercept: f64;
                let mut flag: bool;
                let (mut delta_x, mut delta_y): (f64, f64);
                let (mut x, mut y): (f64, f64);
                let (mut x1, mut y1): (isize, isize);
                let (mut x2, mut y2): (isize, isize);
                let (mut z1, mut z2): (f64, f64);
                let mut dist: f64;
                let mut total_elevation: f64;
                let mut n_elevations: f64;
                let use_max_dist: bool;
                if max_dist == f64::INFINITY {
                    use_max_dist = false;
                } else {
                    use_max_dist = true;
                    max_dist = max_dist * max_dist;
                }
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        current_val = input[(row, col)];
                        if current_val != nodata {
                            total_elevation = 0f64;
                            n_elevations = 0f64;

                            //calculate the y intercept of the line equation
                            y_intercept = -row as f64 - line_slope * col as f64;

                            //find all of the vertical intersections
                            x = col as f64;
                            
                            flag = true;
                            while flag {
                                x = x + x_step as f64;
                                if x < 0.0 || x >= columns as f64 {
                                    flag = false;
                                } else {
                                    //calculate the Y value
                                    y = (line_slope * x + y_intercept) * -1f64;
                                    if y < 0f64 || y >= rows as f64 {
                                        flag = false;
                                    } else {
                                        //estimate z
                                        y1 = y as isize;
                                        y2 = y1 + y_step * -1isize;
                                        z1 = input[(y1, x as isize)];
                                        z2 = input[(y2, x as isize)];
                                        if z1 != nodata && z2 != nodata {
                                            z = z1 + (y - y1 as f64) * (z2 - z1);
                                            total_elevation += z;
                                            n_elevations += 1f64;
                                        }
                                        if use_max_dist {
                                            //calculate the distance
                                            delta_x = (x - col as f64) * cell_size;
                                            delta_y = (y - row as f64) * cell_size;

                                            dist = delta_x * delta_x + delta_y * delta_y;
                                            if dist >= max_dist {
                                                flag = false;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            //find all of the horizontal intersections
                            y = -row as f64;
                            flag = true;
                            while flag {
                                y = y + y_step as f64;
                                if -y < 0f64 || -y >= rows as f64 {
                                    flag = false;
                                } else {
                                    //calculate the X value
                                    x = (y - y_intercept) / line_slope;
                                    if x < 0f64 || x >= columns as f64 {
                                        flag = false;
                                    } else {
                                        //estimate z
                                        x1 = x as isize;
                                        x2 = x1 + x_step;
                                        if x2 < 0 || x2 >= columns {
                                            flag = false;
                                        } else {
                                            z1 = input[(-y as isize, x1)];
                                            z2 = input[(y as isize, x2)];
                                            if z1 != nodata && z2 != nodata {
                                                z = z1 + (x - x1 as f64) * (z2 - z1);
                                                total_elevation += z;
                                                n_elevations += 1f64;
                                            }
                                            if use_max_dist {
                                                //calculate the distance
                                                delta_x = (x - col as f64) * cell_size;
                                                delta_y = (-y - row as f64) * cell_size;
                                                dist = delta_x * delta_x + delta_y * delta_y;
                                                if dist >= max_dist {
                                                    flag = false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if n_elevations > 0f64 {
                                data[col as usize] = total_elevation / n_elevations - current_val;
                            } else {
                                data[col as usize] = nodata;
                            }
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

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Azimuth: {}", azimuth));
        output.add_metadata_entry(format!("Max dist: {}", max_dist));
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