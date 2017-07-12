/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 22, 2017
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

pub struct Hillshade {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Hillshade {
    pub fn new() -> Hillshade { // public constructor
        let name = "Hillshade".to_string();
        
        let description = "Calculates a hillshade raster from an input DEM.".to_string();
        
        let mut parameters = "-i, --input   Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--azimuth     Illumination source azimuth.\n");
        parameters.push_str("--altitude    Illumination source altitude.\n");
        parameters.push_str("--zfactor     Optional multiplier for when the vertical and horizontal units are not the same.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep --azimuth=315.0 --altitude=30.0", short_exe, name).replace("*", &sep);
    
        Hillshade { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Hillshade {
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
        let mut azimuth = 315.0f64;
        let mut altitude = 30.0f64;
        let mut z_factor = 1f64;

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
            } else if vec[0].to_lowercase() == "-azimuth" || vec[0].to_lowercase() == "--azimuth" {
                if keyval {
                    azimuth = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    azimuth = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-altitude" || vec[0].to_lowercase() == "--altitude" {
                if keyval {
                    altitude = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    altitude = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-zfactor" || vec[0].to_lowercase() == "--zfactor" {
                if keyval {
                    z_factor = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    z_factor = args[i+1].to_string().parse::<f64>().unwrap();
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

        azimuth = (azimuth - 90f64).to_radians();
        altitude = altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        let eight_grid_res = input.configs.resolution_x * 8.0;

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

        let mut starting_row;
        let mut ending_row = 0;
        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_row < rows {
            let input = input.clone();
            let rows = rows.clone();
            // let z_factor = z_factor.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let (mut term1, mut term2, mut term3): (f64, f64, f64);
                let (mut fx, mut fy): (f64, f64);
                let mut tan_slope: f64;
                let mut aspect: f64;
                for row in starting_row..ending_row {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            z = z * z_factor;
                            for c in 0..8 {
                                n[c] = input[(row + d_y[c], col + d_x[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z;
                                }
                            }
                            // calculate slope and aspect
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            if fx != 0f64 {
                                tan_slope = (fx * fx + fy * fy).sqrt();
                                aspect = (180f64 - ((fy / fx).atan()).to_degrees() + 90f64 * (fx / (fx).abs())).to_radians();
                                term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                                term2 = sin_theta / tan_slope;
                                term3 = cos_theta * (azimuth - aspect).sin();
                                z = term1 * (term2 - term3);
                            } else {
                                z = 0.5;
                            }
                            z = z * 255.0;
                            if z < 0.0 {
                                z = 0.0;
                            }
                            data[col as usize] = z;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut histo: [f64; 256] = [0.0; 256];
        let nodata = input.configs.nodata;
        let mut num_cells = 0.0;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            let mut bin: usize;
            for col in 0..data.1.len() {
                if data.1[col] != nodata {
                    bin = data.1[col].round() as usize;
                    histo[bin] += 1.0;
                    num_cells += 1.0;
                }
            }
            output.set_row_data(data.0, data.1);
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut new_min = 0;
        let mut new_max = 0;
        let clip_percent = 0.01;
        let target_cell_num = num_cells * clip_percent;
        let mut sum = 0.0;
        for c in 0..256 {
            sum += histo[c];
            if sum >= target_cell_num {
                new_min = c;
                break;
            }
        }

        sum = 0.0;
        for c in (0..256).rev() {
            sum += histo[c];
            if sum >= target_cell_num {
                new_max = c;
                break;
            }
        }

        if new_max > new_min {
            output.configs.display_min = new_min as f64;
            output.configs.display_max = new_max as f64;
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Azimuth: {}", azimuth));
        output.add_metadata_entry(format!("Altitude: {}", altitude));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
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