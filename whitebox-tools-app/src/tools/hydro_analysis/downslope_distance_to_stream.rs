/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 9/07/2017
Last Modified: 04/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to calculate the distance from each grid cell in a raster to the nearest stream cell,
/// measured along the downslope flowpath. The user must specify the name of an input digital elevation model (`--dem`)
/// and streams raster (`--streams`). The DEM must have been pre-processed to remove artifact topographic depressions
/// and flat areas (see `BreachDepressions`). The streams raster should have been created using one of the DEM-based
/// stream mapping methods, i.e. contributing area thresholding. Stream cells are designated in this raster as all
/// non-zero values. The output of this tool, along with the `ElevationAboveStream` tool, can be useful for preliminary
/// flood plain mapping when combined with high-accuracy DEM data.
///
/// By default, this tool calculates flow-path using the D8 flow algorithm. However, the user may specify (`--dinf`) that
/// the tool should use the D-infinity algorithm instead.
///
/// # See Also
/// `ElevationAboveStream`, `DistanceToOutlet`
pub struct DownslopeDistanceToStream {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DownslopeDistanceToStream {
    pub fn new() -> DownslopeDistanceToStream {
        // public constructor
        let name = "DownslopeDistanceToStream".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Measures distance to the nearest downslope stream cell.".to_string();

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

        parameters.push(ToolParameter {
            name: "Use the D-infinity flow algorithm instead of D8?".to_owned(),
            flags: vec!["--dinf".to_owned()],
            description: "Use the D-infinity flow algoirthm instead of D8?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem='dem.tif' --streams='streams.tif' -o='output.tif'", short_exe, name).replace("*", &sep);

        DownslopeDistanceToStream {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DownslopeDistanceToStream {
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
        let mut use_dinf = false;

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
            if flag_val == "-dem" {
                if keyval {
                    dem_file = vec[1].to_string();
                } else {
                    dem_file = args[i + 1].to_string();
                }
            } else if flag_val == "-streams" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-dinf" || flag_val == "-dinfinity" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_dinf = true;
                }
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

        if !use_dinf {
            /////////////////////////////////////////////
            // Perform the D8 flow pointer calculation //
            /////////////////////////////////////////////
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
                            z = dem.get_value(row, col);
                            if z != nodata {
                                dir = 0i8;
                                max_slope = f64::MIN;
                                neighbouring_nodata = false;
                                for i in 0..8 {
                                    z_n = dem.get_value(row + dy[i], col + dx[i]);
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
                    // stream cells get added to the stack; nodata cells get assigned that in the output
                    if streams.get_value(row, col) > 0f64
                        && streams.get_value(row, col) != streams_nodata
                    {
                        output.set_value(row, col, 0f64);
                        stack.push((row, col, 0f64));
                    }
                    if dem.get_value(row, col) == nodata {
                        output.set_value(row, col, nodata);
                        num_solved_cells += 1;
                    }
                    if flow_dir.get_value(row, col) == -1 {
                        if output.get_value(row, col) != 0f64 {
                            stack.push((row, col, nodata));
                            output.set_value(row, col, nodata);
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

            ////////////////////////////////////////////////
            // Calculate the downslope distance to stream //
            ////////////////////////////////////////////////
            let num_cells = dem.num_cells();
            let mut stream_dist: f64;
            let mut dist: f64;
            let (mut row, mut col): (isize, isize);
            let (mut row_n, mut col_n): (isize, isize);
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
            while !stack.is_empty() {
                let cell = stack.pop().expect("Error during pop operation.");
                row = cell.0;
                col = cell.1;
                stream_dist = cell.2;
                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = col + dx[n];
                    if flow_dir.get_value(row_n, col_n) == inflowing_vals[n]
                        && output.get_value(row_n, col_n) == background_value
                    {
                        if stream_dist != nodata {
                            dist = stream_dist + grid_lengths[n];
                            output.set_value(row_n, col_n, dist);
                            stack.push((row_n, col_n, dist));
                        } else {
                            output.set_value(row_n, col_n, nodata);
                            stack.push((row_n, col_n, nodata));
                        }
                    }
                }
                if verbose {
                    num_solved_cells += 1;
                    progress =
                        (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

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
        } else {
            // calculate the D-inf flow directions
            let mut flow_dir: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
            let mut interior_pit_found = false;
            let mut num_procs = num_cpus::get() as isize;
            let configs = whitebox_common::configs::get_configs()?;
            let max_procs = configs.max_procs;
            if max_procs > 0 && max_procs < num_procs {
                num_procs = max_procs;
            }

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = dem.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let nodata = input.configs.nodata;
                    let grid_res = (cell_size_x + cell_size_y) / 2.0;
                    let mut dir: f64;
                    let mut max_slope: f64;
                    let mut e0: f64;
                    let mut af: f64;
                    let mut ac: f64;
                    let (mut e1, mut r, mut s1, mut s2, mut s, mut e2): (
                        f64,
                        f64,
                        f64,
                        f64,
                        f64,
                        f64,
                    );

                    let ac_vals = [0f64, 1f64, 1f64, 2f64, 2f64, 3f64, 3f64, 4f64];
                    let af_vals = [1f64, -1f64, 1f64, -1f64, 1f64, -1f64, 1f64, -1f64];

                    let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
                    let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];

                    let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
                    let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];

                    let atanof1 = 1.0f64.atan();

                    let mut neighbouring_nodata: bool;
                    let mut interior_pit_found = false;
                    const HALF_PI: f64 = PI / 2f64;
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
                                            s2 = (e1 - e2) / grid_res;
                                            r = if s1 != 0f64 {
                                                (s2 / s1).atan()
                                            } else {
                                                PI / 2.0
                                            };
                                            s = (s1 * s1 + s2 * s2).sqrt();
                                            if s1 < 0.0 && s2 < 0.0 {
                                                s *= -1.0;
                                            }
                                            if s1 < 0.0 && s2 == 0.0 {
                                                s *= -1.0;
                                            }
                                            if s1 == 0.0 && s2 < 0.0 {
                                                s *= -1.0;
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
                                                dir = af * r + ac * HALF_PI;
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
                                                dir = af * r + ac * HALF_PI;
                                            }
                                        }
                                    } else {
                                        neighbouring_nodata = true;
                                    }
                                }

                                if max_slope > 0f64 {
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

            let mut output = Raster::initialize_using_file(&output_file, &dem);
            output.reinitialize_values(f64::MAX);
            let mut queue = VecDeque::new();
            let mut num_outflowing = Array2D::new(rows, columns, 0i8, -1i8)?;
            let mut dir: f64;
            let mut z: f64;
            let mut num_solved_cells = 0;
            for row in 0..rows {
                for col in 0..columns {
                    z = dem.get_value(row, col);
                    if z != nodata {
                        dir = flow_dir.get_value(row, col);
                        if dir != -1.0 {
                            if dir == 0.0
                                || dir == 45.0
                                || dir == 90.0
                                || dir == 135.0
                                || dir == 180.0
                                || dir == 225.0
                                || dir == 270.0
                                || dir == 315.0
                                || dir == 360.0
                            {
                                num_outflowing.set_value(row, col, 1);
                            } else if dir != -1.0 {
                                num_outflowing.set_value(row, col, 2);
                            }
                        } else {
                            num_outflowing.set_value(row, col, 0);
                        }
                    } else {
                        num_outflowing.set_value(row, col, 0);
                        output.set_value(row, col, nodata);
                        num_solved_cells += 1;
                    }
                    if streams.get_value(row, col) > 0f64 {
                        queue.push_back((row, col));
                        output.set_value(row, col, 0f64);
                        num_outflowing.set_value(row, col, -1);
                        num_solved_cells += 1;
                    }
                }
            }

            let start_fd = [180f64, 225f64, 270f64, 315f64, 0f64, 45f64, 90f64, 135f64];
            let end_fd = [270f64, 315f64, 360f64, 45f64, 90f64, 135f64, 180f64, 225f64];
            let neighbour_dist = [
                diag_cell_size,
                cell_size_x,
                diag_cell_size,
                cell_size_y,
                diag_cell_size,
                cell_size_x,
                diag_cell_size,
                cell_size_y,
            ];
            // let (mut row, mut column): (isize, isize);
            let (mut row_n, mut col_n): (isize, isize);
            let mut dist: f64;
            let num_cells = rows * columns;
            while !queue.is_empty() {
                let (row, column) = queue.pop_front().unwrap();
                dist = output.get_value(row, column);

                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = column + dx[n];
                    if num_outflowing.get_value(row_n, col_n) > 0 {
                        dir = flow_dir.get_value(row_n, col_n);

                        if n != 3 {
                            if dir > start_fd[n] && dir < end_fd[n] {
                                if dist + neighbour_dist[n] < output.get_value(row_n, col_n) {
                                    output.set_value(row_n, col_n, dist + neighbour_dist[n]);
                                }
                                num_outflowing.decrement(row_n, col_n, 1);
                            }
                        } else {
                            if dir > start_fd[n] || dir < end_fd[n] {
                                if dist + neighbour_dist[n] < output.get_value(row_n, col_n) {
                                    output.set_value(row_n, col_n, dist + neighbour_dist[n]);
                                }
                                num_outflowing.decrement(row_n, col_n, 1);
                            }
                        }

                        if num_outflowing.get_value(row_n, col_n) == 0 {
                            queue.push_back((row_n, col_n));
                            num_solved_cells += 1;
                        }
                    }
                }

                progress = ((num_solved_cells as f64 / num_cells as f64) * 100.0) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }

            for row in 0..rows {
                for col in 0..columns {
                    if output.get_value(row, col) == f64::MAX {
                        output.set_value(row, col, nodata);
                        num_solved_cells += 1;

                        progress = ((num_solved_cells as f64 / num_cells as f64) * 100.0) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }

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
        }

        Ok(())
    }
}
