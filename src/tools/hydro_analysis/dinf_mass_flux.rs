/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: Dec. 29, 2017
Last Modified: 12/10/2018
License: MIT

Notes: Assumes that each of the three input rasters have the same number of rows and
       columns and that any nodata cells present are the same among each of the inputs.
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to perform a mass flux calculation using DEM-based surface flow-routing techniques. For
/// example, it could be used to model the distribution of sediment or phosphorous within a catchment. Flow-routing
/// is based on a D-Infinity flow pointer derived from an input DEM (`--dem`). The user must also specify the
/// names of loading (`--loading`), efficiency (`--efficiency`), and absorption (`--absorption`) rasters, as well
/// as the output raster. Mass Flux operates very much like a flow-accumulation operation except that rather than
/// accumulating catchment areas the algorithm routes a quantity of mass, the spatial distribution of which is
/// specified within the loading image. The efficiency and absorption rasters represent spatial distributions of
/// losses to the accumulation process, the difference being that the efficiency raster is a proportional loss (e.g.
/// only 50% of material within a particular grid cell will be directed downslope) and the absorption raster is an
/// loss specified as a quantity in the same units as the loading image. The efficiency image can range from 0 to 1,
/// or alternatively, can be expressed as a percentage. The equation for determining the mass sent from one grid cell
/// to a neighbouring grid cell is:
///
/// > *Outflowing Mass* = (*Loading* - *Absorption* + *Inflowing Mass*) &times; *Efficiency*
///
/// This tool assumes that each of the three input rasters have the same number of rows and columns and that any
/// **NoData** cells present are the same among each of the inputs.
///
/// # See Also
/// `D8MassFlux`
pub struct DInfMassFlux {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DInfMassFlux {
    pub fn new() -> DInfMassFlux {
        // public constructor
        let name = "DInfMassFlux".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Performs a D-infinity mass flux calculation.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Loading File".to_owned(),
            flags: vec!["--loading".to_owned()],
            description: "Input loading raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Efficiency File".to_owned(),
            flags: vec!["--efficiency".to_owned()],
            description: "Input efficiency raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Absorption File".to_owned(),
            flags: vec!["--absorption".to_owned()],
            description: "Input absorption raster file.".to_owned(),
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
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif --loading=load.tif --efficiency=eff.tif --absorption=abs.tif -o=output.tif", short_exe, name).replace("*", &sep);

        DInfMassFlux {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DInfMassFlux {
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
        let mut input_file = String::new();
        let mut loading_file = String::new();
        let mut efficiency_file = String::new();
        let mut absorption_file = String::new();
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
            if flag_val == "-i" || flag_val == "-dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-loading" {
                if keyval {
                    loading_file = vec[1].to_string();
                } else {
                    loading_file = args[i + 1].to_string();
                }
            } else if flag_val == "-efficiency" {
                if keyval {
                    efficiency_file = vec[1].to_string();
                } else {
                    efficiency_file = args[i + 1].to_string();
                }
            } else if flag_val == "-absorption" {
                if keyval {
                    absorption_file = vec[1].to_string();
                } else {
                    absorption_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !loading_file.contains(&sep) && !loading_file.contains("/") {
            loading_file = format!("{}{}", working_directory, loading_file);
        }
        if !efficiency_file.contains(&sep) && !efficiency_file.contains("/") {
            efficiency_file = format!("{}{}", working_directory, efficiency_file);
        }
        if !absorption_file.contains(&sep) && !absorption_file.contains("/") {
            absorption_file = format!("{}{}", working_directory, absorption_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let start = Instant::now();

        let input = Arc::new(Raster::new(&input_file, "r")?); // the DEM
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        let efficiency = Arc::new(Raster::new(&efficiency_file, "r")?); // the efficiency raster
        if efficiency.configs.rows as isize != rows
            || efficiency.configs.columns as isize != columns
        {
            return Err(Error::new(ErrorKind::InvalidInput,
                "All input images must share the same dimensions (rows and columns) and spatial extent."));
        }
        let efficiency_multiplier = if efficiency.configs.maximum > 1f64 {
            0.01f64 // assumpted to be percent...need proportion
        } else {
            1f64
        };

        let absorption = Arc::new(Raster::new(&absorption_file, "r")?); // the absorption raster
        if absorption.configs.rows as isize != rows
            || absorption.configs.columns as isize != columns
        {
            return Err(Error::new(ErrorKind::InvalidInput,
                "All input images must share the same dimensions (rows and columns) and spatial extent."));
        }

        // calculate the flow directions
        let mut flow_dir: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let grid_res = (cell_size_x + cell_size_y) / 2.0;
                let mut dir: f64;
                let mut max_slope: f64;
                let mut e0: f64;
                let mut af: f64;
                let mut ac: f64;
                let (mut e1, mut r, mut s1, mut s2, mut s, mut e2): (f64, f64, f64, f64, f64, f64);

                let ac_vals = [0f64, 1f64, 1f64, 2f64, 2f64, 3f64, 3f64, 4f64];
                let af_vals = [1f64, -1f64, 1f64, -1f64, 1f64, -1f64, 1f64, -1f64];

                let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
                let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];

                let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
                let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];

                let atanof1 = 1.0f64.atan();

                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        e0 = input[(row, col)];
                        if e0 != nodata {
                            dir = 360.0;
                            max_slope = f64::MIN;
                            neighbouring_nodata = false;
                            for i in 0..8 {
                                ac = ac_vals[i];
                                af = af_vals[i];
                                e1 = input[(row + e1_row[i], col + e1_col[i])];
                                e2 = input[(row + e2_row[i], col + e2_col[i])];
                                if e1 != nodata && e2 != nodata {
                                    if e0 > e1 && e0 > e2 {
                                        s1 = (e0 - e1) / grid_res;
                                        if s1 == 0.0 {
                                            s1 = 0.00001;
                                        }
                                        s2 = (e1 - e2) / grid_res;
                                        r = (s2 / s1).atan();
                                        s = (s1 * s1 + s2 * s2).sqrt();
                                        if s1 < 0.0 && s2 < 0.0 {
                                            s = -1.0 * s;
                                        }
                                        if s1 < 0.0 && s2 == 0.0 {
                                            s = -1.0 * s;
                                        }
                                        if s1 == 0.0 && s2 < 0.0 {
                                            s = -1.0 * s;
                                        }
                                        if s1 == 0.001 && s2 < 0.0 {
                                            s = -1.0 * s;
                                        }
                                        if r < 0.0 || r > atanof1 {
                                            if r < 0.0 {
                                                r = 0.0;
                                                s = s1;
                                            } else {
                                                r = atanof1;
                                                s = (e0 - e2) / diag_cell_size;
                                            }
                                        }
                                        if s >= max_slope && s != 0.00001 {
                                            max_slope = s;
                                            dir = af * r + ac * (PI / 2.0);
                                        }
                                    } else if e0 > e1 || e0 > e2 {
                                        if e0 > e1 {
                                            r = 0.0;
                                            s = (e0 - e1) / grid_res;
                                        } else {
                                            r = atanof1;
                                            s = (e0 - e2) / diag_cell_size;
                                        }
                                        if s >= max_slope && s != 0.00001 {
                                            max_slope = s;
                                            dir = af * r + ac * (PI / 2.0);
                                        }
                                    }
                                } else {
                                    neighbouring_nodata = true;
                                }
                            }

                            if max_slope > 0f64 {
                                // dir = Math.round((dir * (180 / Math.PI)) * 10) / 10;
                                dir = 360.0 - dir.to_degrees() + 90.0;
                                if dir > 360.0 {
                                    dir = dir - 360.0;
                                }
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1f64;
                                if !neighbouring_nodata {
                                    interior_pit_found = true;
                                }
                            }
                        } else {
                            data[col as usize] = -1f64;
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            flow_dir.set_row_data(row, data);
            if pit {
                interior_pit_found = true;
            }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow directions: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // calculate the number of inflowing cells
        let flow_dir = Arc::new(flow_dir);
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let flow_dir = flow_dir.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let start_fd = [180f64, 225f64, 270f64, 315f64, 0f64, 45f64, 90f64, 135f64];
                let end_fd = [270f64, 315f64, 360f64, 45f64, 90f64, 135f64, 180f64, 225f64];
                let mut dir: f64;
                let mut count: i8;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        dir = flow_dir[(row, col)];
                        if dir != nodata {
                            count = 0;
                            for i in 0..8 {
                                dir = flow_dir[(row + d_y[i], col + d_x[i])];
                                if dir >= 0.0 {
                                    //&& dir <= 360.0 {
                                    if i != 3 {
                                        if dir > start_fd[i] && dir < end_fd[i] {
                                            count += 1;
                                        }
                                    } else {
                                        if dir > start_fd[i] || dir < end_fd[i] {
                                            count += 1;
                                        }
                                    }
                                }
                            }
                            data[col as usize] = count;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0usize;
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            num_inflowing.set_row_data(row, data);
            for col in 0..columns {
                if num_inflowing[(row, col)] == 0i8 {
                    stack.push((row, col));
                } else if num_inflowing[(row, col)] == -1i8 {
                    num_solved_cells += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Num. inflowing neighbours: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Create the output image
        let mut output = Raster::initialize_using_file(&output_file, &input);

        // read in the loading file and initialize output with these data.
        let loading = Raster::new(&loading_file, "r")?; // the loading raster
        if loading.configs.rows as isize != rows || loading.configs.columns as isize != columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                "All input images must share the same dimensions (rows and columns) and spatial extent."));
        }
        let load_nodata = absorption.configs.nodata;

        if load_nodata == nodata {
            output.set_data_from_raster(&loading)?;
        // let _ = match output.set_data_from_raster(&loading) {
        //     Ok(_) => // do nothing,
        //     Err(e) => return Err(e),
        // };
        } else {
            let mut load: f64;
            for row in 0..rows {
                for col in 0..columns {
                    load = loading.get_value(row, col);
                    if load != load_nodata {
                        output.set_value(row, col, load);
                    } else {
                        output.set_value(row, col, nodata);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Initializing output raster: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let (mut row, mut col): (isize, isize);
        let mut fa: f64;
        let mut dir: f64;
        let (mut proportion1, mut proportion2): (f64, f64);
        let (mut a1, mut b1, mut a2, mut b2): (isize, isize, isize, isize);
        let mut eff: f64;
        let mut absorp: f64;
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            eff = efficiency.get_value(row, col) * efficiency_multiplier;
            absorp = absorption.get_value(row, col);
            fa = (output.get_value(row, col) - absorp) * eff;
            num_inflowing[(row, col)] = -1i8;

            dir = flow_dir[(row, col)];
            if dir >= 0.0 {
                // find which two cells receive flow and the proportion to each
                if dir >= 0.0 && dir < 45.0 {
                    proportion1 = (45.0 - dir) / 45.0;
                    a1 = col;
                    b1 = row - 1;
                    proportion2 = dir / 45.0;
                    a2 = col + 1;
                    b2 = row - 1;
                } else if dir >= 45.0 && dir < 90.0 {
                    proportion1 = (90.0 - dir) / 45.0;
                    a1 = col + 1;
                    b1 = row - 1;
                    proportion2 = (dir - 45.0) / 45.0;
                    a2 = col + 1;
                    b2 = row;
                } else if dir >= 90.0 && dir < 135.0 {
                    proportion1 = (135.0 - dir) / 45.0;
                    a1 = col + 1;
                    b1 = row;
                    proportion2 = (dir - 90.0) / 45.0;
                    a2 = col + 1;
                    b2 = row + 1;
                } else if dir >= 135.0 && dir < 180.0 {
                    proportion1 = (180.0 - dir) / 45.0;
                    a1 = col + 1;
                    b1 = row + 1;
                    proportion2 = (dir - 135.0) / 45.0;
                    a2 = col;
                    b2 = row + 1;
                } else if dir >= 180.0 && dir < 225.0 {
                    proportion1 = (225.0 - dir) / 45.0;
                    a1 = col;
                    b1 = row + 1;
                    proportion2 = (dir - 180.0) / 45.0;
                    a2 = col - 1;
                    b2 = row + 1;
                } else if dir >= 225.0 && dir < 270.0 {
                    proportion1 = (270.0 - dir) / 45.0;
                    a1 = col - 1;
                    b1 = row + 1;
                    proportion2 = (dir - 225.0) / 45.0;
                    a2 = col - 1;
                    b2 = row;
                } else if dir >= 270.0 && dir < 315.0 {
                    proportion1 = (315.0 - dir) / 45.0;
                    a1 = col - 1;
                    b1 = row;
                    proportion2 = (dir - 270.0) / 45.0;
                    a2 = col - 1;
                    b2 = row - 1;
                } else {
                    // else if dir >= 315.0 && dir <= 360.0 {
                    proportion1 = (360.0 - dir) / 45.0;
                    a1 = col - 1;
                    b1 = row - 1;
                    proportion2 = (dir - 315.0) / 45.0;
                    a2 = col;
                    b2 = row - 1;
                }

                if proportion1 > 0.0 {
                    // && output[(b1, a1)] != nodata {
                    output.increment(b1, a1, fa * proportion1);
                    num_inflowing.decrement(b1, a1, 1i8);
                    if num_inflowing[(b1, a1)] == 0i8 {
                        stack.push((b1, a1));
                    }
                }
                if proportion2 > 0.0 {
                    // && output[(b2, a2)] != nodata {
                    output.increment(b2, a2, fa * proportion2);
                    num_inflowing.decrement(b2, a2, 1i8);
                    if num_inflowing[(b2, a2)] == 0i8 {
                        stack.push((b2, a2));
                    }
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow accumulation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if input.get_value(row, col) == nodata {
                    output.set_value(row, col, nodata);
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Correcting values: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        output.configs.palette = "blueyellow.plt".to_string();
        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("DEM file: {}", input_file));
        output.add_metadata_entry(format!("Loading file: {}", loading_file));
        output.add_metadata_entry(format!("Efficiency file: {}", efficiency_file));
        output.add_metadata_entry(format!("Absorption file: {}", absorption_file));
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
