/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 9, 2017
Last Modified: July 9, 2017
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
use structures::Array2D;
use tools::WhiteboxTool;

pub struct DownslopeDistanceToStream {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DownslopeDistanceToStream {
    pub fn new() -> DownslopeDistanceToStream { // public constructor
        let name = "DownslopeDistanceToStream".to_string();
        
        let description = "Measures distance to the nearest downslope stream cell.".to_string();
        
        let mut parameters = "--dem         Input DEM raster file.\n".to_owned();
        parameters.push_str("--streams     Input streams raster file.\n");
        parameters.push_str("-o, --output  Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --dem='dem.dep' --streams='streams.dep' -o='output.dep'", short_exe, name).replace("*", &sep);
    
        DownslopeDistanceToStream { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DownslopeDistanceToStream {
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
        let mut dem_file = String::new();
        let mut streams_file = String::new();
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
            if vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    dem_file = vec[1].to_string();
                } else {
                    dem_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-streams" || vec[0].to_lowercase() == "--streams" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i+1].to_string();
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

        if !dem_file.contains(&sep) {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !streams_file.contains(&sep) {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading DEM data...") };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);
        if verbose { println!("Reading streams data...") };
        let streams = Raster::new(&streams_file, "r")?;

        let start = time::now();

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        let cell_size_x = dem.configs.resolution_x;
        let cell_size_y = dem.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let flow_nodata = -2i8;
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let inflowing_vals = [ 4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8 ];
        
        
        // make sure the input files have the same size
        if dem.configs.rows != streams.configs.rows || dem.configs.columns != streams.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let dem = dem.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
                let (mut z, mut z_n): (f64, f64);
                let (mut max_slope, mut slope): (f64, f64);
                let mut dir: i8;
                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![flow_nodata; columns as usize];
                    for col in 0..columns {
                        z = dem[(row, col)];
                        if z != nodata {
                            dir = 0i8;
							max_slope = f64::MIN;
                            neighbouring_nodata = false;
							for i in 0..8 {
                                z_n = dem[(row + dy[i], col + dx[i])];
                                if z_n != nodata {
                                    slope = (z - z_n) / grid_lengths[i];
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i as i8;
                                    }
                                } else {
                                    neighbouring_nodata = true;
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1i8;
                                if !neighbouring_nodata {
                                    interior_pit_found = true;
                                }
                            }
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, flow_nodata, flow_nodata)?;
        let mut interior_pit_found = false;
        let mut output = Raster::initialize_using_file(&output_file, &dem);
        let background_value = f64::MIN;
        output.reinitialize_values(background_value);
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().unwrap();
            flow_dir.set_row_data(row, data);
            if pit { interior_pit_found = true; }
            for col in 0..columns {
                if streams[(row, col)] > 0f64 && streams[(row, col)] != streams_nodata {
                    output[(row, col)] = 0f64;
                    stack.push((row, col, dem[(row, col)]));
                }
                if dem[(row, col)] == nodata {
                    output[(row, col)] = nodata;
                    num_solved_cells += 1;
                }
                if flow_dir[(row, col)] == -1 {
                    if output[(row, col)] != 0f64 {
                        stack.push((row, col, nodata));
                        output[(row, col)] = nodata;
                        num_solved_cells += 1;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow directions: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let num_cells = dem.num_cells();
        let mut stream_dist: f64;
        let mut dist: f64;
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
        while !stack.is_empty() {
            let cell = stack.pop().unwrap();
            row = cell.0;
            col = cell.1;
            stream_dist = cell.2;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                if flow_dir[(row_n, col_n)] == inflowing_vals[n] && output[(row_n, col_n)] == background_value {
                    if stream_dist != nodata {
                        dist = stream_dist + grid_lengths[n];
                        output[(row_n, col_n)] = dist;
                        stack.push((row_n, col_n, dist));
                    } else {
                        output[(row_n, col_n)] = nodata;
                        stack.push((row_n, col_n, nodata));
                    }
                }
            }
            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("DEM file: {}", dem_file));
        output.add_metadata_entry(format!("Streams file: {}", streams_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if interior_pit_found {
            println!("**********************************************************************************");
            println!("WARNING: Interior pit cells were found within the input DEM. It is likely that the 
            DEM needs to be processed to remove topographic depressions and flats prior to
            running this tool.");
            println!("**********************************************************************************");
        }
        
        Ok(())
    }
}