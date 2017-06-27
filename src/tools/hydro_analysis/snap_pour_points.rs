/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: June 27, 2017
License: MIT

NOTES: This tool should be updated to take vector pour points as inputs/outputs when vector support has been added.
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct SnapPourPoints {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl SnapPourPoints {
    pub fn new() -> SnapPourPoints { // public constructor
        let name = "SnapPourPoints".to_string();
        
        let description = "Moves outlet points used to specify points of interest in a watershedding operation onto the stream network.".to_string();
        
        let mut parameters = "--pour_pts    Input pour points (outlet) raster file.\n".to_owned();
        parameters.push_str("--flow_accum  Input D8 flow accumulation raster file.\n");
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--snap_dist   Maximum snap distance in map units.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --pour_pts='pour_pts.dep' --flow_accum='d8accum.dep' -o='output.dep' --snap_dist=15.0", short_exe, name).replace("*", &sep);
    
        SnapPourPoints { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for SnapPourPoints {
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
        let mut pourpts_file = String::new();
        let mut flow_accum_file = String::new();
        let mut output_file = String::new();
        let mut snap_dist = 0.0;
        
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
            if vec[0].to_lowercase() == "-pour_pts" || vec[0].to_lowercase() == "--pour_pts" {
                if keyval {
                    pourpts_file = vec[1].to_string();
                } else {
                    pourpts_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-flow_accum" || vec[0].to_lowercase() == "--flow_accum" {
                if keyval {
                    flow_accum_file = vec[1].to_string();
                } else {
                    flow_accum_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-snap_dist" || vec[0].to_lowercase() == "--snap_dist" {
                if keyval {
                    snap_dist = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    snap_dist = args[i+1].to_string().parse::<f64>().unwrap();
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

        if !pourpts_file.contains(&sep) {
            pourpts_file = format!("{}{}", working_directory, pourpts_file);
        }
        if !flow_accum_file.contains(&sep) {
            flow_accum_file = format!("{}{}", working_directory, flow_accum_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let pourpts = Raster::new(&pourpts_file, "r")?;
        let flow_accum = Raster::new(&flow_accum_file, "r")?;
        
        let start = time::now();

        let rows = pourpts.configs.rows as isize;
        let columns = pourpts.configs.columns as isize;
        let nodata = pourpts.configs.nodata;
        let fa_nodata = flow_accum.configs.nodata;
        
        // make sure the input files have the same size
        if pourpts.configs.rows != flow_accum.configs.rows || pourpts.configs.columns != flow_accum.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let snap_dist_int: isize = ((snap_dist / pourpts.configs.resolution_x) / 2.0).floor() as isize;

        let mut output = Raster::initialize_using_file(&output_file, &pourpts);
        
        let mut outlet_id: f64;
        let mut max_accum: f64;
        let mut zn: f64;
        let mut xn: isize;
        let mut yn: isize;
        for row in 0..rows {
            for col in 0..columns {
                outlet_id = pourpts[(row, col)];
                if outlet_id > 0.0 && outlet_id != nodata {
                    max_accum = 0.0;
                    xn = col;
                    yn = row;
                    for x in (col - snap_dist_int)..(col + snap_dist_int + 1) {
                        for y in (row - snap_dist_int)..(row + snap_dist_int + 1) {
                            zn = flow_accum[(y, x)];
                            if zn > max_accum && zn != fa_nodata {
                                max_accum = zn;
                                xn = x;
                                yn = y;
                            }
                        }
                    }
                    output[(yn, xn)] = outlet_id;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
        output.add_metadata_entry(format!("D8 flow accumulation file: {}", flow_accum_file));
        output.add_metadata_entry(format!("Snap distance: {}", snap_dist));
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