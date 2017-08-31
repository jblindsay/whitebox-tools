/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 8, 2017
Last Modified: July 8, 2017
License: MIT
*/
extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use structures::Array2D;
use tools::WhiteboxTool;

pub struct FlowLengthDiff {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl FlowLengthDiff {
    pub fn new() -> FlowLengthDiff { // public constructor
        let name = "FlowLengthDiff".to_string();
        
        let description = "Calculates the local maximum absolute difference in downslope flowpath length, useful in mapping drainage divides and ridges.".to_string();
        
        let mut parameters = "--d8_pntr          Input D8 pointer raster file.\n".to_owned();
        parameters.push_str("-o, --output       Output raster file.\n");
        parameters.push_str("--esri_pntr        Flag indicating whether the D8 pointer uses the ESRI style scheme.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --d8_pntr=pointer.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        FlowLengthDiff { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for FlowLengthDiff {
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
        let mut d8_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        
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
            if vec[0].to_lowercase() == "-d8_pntr" || vec[0].to_lowercase() == "--d8_pntr" {
                if keyval {
                    d8_file = vec[1].to_string();
                } else {
                    d8_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-esri_pntr" || vec[0].to_lowercase() == "--esri_pntr" || vec[0].to_lowercase() == "--esri_style" {
                esri_style = true;
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

        if !d8_file.contains(&sep) {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        
        if verbose { println!("Reading pointer data...") };
        let pntr = Raster::new(&d8_file, "r")?;
        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = pntr.configs.nodata;
        let cell_size_x = pntr.configs.resolution_x;
        let cell_size_y = pntr.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        
        let start = time::now();
        
        let out_nodata = -32768f64;
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        
        let mut pntr_matches: [usize; 129] = [999usize; 129];
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

        let mut dfl: Array2D<f64> = Array2D::new(rows, columns, -999f64, out_nodata)?;
        let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
        let mut dir: f64;
        let mut c: usize;
        let mut flag: bool;
        let mut dist: f64;
        let (mut x, mut y): (isize, isize);
        for row in 0..rows {
            for col in 0..columns {
                if pntr[(row, col)] >= 0.0 && pntr[(row, col)] != nodata {
                    dist = 0f64;
                    flag = false;
                    x = col;
                    y = row;
                    while !flag {
                        // find its downslope neighbour
                        dir = pntr[(y, x)];
                        if dir > 0f64 && dir != nodata {
                            if dir > 128f64 || pntr_matches[dir as usize] == 999 {
                                return Err(Error::new(ErrorKind::InvalidInput,
                                    "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                            }
                            // move x and y accordingly
                            c = pntr_matches[dir as usize];
                            x += dx[c];
                            y += dy[c];

                            dist += grid_lengths[c];

                            if dfl[(y, x)] != -999f64 {
                                dist += dfl[(y, x)];
                                flag = true;
                            }
                        } else {
                            flag = true;
                        }
                    }

                    flag = false;
                    x = col;
                    y = row;
                    while !flag {
                        dfl[(y, x)] = dist;

                        // find its downslope neighbour
                        dir = pntr[(y, x)];
                        if dir > 0f64 && dir != nodata {
                            // move x and y accordingly
                            c = pntr_matches[dir as usize];
                            x += dx[c];
                            y += dy[c];

                            dist -= grid_lengths[c];

                            if dfl[(y, x)] != -999f64 {
                                flag = true;
                            }
                        } else {
                            dfl[(y, x)] = 0f64;
                            flag = true;
                        }
                    }
                } else {
                    dfl[(row, col)] = out_nodata;
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

        let mut output = Raster::initialize_using_file(&output_file, &pntr);
        output.configs.nodata = out_nodata;
        output.reinitialize_values(-999f64);
        output.configs.data_type = DataType::F32;
        let mut z: f64;
        let mut zn: f64;
        let mut max_abs_diff: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = dfl[(row, col)];
                if z != out_nodata {
                    max_abs_diff = f64::NEG_INFINITY;
                    // Use 4-neighbour connectedness
                    for n in (0..8).filter(|x| x % 2 == 1) {
                        zn = dfl[(row + dy[n], col + dx[n])];
                        if zn != out_nodata {
                            if (z - zn).abs() > max_abs_diff {
                                max_abs_diff = (z - zn).abs();
                            } 
                        }
                    }
                    if max_abs_diff != f64::NEG_INFINITY {
                        output[(row, col)] = max_abs_diff;
                    } else {
                        output[(row, col)] = out_nodata;
                    }
                } else {
                    output[(row, col)] = out_nodata;
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
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input D8 pointer file: {}", d8_file));
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