/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: June 26, 2017
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

pub struct NormalizedDifferenceVegetationIndex {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl NormalizedDifferenceVegetationIndex {
    pub fn new() -> NormalizedDifferenceVegetationIndex { // public constructor
        let name = "NormalizedDifferenceVegetationIndex".to_string();
        
        let description = "Calculates the normalized difference vegetation index (NDVI) from near-infrared and red imagery.".to_string();
        
        let mut parameters = "--nir         Input near-infrared band image.".to_owned();
        parameters.push_str("--red         Input red-band image.\n");
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--clip        Optional amount to clip the distribution tails by, in percent (default is 0.0).\n");
        parameters.push_str("--osavi       Optional flag indicating whether the optimized soil-adjusted veg index (OSAVI) should be used.");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --nir=band4.dep --red=band3.dep -o=output.dep
>>.*{0} -r={1} --wd=\"*path*to*data*\" --nir=band4.dep --red=band3.dep -o=output.dep --clip=1.0 --osavi", short_exe, name).replace("*", &sep);
    
        NormalizedDifferenceVegetationIndex { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for NormalizedDifferenceVegetationIndex {
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
        let mut nir_file = String::new();
        let mut red_file = String::new();
        let mut output_file = String::new();
        let mut clip_amount = 0.0;
        let mut osavi_mode = false;
        let mut correction_factor = 0.0;
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
            if vec[0].to_lowercase() == "-nir" || vec[0].to_lowercase() == "--nir" {
                if keyval {
                    nir_file = vec[1].to_string();
                } else {
                    nir_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-red" || vec[0].to_lowercase() == "--red" {
                if keyval {
                    red_file = vec[1].to_string();
                } else {
                    red_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-clip" || vec[0].to_lowercase() == "--clip" {
                if keyval {
                    clip_amount = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    clip_amount = args[i + 1].to_string().parse::<f64>().unwrap();
                }
                if clip_amount < 0.0 { clip_amount == 0.0; }
            } else if vec[0].to_lowercase() == "-osavi" || vec[0].to_lowercase() == "--osavi" {
                osavi_mode = true;
                correction_factor = 0.16;
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

        if !nir_file.contains(&sep) {
            nir_file = format!("{}{}", working_directory, nir_file);
        }
        if !red_file.contains(&sep) {
            red_file = format!("{}{}", working_directory, red_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let nir = Arc::new(Raster::new(&nir_file, "r")?);
        let rows = nir.configs.rows as isize;
        let columns = nir.configs.columns as isize;
        let nir_nodata = nir.configs.nodata;

        let red = Arc::new(Raster::new(&red_file, "r")?);
        let red_nodata = red.configs.nodata;

        // make sure the input files have the same size
        if nir.configs.rows != red.configs.rows || nir.configs.columns != red.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let start = time::now();

        let mut output = Raster::initialize_using_file(&output_file, &nir);

        let mut starting_row;
        let mut ending_row = 0;
        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_row < rows {
            let nir = nir.clone();
            let red = red.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut z_nir, mut z_red) : (f64, f64);
                for row in starting_row..ending_row {
                    let mut data = vec![nir_nodata; columns as usize];
                    for col in 0..columns {
                        z_nir = nir[(row, col)];
                        z_red = red[(row, col)];
                        if z_nir != nir_nodata && z_red != red_nodata {
                            if z_nir + z_red != 0.0 {
                                data[col as usize] = (z_nir - z_red) / (z_nir + z_red + correction_factor); 
                            } else {
                                data[col as usize] = nir_nodata;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if clip_amount > 0.0 {
            println!("Clipping output...");
            output.clip_min_and_max_by_percent(clip_amount);
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("NIR file: {}", nir_file));
        output.add_metadata_entry(format!("Red file: {}", red_file));
        output.add_metadata_entry(format!("Optimised Soil-Adjusted Vegetation Index (OSAVI) mode: {}", osavi_mode));
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