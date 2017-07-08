/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 4, 2017
Last Modified: July 4, 2017
License: MIT

NOTES: Add support for vector seed points.
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct TraceDownslopeFlowpaths {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl TraceDownslopeFlowpaths {
    pub fn new() -> TraceDownslopeFlowpaths { // public constructor
        let name = "TraceDownslopeFlowpaths".to_string();
        
        let description = "Traces downslope flowpaths from one or more target sites (i.e. seed points).".to_string();
        
        let mut parameters = "--seed_pts         Input seed points raster file.\n".to_owned();
        parameters.push_str("--flow_dir         Input D8 flow direction (pointer) raster file.\n");
        parameters.push_str("-o, --output       Output cost pathway raster file.\n");
        parameters.push_str("--esri_pntr        Optional flag indicating whether the D8 pointer uses the ESRI style scheme (default is false).\n");
        parameters.push_str("--zero_background  Optional flag indicating whether the background value of zero should be used.\n");
       
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --seed_pts=seeds.dep --flow_dir=flow_directions.dep --output=flow_paths.dep", short_exe, name).replace("*", &sep);
    
        TraceDownslopeFlowpaths { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for TraceDownslopeFlowpaths {
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
        let mut seed_file = String::new();
        let mut flowdir_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut background_val = f64::NEG_INFINITY;
        
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
            if vec[0].to_lowercase() == "-seed_pts" || vec[0].to_lowercase() == "--seed_pts" {
                if keyval {
                    seed_file = vec[1].to_string();
                } else {
                    seed_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-flow_dir" || vec[0].to_lowercase() == "--flow_dir" {
                if keyval {
                    flowdir_file = vec[1].to_string();
                } else {
                    flowdir_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-esri_pntr" || vec[0].to_lowercase() == "--esri_pntr" || vec[0].to_lowercase() == "--esri_style" {
                esri_style = true;
            } else if vec[0].to_lowercase() == "-zero_background" || vec[0].to_lowercase() == "--zero_background" || vec[0].to_lowercase() == "--esri_style" {
                background_val = 0f64;
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

        if !seed_file.contains(&sep) {
            seed_file = format!("{}{}", working_directory, seed_file);
        }
        if !flowdir_file.contains(&sep) {
            flowdir_file = format!("{}{}", working_directory, flowdir_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        
        if verbose { println!("Reading destination data...") };
        let seeds = Raster::new(&seed_file, "r")?;

        if verbose { println!("Reading backlink data...") };
        let flowdir = Raster::new(&flowdir_file, "r")?;

        // make sure the input files have the same size
        if seeds.configs.rows != flowdir.configs.rows || seeds.configs.columns != flowdir.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let start = time::now();
        let rows = seeds.configs.rows as isize;
        let columns = seeds.configs.columns as isize;
        let nodata = flowdir.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }
        
        let mut output = Raster::initialize_using_file(&output_file, &seeds);
        output.reinitialize_values(background_val);
        
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let mut pntr_matches: [usize; 129] = [0usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 0usize;
            pntr_matches[2] = 1usize;
            pntr_matches[4] = 2usize;
            pntr_matches[8] = 3usize;
            pntr_matches[16] = 4usize;
            pntr_matches[32] = 5usize;
            pntr_matches[64] = 6usize;
            pntr_matches[128] = 7usize;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 1usize;
            pntr_matches[2] = 2usize;
            pntr_matches[4] = 3usize;
            pntr_matches[8] = 4usize;
            pntr_matches[16] = 5usize;
            pntr_matches[32] = 6usize;
            pntr_matches[64] = 7usize;
            pntr_matches[128] = 0usize;
        }
        let (mut x, mut y): (isize, isize);
        let mut flag: bool;
        let mut dir: f64;
        for row in 0..rows {
            for col in 0..columns {
                if seeds[(row, col)] > 0.0 && flowdir[(row, col)] != nodata {
                    flag = false;
                    x = col;
                    y = row;
                    while !flag {
                        if output[(y, x)] == background_val {
                            output[(y, x)] = 1.0;
                        } else {
                            output.increment(y, x, 1.0);
                        }
                        // find its downslope neighbour
                        dir = flowdir[(y, x)];
                        if dir != nodata && dir > 0.0 {
                            // move x and y accordingly
                            x += dx[pntr_matches[dir as usize]];
                            y += dy[pntr_matches[dir as usize]];
                        } else {
                            flag = true;
                        }
                    }
                } else if flowdir[(row, col)] == nodata {
                    output[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Seed points raster file: {}", seed_file));
        output.add_metadata_entry(format!("D8 flow direction (pointer) raster: {}", flowdir_file));
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
