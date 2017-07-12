/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 1, 2017
Last Modified: July 1, 2017
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

pub struct RasterSummaryStats {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RasterSummaryStats {
    pub fn new() -> RasterSummaryStats { // public constructor
        let name = "RasterSummaryStats".to_string();
        
        let description = "Measures a rasters average, standard deviation, num. non-nodata cells, and total.".to_string();
        
        let parameters = "-i, --input     Input raster file.".to_owned();
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=DEM.dep", short_exe, name).replace("*", &sep);
    
        RasterSummaryStats { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RasterSummaryStats {
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

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        //if verbose { println!("Calculating image mean and standard deviation...") };
        //let (mean, stdev) = input.calculate_mean_and_stdev();
        
        // calculate the number of downslope cells
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut n = 0;
                    let mut s = 0.0;
                    let mut sq = 0.0;
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            n += 1;
                            s += z;
                            sq += z * z;
                        }
                    }
                    tx.send((n, s, sq)).unwrap();
                }
            });
        }

        let mut num_cells = 0;
        let mut sum = 0.0;
        let mut sq_sum = 0.0;
        for row in 0..rows {
            let (a, b, c) = rx.recv().unwrap();
            num_cells += a;
            sum += b;
            sq_sum += c;

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mean = sum / num_cells as f64;
        let variance = sq_sum / num_cells as f64 - mean * mean;
        let std_dev = variance.sqrt();

        let end = time::now();
        let elapsed_time = end - start;

        println!("\nNumber of non-nodata grid cells: {}", num_cells);
        println!("Number of nodata grid cells: {}", input.num_cells() - num_cells);
        println!("Image total: {}", sum);
        println!("Image average: {}", mean);
        println!("Image variance: {}", variance);
        println!("Image standard deviation: {}", std_dev);

        println!("\n{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}