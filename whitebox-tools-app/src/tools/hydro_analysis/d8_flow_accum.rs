/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/016/2017
Last Modified: 29/08/2021
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

/// This tool is used to generate a flow accumulation grid (i.e. catchment area) using the
/// D8 (O'Callaghan and Mark, 1984) algorithm. This algorithm is an example of single-flow-direction
/// (SFD) method because the flow entering each grid cell is routed to only one downslope neighbour,
/// i.e. flow divergence is not permitted. The user must specify the name of the input digital
/// elevation model (DEM) or flow pointer raster (`--input`) derived using the D8 or Rho8 method
/// (`D8Pointer`, `Rho8Pointer`). If an input DEM is used, it must have
/// been hydrologically corrected to remove all spurious depressions and flat areas. DEM pre-processing
/// is usually achieved using the `BreachDepressionsLeastCost` or `FillDepressions` tools. If a D8 pointer
/// raster is input, the user must also specify the optional `--pntr` flag. If the D8 pointer follows
/// the Esri pointer scheme, rather than the default WhiteboxTools scheme, the user must also specify the
/// optional `--esri_pntr` flag.
///
/// In addition to the input DEM/pointer, the user must specify the output type. The output flow-accumulation
/// can be 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area),
/// or `specific contributing area` (i.e. the catchment area divided by the flow width. The default value
/// is `cells`. The user must also specify whether the output flow-accumulation grid should be
/// log-tranformed (`--log`), i.e. the output, if this option is selected, will be the natural-logarithm of the
/// accumulated flow value. This is a transformation that is often performed to better visualize the
/// contributing area distribution. Because contributing areas tend to be very high along valley bottoms
/// and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of
/// values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the
/// highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly,
/// however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain
/// indices, such as the wetness index, or relative stream power index.
///
/// Grid cells possessing the **NoData** value in the input DEM/pointer raster are assigned the **NoData**
/// value in the output flow-accumulation image.
///
/// # Reference
/// O'Callaghan, J. F., & Mark, D. M. 1984. The extraction of drainage networks from digital elevation data. 
/// *Computer Vision, Graphics, and Image Processing*, 28(3), 323-344.
///
/// # See Also:
/// `FD8FlowAccumulation`, `QuinnFlowAccumulation`, `QinFlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `Rho8Pointer`, `D8Pointer`, `BreachDepressionsLeastCost`, `FillDepressions`
pub struct D8FlowAccumulation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl D8FlowAccumulation {
    pub fn new() -> D8FlowAccumulation {
        // public constructor
        let name = "D8FlowAccumulation".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates a D8 flow accumulation raster from an input DEM or flow pointer."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM or D8 Pointer File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster DEM or D8 pointer file.".to_owned(),
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

        parameters.push(ToolParameter{
            name: "Output Type".to_owned(), 
            flags: vec!["--out_type".to_owned()], 
            description: "Output type; one of 'cells' (default), 'catchment area', and 'specific contributing area'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["cells".to_owned(), "catchment area".to_owned(), "specific contributing area".to_owned()]),
            default_value: Some("cells".to_owned()),
            optional: true
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
            name: "Is the input raster a D8 flow pointer?".to_owned(),
            flags: vec!["--pntr".to_owned()],
            description: "Is the input raster a D8 flow pointer rather than a DEM?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "If a pointer is input, does it use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "Input D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output a D8 pointer raster at the same time?".to_owned(),
            flags: vec!["--pntr_output".to_owned()],
            description: "Should a D8 pointer raster be generated along with the D8 Flow raster?".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif --out_type='cells'
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif --out_type='specific catchment area' --log --clip", short_exe, name).replace("*", &sep);

        D8FlowAccumulation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for D8FlowAccumulation {
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
        let mut log_transform = false;
        let mut clip_max = false;
        let mut pntr_input = false;
        let mut esri_style = false;
        let mut pntr_output = false;

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
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
                    pntr_input = true;
                }
            } else if flag_val == "-pntr_output" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    pntr_output = true;
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
        // -2 indicates NoData, -1 indicates no downslope neighbour, 0-7 indicate flow to one neighbour.
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -2, -2)?;
        let mut interior_pit_found = false;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        if !pntr_input {
            // calculate the flow direction from the input DEM
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    // let nodata = input.configs.nodata;
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
                        let mut data: Vec<i8> = vec![-2i8; columns as usize];
                        for col in 0..columns {
                            z = input.get_value(row, col);
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
            // The input raster is a D8 flow pointer
            // map the pointer values into 0-7 style pointer values
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    // let nodata = input.configs.nodata;
                    let mut z: f64;
                    let mut interior_pit_found = false;
                    let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                    let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                    let mut neighbouring_nodata: bool;
                    // Create a mapping from the pointer values to cells offsets.
                    // This may seem wasteful, using only 8 of 129 values in the array,
                    // but the mapping method is far faster than calculating z.ln() / ln(2.0).
                    // It's also a good way of allowing for different point styles.
                    let mut pntr_matches: [i8; 129] = [-2i8; 129];
                    if !esri_style {
                        // This maps Whitebox-style D8 pointer values
                        // onto the cell offsets in d_x and d_y.
                        pntr_matches[1] = 0i8;
                        pntr_matches[2] = 1i8;
                        pntr_matches[4] = 2i8;
                        pntr_matches[8] = 3i8;
                        pntr_matches[16] = 4i8;
                        pntr_matches[32] = 5i8;
                        pntr_matches[64] = 6i8;
                        pntr_matches[128] = 7i8;
                    } else {
                        // This maps Esri-style D8 pointer values
                        // onto the cell offsets in d_x and d_y.
                        pntr_matches[1] = 1i8;
                        pntr_matches[2] = 2i8;
                        pntr_matches[4] = 3i8;
                        pntr_matches[8] = 4i8;
                        pntr_matches[16] = 5i8;
                        pntr_matches[32] = 6i8;
                        pntr_matches[64] = 7i8;
                        pntr_matches[128] = 0i8;
                    }
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data: Vec<i8> = vec![-2i8; columns as usize];
                        for col in 0..columns {
                            z = input.get_value(row, col);
                            if z != nodata {
                                if z > 0f64 {
                                    data[col as usize] = pntr_matches[z as usize];
                                } else {
                                    data[col as usize] = -1i8;
                                    // is this no-flow cell interior?
                                    neighbouring_nodata = false;
                                    for i in 0..8 {
                                        if input.get_value(row + dy[i], col + dx[i]) == nodata {
                                            neighbouring_nodata = true;
                                            break;
                                        }
                                    }
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
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let out_nodata = -32768f64;
        output.configs.nodata = out_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous; // if the input is a pointer, this may not be the case by default.
        output.configs.data_type = DataType::F32;
        output.reinitialize_values(1.0);
        drop(input);

        // calculate the number of inflowing cells
        let flow_dir = Arc::new(flow_dir);
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            // let input = input.clone();
            let flow_dir = flow_dir.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
                // let mut z: f64;
                let mut count: i8;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        // z = input.get_value(row, col);
                        // if z != nodata {
                        if flow_dir.get_value(row, col) != -2i8 {
                            count = 0i8;
                            for i in 0..8 {
                                if flow_dir.get_value(row + dy[i], col + dx[i]) == inflowing_vals[i]
                                {
                                    count += 1;
                                }
                            }
                            data[col as usize] = count;
                        } else {
                            data[col as usize] = -1i8;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

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

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let mut dir: i8;
        let mut fa: f64;
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            fa = output[(row, col)];
            num_inflowing.decrement(row, col, 1i8);
            dir = flow_dir.get_value(row, col);
            if dir >= 0 {
                row_n = row + dy[dir as usize];
                col_n = col + dx[dir as usize];
                output.increment(row_n, col_n, fa);
                num_inflowing.decrement(row_n, col_n, 1i8);
                if num_inflowing.get_value(row_n, col_n) == 0i8 {
                    stack.push((row_n, col_n));
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

        let mut cell_area = cell_size_x * cell_size_y;
        // if flow width is allowed to vary by direction, the flow accumulation output will not
        // increase continuously downstream and any applications involving stream network
        // extraction will encounter issues with discontinuous streams. The Whitebox GAT tool
        // used a constant flow width value. I'm reverting this tool to the equivalent.
        // let mut flow_widths = [
        //     diag_cell_size,
        //     cell_size_y,
        //     diag_cell_size,
        //     cell_size_x,
        //     diag_cell_size,
        //     cell_size_y,
        //     diag_cell_size,
        //     cell_size_x,
        // ];

        let avg_cell_size = (cell_size_x + cell_size_y) / 2.0;
        let mut flow_widths = [
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
            avg_cell_size,
        ];
        if out_type == "cells" {
            cell_area = 1.0;
            flow_widths = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        } else if out_type == "ca" {
            flow_widths = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        }

        if log_transform {
            for row in 0..rows {
                for col in 0..columns {
                    // if input.get_value(row, col) == nodata {
                    if flow_dir.get_value(row, col) == -2 {
                        output.set_value(row, col, out_nodata);
                    } else {
                        dir = flow_dir.get_value(row, col);
                        if dir >= 0 {
                            output[(row, col)] =
                                (output[(row, col)] * cell_area / flow_widths[dir as usize]).ln();
                        } else {
                            output[(row, col)] =
                                (output[(row, col)] * cell_area / flow_widths[3]).ln();
                        }
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
                    // if input.get_value(row, col) == nodata {
                    if flow_dir.get_value(row, col) == -2 {
                        output.set_value(row, col, out_nodata);
                    } else {
                        dir = flow_dir.get_value(row, col);
                        if dir >= 0 {
                            output.set_value(
                                row,
                                col,
                                output.get_value(row, col) * cell_area / flow_widths[dir as usize],
                            );
                        } else {
                            output.set_value(
                                row,
                                col,
                                output.get_value(row, col) * cell_area / flow_widths[3],
                            );
                        }
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

        
        if pntr_output {
            // Initialize raster for D8 pointer output
            let file_path = path::Path::new(&output_file);
            let extension = file_path.extension().unwrap().to_str().unwrap().to_string();
            let basename = file_path.file_stem().unwrap().to_str().unwrap().to_string();
            let dirpath = file_path.parent().unwrap();
            let d8pointer_file = dirpath.join(format!("{}_d8pointer.{}", basename, extension))
                .to_str().unwrap().to_string();

            let header = output.configs.clone();
            drop(output);
            let mut d8pointer = Raster::initialize_using_config(&d8pointer_file, &header);
            d8pointer.configs.nodata = 255_f64;
            d8pointer.configs.data_type = DataType::U8;

            d8pointer.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            d8pointer.add_metadata_entry(format!("Input file: {}", input_file));

            // Updating pointer values using WhiteboxTools' base-2 numeric index convention
            let pntr_matches_u8 = [255u8, 0u8, 1u8, 2u8, 4u8, 8u8, 16u8, 32u8, 64u8, 128u8];
            for row in 0..rows {
                for col in 0..columns {
                    let idx = flow_dir.get_value(row, col) + 2;
                    d8pointer.set_value(row, col, pntr_matches_u8[idx as usize].into());
                }
            }
    
            if verbose {
                println!("Saving D8 pointer...")
            };
            let _ = match d8pointer.write() {
                Ok(_) => {
                    if verbose {
                        println!("D8 pointer file written")
                    }
                }
                Err(e) => return Err(e),
            };
        }

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
