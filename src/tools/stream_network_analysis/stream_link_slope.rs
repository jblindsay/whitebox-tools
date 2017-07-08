/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: June 27, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::cmp::Ordering::Equal;
use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct StreamLinkSlope {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl StreamLinkSlope {
    pub fn new() -> StreamLinkSlope { // public constructor
        let name = "StreamLinkSlope".to_string();
        
        let description = "Estimates the average slope of each link (or tributary) in a stream network.".to_string();
        
        let mut parameters = "--d8_pntr          Input D8 pointer raster file.\n".to_owned();
        parameters.push_str("--linkid           Input streams link ID (or tributary ID) raster file.\n");
        parameters.push_str("--dem              Input digital elevation model (DEM) raster file.");
        parameters.push_str("-o, --output       Output raster file.\n");
        parameters.push_str("--esri_pntr        Flag indicating whether the D8 pointer uses the ESRI style scheme (default is false).\n");
        parameters.push_str("--zero_background  Flag indicating whether the background value of zero should be used.\n");
       
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --d8_pntr=D8.dep --linkid=streamsID.dep --dem=dem.dep -o=output.dep
>>.*{0} -r={1} --wd=\"*path*to*data*\" --d8_pntr=D8.flt --linkid=streamsID.flt --dem=dem.flt -o=output.flt --esri_pntr --zero_background", short_exe, name).replace("*", &sep);
    
        StreamLinkSlope { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for StreamLinkSlope {
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
        let mut streams_file = String::new();
        let mut dem_file = String::new();
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
            if vec[0].to_lowercase() == "-d8_pntr" || vec[0].to_lowercase() == "--d8_pntr" {
                if keyval {
                    d8_file = vec[1].to_string();
                } else {
                    d8_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-linkid" || vec[0].to_lowercase() == "--linkid" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
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

        if !d8_file.contains(&sep) {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !streams_file.contains(&sep) {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !dem_file.contains(&sep) {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading pointer data...") };
        let pntr = Raster::new(&d8_file, "r")?;
        let pntr_nodata = pntr.configs.nodata;
        if verbose { println!("Reading link ID data...") };
        let streams = Raster::new(&streams_file, "r")?;
        if verbose { println!("Reading DEM data...") };
        let dem = Raster::new(&dem_file, "r")?;
        
        let start = time::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = streams.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }
        let cell_size_x = streams.configs.resolution_x;
        let cell_size_y = streams.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        
        
        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows || streams.configs.columns != pntr.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }
        if streams.configs.rows != dem.configs.rows || streams.configs.columns != dem.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let max_id = streams.configs.maximum as usize + 1;
        let mut min_elev = vec![f64::INFINITY; max_id];
        let mut max_elev = vec![f64::NEG_INFINITY; max_id];
        let mut link_length = vec![0.0; max_id];

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        output.configs.data_type = DataType::F32;

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
        let mut current_id: usize;
        let mut z: f64;
        let mut dir: usize;
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 && streams[(row, col)] != nodata {
                    current_id = streams[(row, col)] as usize;
                    z = dem[(row, col)];
                    if z < min_elev[current_id] { min_elev[current_id] = z; }
                    if z > max_elev[current_id] { max_elev[current_id] = z; }

                    dir = pntr[(row, col)] as usize;
                    if dir > 0 && pntr[(row, col)] != pntr_nodata {
                        if dir > 128 || pntr_matches[dir] == 999 {
                            return Err(Error::new(ErrorKind::InvalidInput,
                                "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                        }

                        link_length[current_id] += grid_lengths[pntr_matches[dir]];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for i in 0..max_elev.len() {
            if link_length[i] > 0.0 {
                max_elev[i] = (max_elev[i] - min_elev[i]) / link_length[i] * 100.0;
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 && streams[(row, col)] != nodata {
                    current_id = streams[(row, col)] as usize;
                    if link_length[current_id] > 0.0 {
                        output[(row, col)] = max_elev[current_id];
                    } else {
                        output[(row, col)] = 0.0;
                    }
                } else {
                    output[(row, col)] = background_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        if background_val == 0.0f64 {
            output.configs.palette = "spectrum_black_background.plt".to_string();
        } else {
            output.configs.palette = "spectrum.plt".to_string();
        }
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        // sort max_elev
        max_elev.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Equal));
        let t = (max_elev.len() as f64 * 0.01) as usize;
        output.configs.display_max = max_elev[t];
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input d8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Input streams ID file: {}", streams_file));
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