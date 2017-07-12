/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 12, 2017
Last Modified: July 12, 2017
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

pub struct PennockLandformClass {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl PennockLandformClass {
    pub fn new() -> PennockLandformClass { // public constructor
        let name = "PennockLandformClass".to_string();
        
        let description = "Classifies hillslope zones based on slope, profile curvature, and plan curvature.".to_string();
        
        let mut parameters = "-i, --dem      Input raster DEM file.".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--zfactor      Optional multiplier for when the vertical and horizontal units are not the same.");
        parameters.push_str("--slope        Slope threshold value, in degrees (default is 3.0).");
        parameters.push_str("--prof         Profile curvature threshold value (default is 0.1).");
        parameters.push_str("--plan         Plan curvature threshold value (default is 0.0).");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep --slope=3.0 --prof=0.1 --plan=0.0", short_exe, name).replace("*", &sep);
    
        PennockLandformClass { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for PennockLandformClass {
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
        let mut z_factor = 1f64;
        let mut slope_threshold = 3f64;
        let mut prof_threshold = 0.1_f64;
        let mut plan_threshold = 0f64;

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
            } else if vec[0].to_lowercase() == "-zfactor" || vec[0].to_lowercase() == "--zfactor" {
                if keyval {
                    z_factor = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    z_factor = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
                if keyval {
                    slope_threshold = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    slope_threshold = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-prof" || vec[0].to_lowercase() == "--prof" {
                if keyval {
                    prof_threshold = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    prof_threshold = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-plan" || vec[0].to_lowercase() == "--plan" {
                if keyval {
                    plan_threshold = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    plan_threshold = args[i+1].to_string().parse::<f64>().unwrap();
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

        let cell_size = input.configs.resolution_x;
        let cell_size_times2 = cell_size * 2.0f64;
        let cell_size_sqrd = cell_size * cell_size;
        let four_times_cell_size_sqrd = cell_size_sqrd * 4.0f64;
        let eight_grid_res = cell_size * 8.0;

        if input.is_in_geographic_coordinates() {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (113200.0 * mid_lat.cos());
            }
        }
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
                
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let (mut zx, mut zy, mut zxx, mut zyy, mut zxy, mut zx2, mut zy2): (f64, f64, f64, f64, f64, f64, f64);
                let mut p: f64;
                let mut q: f64;
                let (mut fx, mut fy): (f64, f64);
                let mut slope: f64;
                let mut plan: f64;
                let mut prof: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            z = z * z_factor;
                            for c in 0..8 {
                                n[c] = input[(row + dy[c], col + dx[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z;
                                }
                            }
                            // calculate curvature
                            zx = (n[1] - n[5]) / cell_size_times2;
                            zy = (n[7] - n[3]) / cell_size_times2;
                            zxx = (n[1] - 2.0f64 * z + n[5]) / cell_size_sqrd;
                            zyy = (n[7] - 2.0f64 * z + n[3]) / cell_size_sqrd;
                            zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
                            zx2 = zx * zx;
                            zy2 = zy * zy;
                            p = zx2 + zy2;
                            q = p + 1f64;
                            if p > 0.0f64 {
                                fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                                fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                                slope = (fx * fx + fy * fy).sqrt().atan().to_degrees();
                                plan = -1f64 * ((zxx * zy2 - 2f64 * zxy * zx * zy + zyy * zx2) / p.powf(1.5f64)).to_degrees();
                                prof = -1f64 * ((zxx * zy2 - 2f64 * zxy * zx * zy + zyy * zx2) / (p * q.powf(1.5f64))).to_degrees();
                                
                                if prof < -prof_threshold && plan <= -plan_threshold && slope > slope_threshold {
                                    //Convergent Footslope
                                    data[col as usize] = 1f64;
                                } else if prof < -prof_threshold && plan > plan_threshold && slope > slope_threshold {
                                    //Divergent Footslope
                                    data[col as usize] = 2f64;
                                } else if prof > prof_threshold && plan <= plan_threshold && slope > slope_threshold {
                                    //Convergent Shoulder
                                    data[col as usize] = 3f64;
                                } else if prof > prof_threshold && plan > plan_threshold && slope > slope_threshold {
                                    //Divergent Shoulder
                                    data[col as usize] = 4f64;
                                } else if prof >= -prof_threshold && prof < prof_threshold && slope > slope_threshold && plan <= -plan_threshold {
                                    //Convergent Backslope
                                    data[col as usize] = 5f64;
                                } else if prof >= -prof_threshold && prof < prof_threshold && slope > slope_threshold && plan > plan_threshold {
                                    //Divergent Backslope
                                    data[col as usize] = 6f64;
                                } else if slope <= slope_threshold {
                                    //Level
                                    data[col as usize] = 7f64;
                                } else {
                                    data[col as usize] = nodata;
                                }
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

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
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
        output.add_metadata_entry(format!("Slope threshold: {}", slope_threshold));
        output.add_metadata_entry(format!("Profile curvature threshold: {}", prof_threshold));
        output.add_metadata_entry(format!("Plan curvature threshold: {}", plan_threshold));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        output.add_metadata_entry(format!("CLASSIFICATION KEY"));
        output.add_metadata_entry(format!("Value  Class"));
        output.add_metadata_entry(format!("1      Convergent Footslope"));
        output.add_metadata_entry(format!("2      Divergent Footslope"));
        output.add_metadata_entry(format!("3      Convergent Shoulder"));
        output.add_metadata_entry(format!("4      Divergent Shoulder"));
        output.add_metadata_entry(format!("5      Convergent Backslope"));
        output.add_metadata_entry(format!("6      Divergent Backslope"));
        output.add_metadata_entry(format!("7      Level"));
        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("CLASSIFICATION KEY");
        println!("Value  Class");
        println!("1      Convergent Footslope");
        println!("2      Divergent Footslope");
        println!("3      Convergent Shoulder");
        println!("4      Divergent Shoulder");
        println!("5      Convergent Backslope");
        println!("6      Divergent Backslope");
        println!("7      Level");
        

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}