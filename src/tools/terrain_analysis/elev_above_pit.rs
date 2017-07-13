/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 13, 2017
Last Modified: July 13, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use structures::Array2D;
use tools::WhiteboxTool;

pub struct ElevAbovePit {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ElevAbovePit {
    pub fn new() -> ElevAbovePit { // public constructor
        let name = "ElevAbovePit".to_string();
        
        let description = "Calculate the elevation of each grid cell above the nearest downstream pit cell or grid edge cell.".to_string();
        
        let mut parameters = "--dem         Input DEM raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        ElevAbovePit { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for ElevAbovePit {
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
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading DEM data...") };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let num_cells = dem.num_cells();
        let cell_size_x = dem.configs.resolution_x;
        let cell_size_y = dem.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        
        let start = time::now();
        
        let flow_nodata = -2i8;
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
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![flow_nodata; columns as usize];
                    for col in 0..columns {
                        z = dem[(row, col)];
                        if z != nodata {
                            dir = 0i8;
							max_slope = f64::MIN;
							for i in 0..8 {
                                z_n = dem[(row + dy[i], col + dx[i])];
                                if z_n != nodata {
                                    slope = (z - z_n) / grid_lengths[i];
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i as i8;
                                    }
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1i8;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, flow_nodata, flow_nodata)?;
        let mut output = Raster::initialize_using_file(&output_file, &dem);
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
            flow_dir.set_row_data(row, data);
            for col in 0..columns {
                if flow_dir[(row, col)] == -1 {
                    stack.push((row, col, dem[(row, col)]));
                    output[(row, col)] = 0f64;
                    num_solved_cells += 1;
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

        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let mut basin_z: f64;
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let inflowing_vals = [ 4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8 ];
        while !stack.is_empty() {
            let cell = stack.pop().unwrap();
            row = cell.0;
            col = cell.1;
            basin_z = cell.2;

            // add inflowing neighbours to the stack and assign their elevation difference values
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                if flow_dir[(row_n, col_n)] == inflowing_vals[n] {
                    stack.push((row_n, col_n, basin_z));
                    output[(row_n, col_n)] = dem[(row_n, col_n)] - basin_z;
                }
            }
            
            num_solved_cells += 1;

            if verbose {
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
        output.add_metadata_entry(format!("Input DEM file: {}", dem_file));
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