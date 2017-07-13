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

pub struct ElevRelativeToWatershedMinMax {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ElevRelativeToWatershedMinMax {
    pub fn new() -> ElevRelativeToWatershedMinMax { // public constructor
        let name = "ElevRelativeToWatershedMinMax".to_string();
        
        let description = "Calculates the elevation of a location relative to the minimum and maximum elevations in a watershed.".to_string();
        
        let mut parameters = "-i, --dem      Input raster DEM file.\n".to_owned();
        parameters.push_str("--watersheds   Input watersheds raster file.\n");
        parameters.push_str("-o, --output   Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" --dem=DEM.dep --watersheds=watershed.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        ElevRelativeToWatershedMinMax { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for ElevRelativeToWatershedMinMax {
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
        let mut watersheds_file = String::new();
        let mut output_file = String::new();

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
            } else if vec[0].to_lowercase() == "-watersheds" || vec[0].to_lowercase() == "--watersheds" {
                if keyval {
                    watersheds_file = vec[1].to_string();
                } else {
                    watersheds_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
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
        if !watersheds_file.contains(&sep) {
            watersheds_file = format!("{}{}", working_directory, watersheds_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        // let min_val = input.configs.minimum;
        
        let watersheds = Arc::new(Raster::new(&watersheds_file, "r")?);
        let watershed_nodata = watersheds.configs.nodata;

        // make sure the input files have the same size
        if watersheds.configs.rows != input.configs.rows || watersheds.configs.columns != input.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let start = time::now();
        
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let min_watershed = watersheds.configs.minimum;
        let max_watershed = watersheds.configs.maximum;
        let range_watersheds = max_watershed - min_watershed;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let watersheds = watersheds.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut watershed_min_vals = vec![f64::INFINITY; range_watersheds as usize + 1];
                let mut watershed_max_vals = vec![f64::NEG_INFINITY; range_watersheds as usize + 1];
                let mut z: f64;
                let mut watershed: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        z = input[(row, col)];
                        watershed = watersheds[(row, col)];
                        if z != nodata && watershed != watershed_nodata {
                            watershed -= min_watershed;
                            if z < watershed_min_vals[watershed as usize] {
                                watershed_min_vals[watershed as usize] = z;
                            }
                            if z > watershed_max_vals[watershed as usize] {
                                watershed_max_vals[watershed as usize] = z;
                            }
                        }
                    }
                }
                tx.send((watershed_min_vals, watershed_max_vals)).unwrap();
            });
        }

        let mut watershed_min_vals = vec![f64::INFINITY; range_watersheds as usize + 1];
        let mut watershed_max_vals = vec![f64::NEG_INFINITY; range_watersheds as usize + 1];        
        for tid in 0..num_procs {
            let (mins, maxs) = rx.recv().unwrap();
            for i in 0..mins.len() { //(range_watersheds as usize+1) {
                if mins[i] != f64::INFINITY && mins[i] < watershed_min_vals[i] {
                    watershed_min_vals[i] = mins[i];
                }
                if maxs[i] != f64::NEG_INFINITY && maxs[i] > watershed_max_vals[i] {
                    watershed_max_vals[i] = maxs[i];
                }
            }
            if verbose {
                progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let watersheds = watersheds.clone();
            let watershed_min_vals = watershed_min_vals.clone();
            let watershed_max_vals = watershed_max_vals.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut watershed: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        watershed = watersheds[(row, col)];
                        if z != nodata && watershed != watershed_nodata {
                            watershed -= min_watershed;
                            data[col as usize] = (z - watershed_min_vals[watershed as usize]) / (watershed_max_vals[watershed as usize] - watershed_min_vals[watershed as usize]) * 100f64;
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

        println!("I'm here");

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Watersheds file: {}", watersheds_file));
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