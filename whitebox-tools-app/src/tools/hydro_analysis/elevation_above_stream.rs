/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 9, 2017
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to calculate the elevation of each grid cell in a raster above the nearest stream cell,
/// measured along the downslope flowpath. This terrain index, a measure of relative topographic position, is
/// essentially equivalent to the 'height above drainage' (HAND), as described by Renno et al. (2008). The user must
/// specify the name of an input digital elevation model (`--dem`) and streams raster (`--streams`). The DEM
/// must have been pre-processed to remove artifact topographic depressions and flat areas (see `BreachDepressions`).
/// The streams raster should have been created using one of the DEM-based stream mapping methods, i.e. contributing
/// area thresholding. Stream cells are designated in this raster as all non-zero values. The output of this tool,
/// along with the `DownslopeDistanceToStream` tool, can be useful for preliminary flood plain mapping when combined
/// with high-accuracy DEM data.
///
/// The difference between `ElevationAboveStream` and `ElevationAboveStreamEuclidean` is that the former calculates
/// distances along drainage flow-paths while the latter calculates straight-line distances to streams channels.
///
/// # Reference
/// Renno, C. D., Nobre, A. D., Cuartas, L. A., Soares, J. V., Hodnett, M. G., Tomasella, J., & Waterloo, M. J.
/// (2008). HAND, a new terrain descriptor using SRTM-DEM: Mapping terra-firme rainforest environments in Amazonia.
/// Remote Sensing of Environment, 112(9), 3469-3481.
///
/// # See Also
/// `ElevationAboveStreamEuclidean`, `DownslopeDistanceToStream`, `ElevAbovePit`, `BreachDepressions`
pub struct ElevationAboveStream {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ElevationAboveStream {
    pub fn new() -> ElevationAboveStream {
        // public constructor
        let name = "ElevationAboveStream".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates the elevation of cells above the nearest downslope stream cell."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem='dem.tif' --streams='streams.tif' -o='output.tif'", short_exe, name).replace("*", &sep);

        ElevationAboveStream {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ElevationAboveStream {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut dem_file = String::new();
        let mut streams_file = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-dem" || flag_val == "-i" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading DEM data...")
        };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);
        if verbose {
            println!("Reading streams data...")
        };
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        let cell_size_x = dem.configs.resolution_x;
        let cell_size_y = dem.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let flow_nodata = -2i8;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let inflowing_vals = [4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8];

        // make sure the input files have the same size
        if dem.configs.rows != streams.configs.rows
            || dem.configs.columns != streams.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let dem = dem.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let grid_lengths = [
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                ];
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
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            flow_dir.set_row_data(row, data);
            if pit {
                interior_pit_found = true;
            }
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
        let mut stream_elev: f64;
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            stream_elev = cell.2;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                if flow_dir[(row_n, col_n)] == inflowing_vals[n]
                    && output[(row_n, col_n)] == background_value
                {
                    stack.push((row_n, col_n, stream_elev));
                    if stream_elev != nodata {
                        output[(row_n, col_n)] = dem[(row_n, col_n)] - stream_elev;
                    } else {
                        output[(row_n, col_n)] = nodata;
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

        // let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        // let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        // let mut flag: bool;
        // let (mut x, mut y): (isize, isize);
        // let mut val: f64;
        // let mut dir: i8;
        // let mut stream_elev: f64;
        // for row in 0..rows {
        //     for col in 0..columns {
        //         if output[(row, col)] == background_value {
        //             flag = false;
        //             x = col;
        //             y = row;
        //             stream_elev = nodata;
        //             while !flag {
        //                 // find its downslope neighbour
        //                 dir = flow_dir[(y, x)];
        //                 if dir >= 0 {
        //                     // move x and y accordingly
        //                     x += dx[dir as usize];
        //                     y += dy[dir as usize];

        //                     // if the new cell already has a value in the output, use that as the outletID
        //                     val = output[(y, x)];
        //                     if val != background_value && val != nodata {
        //                         stream_elev = dem[(y, x)] - val;
        //                         flag = true;
        //                     } else if val == nodata {
        //                         flag = true;
        //                     }
        //                 } else {
        //                     flag = true;
        //                 }
        //             }

        //             if stream_elev != nodata {
        //                 flag = false;
        //                 x = col;
        //                 y = row;
        //                 while !flag {
        //                     output[(y, x)] = dem[(y, x)] - stream_elev;
        //                     // find its downslope neighbour
        //                     dir = flow_dir[(y, x)];
        //                     if dir >= 0 {
        //                         // move x and y accordingly
        //                         x += dx[dir as usize];
        //                         y += dy[dir as usize];

        //                         // if the new cell already has a value in the output, use that as the outletID
        //                         val = output[(y, x)];
        //                         if val != background_value {
        //                             flag = true;
        //                         } else if val == nodata {
        //                             flag = true;
        //                         }
        //                     } else {
        //                         flag = true;
        //                     }
        //                 }
        //             } else {
        //                 output[(row, col)] = nodata;
        //             }
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("DEM file: {}", dem_file));
        output.add_metadata_entry(format!("Streams file: {}", streams_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

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
