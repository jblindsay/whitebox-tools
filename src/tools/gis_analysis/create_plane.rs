/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 11, 2017
Last Modified: July 11, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::WhiteboxTool;

pub struct CreatePlane {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl CreatePlane {
    pub fn new() -> CreatePlane {
        // public constructor
        let name = "CreatePlane".to_string();

        let description = "Creates a raster image based on the equation for a simple plane."
            .to_string();

        let mut parameters = "--base          Input base raster file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--gradient      Slope gradient in degrees (-85.0 to 85.0.\n");
        parameters.push_str("--aspect        Aspect (direction) in degrees clockwise from north (0.0-360.0).\n");
        parameters.push_str("--constant      Constant value (default is 0.0).\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --base=base.dep -o=NewRaster.dep --gradient=15.0 --aspect=315.0", short_exe, name).replace("*", &sep);

        CreatePlane {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CreatePlane {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut base_file = String::new();
        let mut output_file = String::new();
        let mut slope = 15.0;
        let mut aspect = 90.0;
        let mut constant_val = 0.0;

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
            if vec[0].to_lowercase() == "-base" || vec[0].to_lowercase() == "--base" {
                if keyval {
                    base_file = vec[1].to_string();
                } else {
                    base_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
                if keyval {
                    slope = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    slope = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-aspect" || vec[0].to_lowercase() == "--aspect" {
                if keyval {
                    aspect = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    aspect = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-constant" ||
                      vec[0].to_lowercase() == "--constant" {
                if keyval {
                    constant_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    constant_val = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !base_file.contains(&sep) {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let base = Raster::new(&base_file, "r")?;

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        if slope < -85.0 {
            slope = -85.0;
        }
        if slope > 85.0 {
            slope = 85.0;
        }
        if aspect > 360.0 {
            let mut flag = false;
            while !flag {
                aspect -= 360.0;
                if aspect <= 360.0 {
                    flag = true;
                }
            }
        }
        if aspect > 180.0 {
            aspect -= 180.0;
        } else {
            aspect += 180.0;
        }
        slope = slope.to_radians();
        aspect = aspect.to_radians();

        let rows = base.configs.rows as isize;
        let columns = base.configs.columns as isize;
        let north = base.configs.north;
        let south = base.configs.south;
        let east = base.configs.east;
        let west = base.configs.west;
        let xrange = east - west;
        let yrange = north - south;
        let nodata = base.configs.nodata;

        let mut output = Raster::initialize_using_file(&output_file, &base);
        output.configs.data_type = DataType::F32;
        output.configs.palette = "spectrum.plt".to_string();

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut x, mut y): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        x = west + xrange * (col as f64 / (columns as f64 - 1f64));
                        y = north - yrange * (row as f64 / (rows as f64 - 1f64));
                        data[col as usize] = slope.tan() * aspect.sin() * x +
                                             slope.tan() * aspect.cos() * y +
                                             constant_val;
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Base raster file: {}", base_file));
        output.add_metadata_entry(format!("Slope: {}", slope));
        output.add_metadata_entry(format!("Aspect: {}", aspect));
        output.add_metadata_entry(format!("Constant: {}", constant_val));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

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

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
