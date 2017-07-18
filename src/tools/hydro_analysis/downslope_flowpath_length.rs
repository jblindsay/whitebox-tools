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

pub struct DownslopeFlowpathLength {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DownslopeFlowpathLength {
    pub fn new() -> DownslopeFlowpathLength { // public constructor
        let name = "DownslopeFlowpathLength".to_string();
        
        let description = "Calculates the downslope flowpath length from each cell to basin outlet.".to_string();
        
        let mut parameters = "--d8_pntr          Input D8 pointer raster file.\n".to_owned();
        parameters.push_str("--watersheds       Optional input watershed raster file.\n");
        parameters.push_str("--weights          Optional input weights raster file.\n");
        parameters.push_str("-o, --output       Output raster file.\n");
        parameters.push_str("--esri_pntr        Flag indicating whether the D8 pointer uses the ESRI style scheme.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=pointer.dep -o=flowpath_len.dep
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=pointer.flt --watersheds=basin.flt --weights=weights.flt -o=flowpath_len.flt --esri_pntr", short_exe, name).replace("*", &sep);
    
        DownslopeFlowpathLength { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DownslopeFlowpathLength {
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
        let mut watersheds_file = String::new();
        let mut weights_file = String::new();
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
            } else if vec[0].to_lowercase() == "-watersheds" || vec[0].to_lowercase() == "--watersheds" {
                if keyval {
                    watersheds_file = vec[1].to_string();
                } else {
                    watersheds_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-weights" || vec[0].to_lowercase() == "--weights" {
                if keyval {
                    weights_file = vec[1].to_string();
                } else {
                    weights_file = args[i+1].to_string();
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
        let use_watersheds: bool;
        if !watersheds_file.is_empty() {
            use_watersheds = true;
            if !watersheds_file.contains(&sep) {
                watersheds_file = format!("{}{}", working_directory, watersheds_file);
            }
        } else {
            use_watersheds = false;
        }
        let use_weights: bool;
        if !weights_file.is_empty() {
            use_weights = true;
            if !weights_file.contains(&sep) {
                weights_file = format!("{}{}", working_directory, weights_file);
            }
        } else {
            use_weights = false
        }
        
        if verbose { println!("Reading pointer data...") };
        let pntr = Raster::new(&d8_file, "r")?;
        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = pntr.configs.nodata;
        let cell_size_x = pntr.configs.resolution_x;
        let cell_size_y = pntr.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        
        if verbose { println!("Initializing watershed data...") };
        let watersheds: Array2D<f64> = match use_watersheds {
            false => Array2D::new(1, 1, 1f64, 1f64)?,
            true => {
                // if verbose { println!("Reading watershed data...") };
                let r = Raster::new(&watersheds_file, "r")?;
                if r.configs.rows != rows as usize || r.configs.columns != columns as usize {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                        "The input files must have the same number of rows and columns and spatial extent."));
                }
                r.get_data_as_array2d()
            },
        };
        // let watershed_nodata = watersheds.nodata;

        if verbose { println!("Initializing weights data...") };
        let weights: Array2D<f64> = match use_weights {
            false => Array2D::new(1, 1, 1f64, 1f64)?,
            true => {
                // if verbose { println!("Reading weights data...") };
                let r = Raster::new(&weights_file, "r")?;
                if r.configs.rows != rows as usize || r.configs.columns != columns as usize {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                        "The input files must have the same number of rows and columns and spatial extent."));
                }
                r.get_data_as_array2d()
            },
        };
        
        let start = time::now();

        let mut output = Raster::initialize_using_file(&output_file, &pntr);
        let out_nodata = -32768f64;
        output.configs.nodata = out_nodata;
        output.reinitialize_values(-999f64);
        output.configs.data_type = DataType::F32;

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

        let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
        let mut current_id: f64;
        let mut dir: f64;
        let mut c: usize;
        let mut flag: bool;
        let mut dist: f64;
        let (mut x, mut y): (isize, isize);
        for row in 0..rows {
            for col in 0..columns {
                if pntr[(row, col)] >= 0.0 && pntr[(row, col)] != nodata {
                    current_id = watersheds[(row, col)];
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

                            dist += grid_lengths[c] * weights[(y, x)];

                            if output[(y, x)] != -999f64 {
                                dist += output[(y, x)] * weights[(y, x)];
                                flag = true;
                            } else if watersheds[(y, x)] != current_id {
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
                        output[(y, x)] = dist;

                        // find its downslope neighbour
                        dir = pntr[(y, x)];
                        if dir > 0f64 && dir != nodata {
                            // move x and y accordingly
                            c = pntr_matches[dir as usize];
                            x += dx[c];
                            y += dy[c];

                            dist -= grid_lengths[c] * weights[(y, x)];

                            if output[(y, x)] != -999f64 || watersheds[(y, x)] != current_id {
                                flag = true;
                            }
                        } else {
                            output[(y, x)] = 0f64;
                            flag = true;
                        }
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
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input D8 pointer file: {}", d8_file));
        if use_watersheds {
            output.add_metadata_entry(format!("Input watersheds file: {}", watersheds_file));
        }
        if use_weights {
            output.add_metadata_entry(format!("Input weights file: {}", weights_file));
        }
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