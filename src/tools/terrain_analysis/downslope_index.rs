/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 17, 2017
Last Modified: July 17, 2017
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

pub struct DownslopeIndex {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DownslopeIndex {
    pub fn new() -> DownslopeIndex { // public constructor
        let name = "DownslopeIndex".to_string();
        
        let description = "Calculates the Hjerdt et al. (2004) downslope index.".to_string();
        
        let mut parameters = "-i, --dem       Input DEM raster file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--drop          Vertical drop value (default is 2.0).\n");
        parameters.push_str("--out_type      Output type, options include 'tangent', 'degrees', 'radians', 'distance' (default is 'tangent').\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=pointer.dep -o=dsi.dep --drop=5.0 --out_type=distance", short_exe, name).replace("*", &sep);
    
        DownslopeIndex { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DownslopeIndex {
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
        let mut drop_val = 2.0;
        let mut out_type = String::from("tangent");
        let mut out_val = 1;
        
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
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
            } else if vec[0].to_lowercase() == "-drop" || vec[0].to_lowercase() == "--drop" {
                if keyval {
                    drop_val = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    drop_val = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-out_type" || vec[0].to_lowercase() == "--out_type" {
                if keyval {
                    out_type = vec[1].to_lowercase();
                } else {
                    out_type = args[i+1].to_lowercase();
                }
                if out_type.contains("tan") {
                    out_type = String::from("tangent");
                    out_val = 1;
                } else if out_type.contains("sl") {
                    out_type = String::from("slope");
                    out_val = 2;
                } else if out_type.contains("rad") {
                    out_type = String::from("radians");
                    out_val = 3;
                } else {
                    out_type = String::from("distance");
                    out_val = 4;
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
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        
        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        
        let start = time::now();
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
               
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, 
                                    diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
                let (mut z, mut z_n): (f64, f64);
                let (mut max_slope, mut slope): (f64, f64);
                let mut dir: i8;
                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
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
                        } else {
                            data[col as usize] = -1i8;
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

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

        let flow_dir = Arc::new(flow_dir); // wrap flow_dir in an Arc
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let flow_dir = flow_dir.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, 
                                    diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
                let (mut z, mut zn, mut z_drop): (f64, f64, f64);
                let mut dist: f64;
                let (mut row_n, mut col_n): (isize, isize);
                let mut dir: i8;
                let mut flag: bool;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            row_n = row;
                            col_n = col;
                            z_drop = z;
                            dist = 0f64;
                            flag = true;
                            while flag {
                                // find the downstream cell
                                dir = flow_dir[(row, col)];
                                if dir > 0 {
                                    dist += grid_lengths[dir as usize];
                                    row_n += dy[dir as usize];
                                    col_n += dx[dir as usize];
                                    zn = input[(row_n, col_n)];
                                    if zn != nodata {
                                        if (z - zn) >= drop_val {
                                            z_drop = zn;
                                            flag = false;
                                        }
                                    } else {
                                        flag = false;
                                    }
                                } else {
                                    flag = false;
                                }
                            }
                            if dist > 0f64 {
                                data[col as usize] = match out_val {
                                    1 => (z - z_drop) / dist,
                                    2 => ((z - z_drop) / dist).atan().to_degrees(),
                                    3 => ((z - z_drop) / dist).atan(),
                                    _ => dist,
                                };
                            } else {
                                data[col as usize] = 0f64;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
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
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("drop_val value: {}", drop_val));
        output.add_metadata_entry(format!("Output type: {}", out_type));
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