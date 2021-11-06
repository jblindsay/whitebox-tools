/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/06/2017
Last Modified: 21/02/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
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

/// This tool is used to generate a flow accumulation grid (i.e. contributing area) using the D-infinity algorithm
/// (Tarboton, 1997). This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering
/// each grid cell is routed to one or two downslope neighbour, i.e. flow divergence is permitted. The user must
/// specify the name of the input digital elevation model or D-infinity pointer raster (`--input`). If an input DEM is
/// specified, the DEM should have been hydrologically corrected
/// to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the
/// `BreachDepressionsLeastCost` or `FillDepressions` tool.
///
/// In addition to the input DEM/pointer raster name, the user must specify the output type (`--out_type`). The output
/// flow-accumulation
/// can be 1) specific catchment area (SCA), which is the upslope contributing area divided by the contour length (taken
/// as the grid resolution), 2) total catchment area in square-metres, or 3) the number of upslope grid cells. The user
/// must also specify whether the output flow-accumulation grid should be log-tranformed, i.e. the output, if this option
/// is selected, will be the natural-logarithm of the accumulated area. This is a transformation that is often performed
/// to better visualize the contributing area distribution. Because contributing areas tend to be very high along valley
/// bottoms and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of values on
/// hillslopes tends to be 'washed out' because the palette is stretched out to represent the highest values.
/// Log-transformation (`--log`) provides a means of compensating for this phenomenon. Importantly, however, log-transformed
/// flow-accumulation grids must not be used to estimate other secondary terrain indices, such as the wetness index, or
/// relative stream power index.
///
/// Grid cells possessing the NoData value in the input DEM/pointer raster are assigned the NoData value in the output
/// flow-accumulation image. The output raster is of the float data type and continuous data scale.
///
/// # Reference
/// Tarboton, D. G. (1997). A new method for the determination of flow directions and upslope areas in grid digital
/// elevation models. Water resources research, 33(2), 309-319.
///
/// # See Also
/// `DInfPointer`, D8FlowAccumulation`, `QuinnFlowAccumulation`, `QinFlowAccumulation`, `FD8FlowAccumulation`, `MDInfFlowAccumulation`, `Rho8Pointer`, `BreachDepressionsLeastCost`, `FillDepressions`
pub struct DInfFlowAccumulation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DInfFlowAccumulation {
    pub fn new() -> DInfFlowAccumulation {
        // public constructor
        let name = "DInfFlowAccumulation".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates a D-infinity flow accumulation raster from an input DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM or D-inf Pointer File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster DEM or D-infinity pointer file.".to_owned(),
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
            name: "Output Type".to_owned(),
            flags: vec!["--out_type".to_owned()],
            description: "Output type; one of 'cells', 'sca' (default), and 'ca'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "Cells".to_owned(),
                "Specific Contributing Area".to_owned(),
                "Catchment Area".to_owned(),
            ]),
            default_value: Some("Specific Contributing Area".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Convergence Threshold (grid cells; blank for none)".to_owned(),
            flags: vec!["--threshold".to_owned()],
            description:
                "Optional convergence threshold parameter, in grid cells; default is infinity."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Log-transform the output?".to_owned(),
            flags: vec!["--log".to_owned()],
            description: "Optional flag to request the output be log-transformed.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Clip the upper tail by 1%?".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Optional flag to request clipping the display max by 1%.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Is the input raster a D-inf flow pointer?".to_owned(),
            flags: vec!["--pntr".to_owned()],
            description: "Is the input raster a D-infinity flow pointer rather than a DEM?"
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif --out_type=sca
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif --out_type=sca --threshold=10000 --log --clip", short_exe, name).replace("*", &sep);

        DInfFlowAccumulation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DInfFlowAccumulation {
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
        let mut output_file = String::new();
        let mut out_type = String::from("sca");
        let mut convergence_threshold = f64::INFINITY;
        let mut log_transform = false;
        let mut clip_max = false;
        let mut pntr_input = false;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-out_type" {
                if keyval {
                    out_type = vec[1].to_lowercase();
                } else {
                    out_type = args[i + 1].to_lowercase();
                }
                if out_type.contains("specific") || out_type.contains("sca") {
                    out_type = String::from("sca");
                } else if out_type.contains("cells") {
                    out_type = String::from("cells");
                } else {
                    out_type = String::from("ca");
                }
            } else if flag_val == "-threshold" {
                if keyval {
                    convergence_threshold = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    convergence_threshold = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-log" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    log_transform = true;
                }
            } else if flag_val == "-clip" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    clip_max = true;
                }
            } else if flag_val == "-pntr" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    pntr_input = true;
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // calculate the flow directions
        let mut flow_dir: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        let mut interior_pit_found = false;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        if !pntr_input {
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
        } else {
            // pointer file input
            flow_dir = input.get_data_as_array2d();
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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(1.0);
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
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

        let (mut row, mut col): (isize, isize);
        let mut fa: f64;
        let mut dir: f64;
        let (mut proportion1, mut proportion2): (f64, f64);
        let (mut a1, mut b1, mut a2, mut b2): (isize, isize, isize, isize);

        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            fa = output[(row, col)];
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

                if fa >= convergence_threshold {
                    if proportion1 >= proportion2 {
                        proportion1 = 1.0;
                        proportion2 = 0.0;
                        num_inflowing.decrement(b2, a2, 1i8);
                        if num_inflowing[(b2, a2)] == 0i8 {
                            stack.push((b2, a2));
                        }
                    } else {
                        proportion1 = 0.0;
                        proportion2 = 1.0;
                        num_inflowing.decrement(b1, a1, 1i8);
                        if num_inflowing[(b1, a1)] == 0i8 {
                            stack.push((b1, a1));
                        }
                    }
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

        let mut cell_area = input.configs.resolution_x * input.configs.resolution_y;
        let mut avg_cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        if out_type == "cells" {
            cell_area = 1.0;
            avg_cell_size = 1.0;
        } else if out_type == "ca" {
            avg_cell_size = 1.0;
        }

        if log_transform {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] == nodata {
                        output[(row, col)] = nodata;
                    } else {
                        output[(row, col)] = (output[(row, col)] * cell_area / avg_cell_size).ln();
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
        } else {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] == nodata {
                        output[(row, col)] = nodata;
                    } else {
                        output[(row, col)] = output[(row, col)] * cell_area / avg_cell_size;
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
        }

        output.configs.palette = "blueyellow.plt".to_string();
        if clip_max {
            output.clip_display_max(1.0);
        }
        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
