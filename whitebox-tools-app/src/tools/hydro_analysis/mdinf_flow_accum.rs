/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/02/2020
Last Modified: 12/02/2020
License: MIT

This tool has been created as a port of the Java MD-infinity implementation written by Jan Seibert (jan.seibert@geo.uzh.ch) and
Marc Vis (marc.vis@geo.uzh.ch) for Whitebox GAT:

https://github.com/jblindsay/whitebox-geospatial-analysis-tools/blob/master/GeasyTools/plugins/FlowAccumMDInf.java
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

/// This tool is used to generate a flow accumulation grid (i.e. contributing area) using the MD-infinity algorithm
/// (Seibert and McGlynn, 2007). This algorithm is an examples of a multiple-flow-direction (MFD) method because the flow entering
/// each grid cell is routed to one or two downslope neighbour, i.e. flow divergence is permitted. The user must
/// specify the name of the input digital elevation model (`--dem`). The DEM should have been hydrologically corrected
/// to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using the
/// `BreachDepressions` or `FillDepressions` tool.
///
/// In addition to the input flow-pointer grid name, the user must specify the output type (`--out_type`). The output
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
/// Grid cells possessing the NoData value in the input DEM raster are assigned the NoData value in the output
/// flow-accumulation image. The output raster is of the float data type and continuous data scale.
///
/// # Reference
/// Seibert, J. and McGlynn, B.L., 2007. A new triangular multiple flow direction algorithm for computing upslope areas from
/// gridded digital elevation models. Water resources research, 43(4).
///
/// # See Also
/// `D8FlowAccumulation`, `FD8FlowAccumulation`, `QuinnFlowAccumulation`, `QinFlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `Rho8Pointer`, `BreachDepressionsLeastCost`
pub struct MDInfFlowAccumulation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MDInfFlowAccumulation {
    pub fn new() -> MDInfFlowAccumulation {
        // public constructor
        let name = "MDInfFlowAccumulation".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates an FD8 flow accumulation raster from an input DEM.".to_string();

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
            description: "Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["cells".to_owned(), "specific contributing area".to_owned(), "catchment area".to_owned()]),
            default_value: Some("specific contributing area".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Exponent Parameter".to_owned(),
            flags: vec!["--exponent".to_owned()],
            description: "Optional exponent parameter; default is 1.1.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.1".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --out_type='cells'
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --out_type='catchment area' --exponent=1.5 --threshold=10000 --log --clip", short_exe, name).replace("*", &sep);

        MDInfFlowAccumulation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MDInfFlowAccumulation {
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
        let mut exponent = 1.1;
        let mut convergence_threshold = f64::INFINITY;
        let mut log_transform = false;
        let mut clip_max = false;

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
                input_file = if keyval {
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
            } else if flag_val == "-out_type" {
                out_type = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i + 1].to_lowercase()
                };
                out_type = if out_type.contains("specific") || out_type.contains("sca") {
                    String::from("sca")
                } else if out_type.contains("cells") {
                    String::from("cells")
                } else {
                    String::from("ca")
                };
            } else if flag_val == "-exponent" {
                exponent = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-threshold" {
                convergence_threshold = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                if convergence_threshold == 0f64 {
                    convergence_threshold = f64::INFINITY;
                }
            } else if flag_val == "-log" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    log_transform = true;
                }
            } else if flag_val == "-clip" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    clip_max = true;
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
        let grid_res = (cell_size_x + cell_size_y) / 2f64;

        // calculate the number of inflowing cells
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut z: f64;
                let mut count: i8;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            count = 0i8;
                            for i in 0..8 {
                                if input.get_value(row + d_y[i], col + d_x[i]) > z {
                                    count += 1;
                                }
                            }
                            data[col as usize] = count;
                            if count == 8 {
                                interior_pit_found = true;
                            }
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(1.0);
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            num_inflowing.set_row_data(row, data);
            if pit {
                interior_pit_found = true;
            }
            for col in 0..columns {
                if num_inflowing.get_value(row, col) == 0i8 {
                    stack.push((row, col));
                } else if num_inflowing.get_value(row, col) == -1i8 {
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

        let xd = [0, -1, -1, -1, 0, 1, 1, 1];
        let yd = [-1, -1, 0, 1, 1, 1, 0, -1];
        let dd = [
            1f64,
            2f64.sqrt(),
            1f64,
            2f64.sqrt(),
            1f64,
            2f64.sqrt(),
            1f64,
            2f64.sqrt(),
        ];

        let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
        let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let (mut z, mut z_n, mut p1, mut p2): (f64, f64, f64, f64);
        let (mut z1, mut z2): (f64, f64);
        let (mut hr, mut hs): (f64, f64);
        let (mut nx, mut ny, mut nz): (f64, f64, f64);
        let (mut i, mut ii, mut i_max): (usize, usize, usize);
        let quarter_pi = PI / 4f64;
        let mut fa: f64;
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
        let (mut max_slope, mut slope): (f64, f64);
        let mut dir: i8;
        let mut total_weights: f64;
        let mut r_facet: [f64; 8] = [0.0; 8];
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            z = input.get_value(row, col);
            fa = output.get_value(row, col);
            num_inflowing.set_value(row, col, -1i8);

            total_weights = 0.0;
            let mut weights: [f64; 8] = [0.0; 8];
            let mut downslope: [bool; 8] = [false; 8];
            if fa < convergence_threshold {
                let mut valley: [f64; 8] = [0.0; 8];
                let mut valley_sum: f64 = 0f64;
                let mut valley_max: f64 = 0f64;
                i_max = 0;
                let mut s_facet = [
                    nodata, nodata, nodata, nodata, nodata, nodata, nodata, nodata,
                ];
                for c in 0..8 {
                    i = c;
                    ii = (i + 1) % 8;

                    p1 = input.get_value(row + yd[i], col + xd[i]);
                    p2 = input.get_value(row + yd[ii], col + xd[ii]);
                    if p1 < z && p1 != nodata {
                        downslope[c] = true;
                    }
                    if p1 != nodata && p2 != nodata {
                        // Calculate the elevation difference between the center point and the points p1 and p2
                        z1 = p1 - z;
                        z2 = p2 - z;

                        // Calculate the coordinates of the normal to the triangular facet
                        nx = (yd[i] as f64 * z2 - yd[ii] as f64 * z1) * grid_res;
                        ny = (xd[ii] as f64 * z1 - xd[i] as f64 * z2) * grid_res;
                        nz = (xd[i] * yd[ii] - xd[ii] * yd[i]) as f64 * grid_res * grid_res;

                        // Calculate the downslope direction of the triangular facet
                        if nx == 0f64 {
                            if ny >= 0f64 {
                                hr = 0f64;
                            } else {
                                hr = PI;
                            }
                        } else {
                            if nx >= 0f64 {
                                hr = PI / 2f64 - (ny / nx).atan();
                            } else {
                                hr = 3f64 * PI / 2f64 - (ny / nx).atan();
                            }
                        }

                        // Calculate the slope of the triangular facet
                        hs = -((nz / (nx * nx + ny * ny + nz * nz).sqrt()).acos()).tan();

                        // If the downslope direction is outside the triangular facet, then use the direction of p1 or p2
                        if (hr < i as f64 * quarter_pi) || (hr > (i + 1) as f64 * quarter_pi) {
                            if p1 < p2 {
                                hr = i as f64 * quarter_pi;
                                hs = (z - p1) / (dd[i] * grid_res);
                            } else {
                                hr = ii as f64 * quarter_pi;
                                hs = (z - p2) / (dd[ii] * grid_res);
                            }
                        }

                        r_facet[c] = hr;
                        s_facet[c] = hs;
                    } else {
                        if p1 != nodata && p1 < z {
                            hr = (i as f64) / 4.0 * PI;
                            hs = (z - p1) / (dd[ii] * grid_res);

                            r_facet[c] = hr;
                            s_facet[c] = hs;
                        }
                    }
                }

                // Compute the total area of the triangular facets where water is flowing to
                for c in 0..8 {
                    i = c;
                    ii = (i + 1) % 8;

                    if s_facet[i] > 0f64 {
                        // If the slope is downhill
                        valley[i] = if r_facet[i] > (i as f64 * quarter_pi)
                            && r_facet[i] < ((i + 1) as f64 * quarter_pi)
                        {
                            // If the downslope direction is inside the 45 degrees of the triangular facet
                            s_facet[i]
                        } else if r_facet[i] == r_facet[ii] {
                            // If two adjacent triangular facets have the same downslope direction
                            s_facet[i]
                        } else if (s_facet[ii] == nodata)
                            && (r_facet[i] == ((i + 1) as f64 * quarter_pi))
                        {
                            // If the downslope direction is on the border of the current triangular facet, and the corresponding neigbour's downslope is NoData
                            s_facet[i]
                        } else {
                            ii = (i + 7) % 8;
                            if (s_facet[ii] == nodata) && (r_facet[i] == (i as f64 * quarter_pi)) {
                                // If the downslope direction is on the other border of the current triangular facet, and the corresponding neigbour's downslope is NoData
                                s_facet[i]
                            } else {
                                0f64
                            }
                        }
                    }

                    if exponent != 1f64 {
                        valley[i] = valley[i].powf(exponent);
                    }
                    valley_sum += valley[i];
                    if valley_max < valley[i] {
                        i_max = i;
                        valley_max = valley[i];
                    }
                }

                // Compute the proportional contribution for each of the triangular facets
                if valley_sum > 0f64 {
                    if exponent < 10f64 {
                        for i in 0..8 {
                            // valley[i] = valley[i].powf(exponent) / valley_sum;
                            valley[i] /= valley_sum;
                            weights[i] = 0f64;
                        }
                    } else {
                        for i in 0..8 {
                            if i != i_max {
                                valley[i] = 0f64;
                            } else {
                                valley[i] = 1f64;
                            }
                            weights[i] = 0f64;
                        }
                    }

                    if r_facet[7] == 0f64 {
                        r_facet[7] = 2f64 * PI;
                    }

                    // Compute the contribution to each of the neighbouring grid cells
                    for c in 0..8 {
                        i = c;
                        ii = (i + 1) % 8;

                        if valley[i] > 0f64 {
                            weights[i] +=
                                valley[i] * ((i + 1) as f64 * quarter_pi - r_facet[i]) / quarter_pi;
                            weights[ii] +=
                                valley[i] * (r_facet[i] - i as f64 * quarter_pi) / quarter_pi;
                        }
                    }
                }

                for i in 0..8 {
                    if downslope[i] {
                        row_n = row + yd[i];
                        col_n = col + xd[i];
                        if weights[i] > 0f64 {
                            output.increment(row_n, col_n, fa * weights[i]);
                        }
                        num_inflowing.decrement(row_n, col_n, 1i8);
                        if num_inflowing.get_value(row_n, col_n) == 0i8 {
                            stack.push((row_n, col_n));
                        }
                    }
                }
            } else {
                // find the steepest downslope neighbour and give it all to them
                dir = 0i8;
                max_slope = f64::MIN;
                for i in 0..8 {
                    z_n = input.get_value(row + d_y[i], col + d_x[i]);
                    if z_n != nodata {
                        slope = (z - z_n) / grid_lengths[i];
                        if slope > 0f64 {
                            downslope[i] = true;
                            if slope > max_slope {
                                max_slope = slope;
                                dir = i as i8;
                            }
                        }
                    }
                }
                if max_slope >= 0f64 {
                    weights[dir as usize] = 1.0;
                    total_weights = 1.0;
                }

                for i in 0..8 {
                    if downslope[i] {
                        row_n = row + d_y[i];
                        col_n = col + d_x[i];
                        if total_weights > 0.0 {
                            output.increment(row_n, col_n, fa * (weights[i] / total_weights));
                        }
                        num_inflowing.decrement(row_n, col_n, 1i8);
                        if num_inflowing.get_value(row_n, col_n) == 0i8 {
                            stack.push((row_n, col_n));
                        }
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

        let mut cell_area = cell_size_x * cell_size_y;
        let mut avg_cell_size = (cell_size_x + cell_size_y) / 2.0;
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
