/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 2, 2017
Last Modified: July 2, 2017
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

pub struct RelativeStreamPowerIndex {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RelativeStreamPowerIndex {
    pub fn new() -> RelativeStreamPowerIndex { // public constructor
        let name = "RelativeStreamPowerIndex".to_string();
        
        let description = "Calculates the relative stream power index.".to_string();
        
        let mut parameters = "--sca          Input specific contributing area (SCA) raster file.".to_owned();
        parameters.push_str("--slope        Input slope raster file.\n");
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--exponent     SCA exponent value (default is 1.0).\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --sca='flow_accum.dep' --slope='slope.dep' -o=output.dep --exponent=1.1", short_exe, name).replace("*", &sep);
    
        RelativeStreamPowerIndex { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RelativeStreamPowerIndex {
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
        let mut sca_file = String::new();
        let mut slope_file = String::new();
        let mut output_file = String::new();
        let mut sca_exponent = 1.0;
         
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
            if vec[0].to_lowercase() == "-sca" || vec[0].to_lowercase() == "--sca" {
                if keyval {
                    sca_file = vec[1].to_string();
                } else {
                    sca_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
                if keyval {
                    slope_file = vec[1].to_string();
                } else {
                    slope_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-exponent" || vec[0].to_lowercase() == "--exponent" {
                if keyval {
                    sca_exponent = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sca_exponent = args[i+1].to_string().parse::<f64>().unwrap();
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

        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !sca_file.contains(&sep) {
            sca_file = format!("{}{}", working_directory, sca_file);
        }
        if !slope_file.contains(&sep) {
            slope_file = format!("{}{}", working_directory, slope_file);
        }


        if verbose { println!("Reading data...") };
        let sca = Arc::new(Raster::new(&sca_file, "r")?);
        let slope = Arc::new(Raster::new(&slope_file, "r")?);

        let start = time::now();
        let rows = sca.configs.rows as isize;
        let columns = sca.configs.columns as isize;
        let sca_nodata = sca.configs.nodata;
        let slope_nodata = slope.configs.nodata;

        // make sure the input files have the same size
        if sca.configs.rows != slope.configs.rows || sca.configs.columns != slope.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }
        
        // calculate the number of downslope cells
        let mut starting_row;
        let mut ending_row = 0;
        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_row < rows {
            let sca = sca.clone();
            let slope = slope.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx = tx.clone();
            thread::spawn(move || {
                let mut sca_val: f64;
                let mut slope_val: f64;
                for row in starting_row..ending_row {
                    let mut data: Vec<f64> = vec![sca_nodata; columns as usize];
                    for col in 0..columns {
                        sca_val = sca[(row, col)];
                        slope_val = slope[(row, col)];
                        if sca_val != sca_nodata && slope_val != slope_nodata {
                            data[col as usize] = sca_val.powf(sca_exponent) * slope_val.to_radians().tan();
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &sca);
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
        output.configs.data_type = DataType::F32;
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.clip_display_min_max(1.0);
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("SCA raster: {}", sca_file));
        output.add_metadata_entry(format!("Slope raster: {}", slope_file));
        output.add_metadata_entry(format!("SCA exponent: {}", sca_exponent));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        if sca.configs.maximum < 100.0 {
            println!("WARNING: The input SCA data layer contained only low values. It is likely that it has been
            log-transformed. This tool requires non-transformed SCA as an input.")
        }

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        
        Ok(())
    }
}