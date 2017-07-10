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

pub struct MaxBranchLength {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl MaxBranchLength {
    pub fn new() -> MaxBranchLength { // public constructor
        let name = "MaxBranchLength".to_string();
        
        let description = "Branch length is used to map drainage divides or ridge lines.".to_string();
        
        let mut parameters = "--dem          Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output   Output raster file.\n");
        parameters.push_str("--log          Optional flag to request the output be log-transformed.\n");
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        MaxBranchLength { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for MaxBranchLength {
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
        let mut log_transform = false;
        
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
            } else if vec[0].to_lowercase() == "-log" || vec[0].to_lowercase() == "--log" {
                log_transform = true;
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

        // calculate the flow direction
        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        
        let flow_nodata = -2i8;
        
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
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
                        z = input[(row, col)];
                        if z != nodata {
                            dir = 0i8;
							max_slope = f64::MIN;
                            neighbouring_nodata = false;
							for i in 0..8 {
                                z_n = input[(row + dy[i], col + dx[i])];
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
        for r in 0..rows {
            let (row, data, pit) = rx.recv().unwrap();
            flow_dir.set_row_data(row, data);
            if pit { interior_pit_found = true; }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow directions: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(0f64);
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
        let mut dir: i8;
        let (mut dist1, mut dist2): (f64, f64);
        let mut flag1: bool;
        let mut flag2: bool;
        let (mut r1, mut c1): (isize, isize);
        let (mut r2, mut c2): (isize, isize);
        let mut idx: isize;
        let mut paths: Array2D<isize> = Array2D::new(rows, columns, 0, 0)?;
        let mut path_lengths: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        for row in 0..rows {
            for col in 0..columns {
                if flow_dir[(row, col)] >= 0i8 {
                    idx = row * rows as isize + col + 1;

                    // right cell
                    r2 = row;
                    c2 = col + 1;
                    if flow_dir[(r2, c2)] >= 0i8 {
                        r1 = row;
                        c1 = col;
                        dist1 = 0f64;
                        dist2 = 0f64;
                        flag1 = true;
                        flag2 = true;
                        while flag1 || flag2 {
                            if flag1 {
                                if paths[(r1, c1)] == idx { // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist2 = path_lengths[(r1, c1)];
                                }
                                paths[(r1, c1)] = idx;
                                path_lengths[(r1, c1)] = dist1;
                                dir = flow_dir[(r1, c1)];
                                if dir >= 0 {
                                    r1 += dy[dir as usize];
                                    c1 += dx[dir as usize];
                                    dist1 += grid_lengths[dir as usize];
                                } else {
                                    flag1 = false;
                                }
                            }

                            if flag2 {
                                if paths[(r2, c2)] == idx { // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist1 = path_lengths[(r2, c2)];
                                }
                                paths[(r2, c2)] = idx;
                                path_lengths[(r2, c2)] = dist2;
                                dir = flow_dir[(r2, c2)];
                                if dir >= 0 {
                                    r2 += dy[dir as usize];
                                    c2 += dx[dir as usize];
                                    dist2 += grid_lengths[dir as usize];
                                } else {
                                    flag2 = false;
                                }
                            }
                        }
                        if dist1 > output[(row, col)] { output.set_value(row, col, dist1); }
                        if dist2 > output[(row, col + 1)] { output.set_value(row, col + 1, dist2); }
                    }

                    // lower cell
                    r2 = row + 1;
                    c2 = col;
                    if flow_dir[(r2, c2)] >= 0i8 {
                        idx = -idx;
                        r1 = row;
                        c1 = col;
                        dist1 = 0f64;
                        dist2 = 0f64;
                        flag1 = true;
                        flag2 = true;
                        while flag1 || flag2 {
                            if flag1 {
                                if paths[(r1, c1)] == idx { // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist2 = path_lengths[(r1, c1)];
                                }
                                paths[(r1, c1)] = idx;
                                path_lengths[(r1, c1)] = dist1;
                                dir = flow_dir[(r1, c1)];
                                if dir >= 0 {
                                    r1 += dy[dir as usize];
                                    c1 += dx[dir as usize];
                                    dist1 += grid_lengths[dir as usize];
                                } else {
                                    flag1 = false;
                                }
                            }

                            if flag2 {
                                if paths[(r2, c2)] == idx { // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist1 = path_lengths[(r2, c2)];
                                }
                                paths[(r2, c2)] = idx;
                                path_lengths[(r2, c2)] = dist2;
                                dir = flow_dir[(r2, c2)];
                                if dir >= 0 {
                                    r2 += dy[dir as usize];
                                    c2 += dx[dir as usize];
                                    dist2 += grid_lengths[dir as usize];
                                } else {
                                    flag2 = false;
                                }
                            }
                        }
                        if dist1 > output[(row, col)] { output.set_value(row, col, dist1); }
                        if dist2 > output[(row + 1, col)] { output.set_value(row + 1, col, dist2); }
                    }

                } else if input[(row, col)] == nodata {
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

        if log_transform {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] != nodata {
                        if output[(row, col)] > 0f64 {
                            output[(row, col)] = output[(row, col)].ln();
                        } else {
                            output[(row, col)] = nodata;
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Log transformation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        output.configs.palette = "grey.plt".to_string();
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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