/* 
Authors:  Dr. John Lindsay
Created: 15/07/2021
Last Modified: 15/07/2021
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use num_cpus;
use whitebox_common::structures::{Array2D};
use whitebox_common::utils::{ get_formatted_elapsed_time, wrapped_print };
use whitebox_raster::*;

/// This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Qin et al. (2007) 
/// flow algorithm, not to be confused with the similarly named `QuinnFlowAccumulation` tool. This algorithm is an 
/// examples of a multiple-flow-direction (MFD) method because the flow entering each grid cell is routed to more 
/// than one downslope neighbour, i.e. flow *divergence* is permitted. It is based on a modification of the Freeman
/// (1991; `FD8FlowAccumulation`) and Quinn et al. (1995; `QuinnFlowAccumulation`) methods. The Qin method relates 
/// the degree of flow dispersion from a grid cell to the local maximum downslope gradient. Specifically, steeper 
/// terrain experiences more convergent flow while flatter slopes experience more flow divergence. 
///
/// The following equations are used to calculate the portion flow (*F<sub>i</sub>*)
/// given to each neighbour, *i*:
///
/// > *F<sub>i</sub>* = *L<sub>i</sub>*(tan&beta;)<sup>*f(e)*</sup> / &Sigma;<sub>*i*=1</sub><sup>*n*</sup>[*L<sub>i</sub>*(tan&beta;)<sup>*f(e)*</sup>]
/// >
/// > *f(e)* = min(*e*, *e<sub>U</sub>*) / *e<sub>U</sub>* &times; (*p<sub>U</sub>* - 1.1) + 1.1
///
/// Where *L<sub>i</sub>* is the contour length, and is 0.5&times;cell size for cardinal directions and 0.354&times;cell size for
/// diagonal directions, *n* = 8, and represents each of the eight neighbouring grid cells. The exponent *f(e)* controls
/// the proportion of flow allocated to each downslope neighbour of a grid cell, based on the local maximum downslope
/// gradient (*e*), and the user-specified upper boundary of *e* (*e<sub>U</sub>*; `--max_slope`), and the upper 
/// boundary of the exponent (*p<sub>U</sub>*; `--exponent`), *f(e)*. Note that the original Qin (2007)
/// implementation allowed for user-specified lower boundaries on the slope (*e<sub>L</sub>*) and exponent (*p<sub>L</sub>*) 
/// parameters as well. In this implementation, these parameters are assumed to be 0.0 and 1.1 respectively, and are
/// not user adjustable. Also note, the `--exponent` parameter should be less than 50.0, as higher values may cause
/// numerical instability.
///
/// The user must specify the  name (`--dem`) of the input digital elevation model (DEM) and the output file (`--output`). 
/// The DEM must have been hydrologically corrected to remove all spurious depressions and flat areas. DEM 
/// pre-processing is usually achieved using either the `BreachDepressions` (also `BreachDepressionsLeastCost`) or 
/// `FillDepressions` tool. 
/// 
/// The user-specified non-dispersive, channel initiation *threshold* (`--threshold`) is a flow-accumulation 
/// value (measured in upslope grid cells, which is directly proportional to area) above which flow dispersion is 
/// no longer permitted. Grid cells with flow-accumulation values above this area threshold will have their flow
/// routed in a manner that is similar to the D8 single-flow-direction algorithm, directing all flow towards the
/// steepest downslope neighbour. This is usually done under the assumption that flow dispersion, whilst appropriate
/// on hillslope areas, is not realistic once flow becomes channelized. Importantly, the `--threshold` parameter sets 
/// the spatial extent of the stream network, with lower values resulting in more extensive networks. 
/// 
/// In addition to the input DEM, output file (`--output`), and exponent, the user must also specify the output type (`--out_type`). The output flow-accumulation
/// can be: 1) `cells` (i.e. the number of inflowing grid cells), `catchment area` (i.e. the upslope area),
/// or `specific contributing area` (i.e. the catchment area divided by the flow width). The default value
/// is `specific contributing area`. The user must also specify whether the output flow-accumulation grid should be
/// log-tranformed (`--log`), i.e. the output, if this option is selected, will be the natural-logarithm of the
/// accumulated flow value. This is a transformation that is often performed to better visualize the
/// contributing area distribution. Because contributing areas tend to be very high along valley bottoms
/// and relatively low on hillslopes, when a flow-accumulation image is displayed, the distribution of
/// values on hillslopes tends to be 'washed out' because the palette is stretched out to represent the
/// highest values. Log-transformation provides a means of compensating for this phenomenon. Importantly,
/// however, log-transformed flow-accumulation grids must not be used to estimate other secondary terrain
/// indices, such as the wetness index (`WetnessIndex`), or relative stream power index (`StreamPowerIndex`).
///
/// # Reference
/// Freeman, T. G. (1991). Calculating catchment area with divergent flow based on a regular grid. Computers and
/// Geosciences, 17(3), 413-422.
///
/// Qin, C., Zhu, A. X., Pei, T., Li, B., Zhou, C., & Yang, L. 2007. An adaptive approach to selecting a 
/// flow‐partition exponent for a multiple‐flow‐direction algorithm. *International Journal of Geographical 
/// Information Science*, 21(4), 443-458.
///
/// Quinn, P. F., K. J. Beven, Lamb, R. 1995. The in (a/tanβ) index: How to calculate it and how to use it within 
/// the topmodel framework. *Hydrological Processes* 9(2): 161-182.
/// 
/// # See Also
/// `D8FlowAccumulation`, `QuinnFlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `Rho8Pointer`, `WetnessIndex`
fn main() {
    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}
            Err(e) => panic!("{:?}", e),
        }
    }

    if args.len() <= 1 || args[1].trim() == "help" {
        // print help
        help();
    }

    if args[1].trim() == "version" {
        // print version information
        version();
    }
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("qin_flow_accumulation{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    qin_flow_accumulation Help

    This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Qin et al. (2007) 
    flow algorithm.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --output       Name of the output raster file.
    --out_type     Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'.
    --exponent     Optional upper-bound exponent parameter; default is 10.0.
    --max_slope    Optional upper-bound slope parameter, in degrees (0-90); default is 45.0.
    --threshold    Optional convergence threshold parameter, in grid cells; default is infinity.
    --log          Log-transform the output values?
    --clip         Optional flag to request clipping the display max by 1%.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=QMFD.tif --out_type='specific contributing area' --exponent=15.0 --max_slope=30.0 --threshold=10000
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "qin_flow_accumulation v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("QinFlowAccumulation") // This should be camel case and is a reference to the tool name.
}

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    let tool_name = get_tool_name();

    let sep: String = path::MAIN_SEPARATOR.to_string();

    // Read in the environment variables and get the necessary values
    let configurations = whitebox_common::configs::get_configs()?;
    let mut working_directory = configurations.working_directory.clone();
    if !working_directory.is_empty() && !working_directory.ends_with(&sep) {
        working_directory += &sep;
    }

    // read the arguments
    let mut dem_file = String::new();
    let mut output_file: String = String::new();
    let mut out_type = String::from("sca");
    let mut convergence_threshold = 0f64;
    let mut upper_bound_exponent = 10f64;
    let lower_bound_exponent = 1.1f64;
    let mut upper_slope = 45f64; // Input in degrees, but gets converted to tan-slope later
    // let lower_slope = 0f64;
    let mut z_factor = 1f64;
    let mut log_transform = false;
    let mut clip_max = false;

    if args.len() <= 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Tool run with too few parameters.",
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
        if flag_val == "-d" || flag_val == "-dem" {
            dem_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-output" {
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
            upper_bound_exponent = if keyval {
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
        } else if flag_val == "-max_slope" {
            upper_slope = if keyval {
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
        } else if flag_val == "-log" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                log_transform = true;
            }
        } else if flag_val == "-clip" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                clip_max = true;
            }
        } else if flag_val == "-zfactor" {
            z_factor = if keyval {
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
        }
    }

    if configurations.verbose_mode {
        let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
        // 28 = length of the 'Powered by' by statement.
        println!("{}", "*".repeat(welcome_len));
        println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
        println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
        println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
        println!("{}", "*".repeat(welcome_len));
    }

    let mut progress: usize;
    let mut old_progress: usize = 1;

    let start = Instant::now();

    if upper_slope > 90.0 { upper_slope = 90.0; }
    upper_slope = upper_slope.to_radians().tan();

    if upper_bound_exponent < lower_bound_exponent {
        if configurations.verbose_mode {
            wrapped_print("Warning: The upper-bound exponent parameter (--exponent) must be greater than 1.1.", 50);
        }
        upper_bound_exponent = lower_bound_exponent; 
    }

    if upper_bound_exponent >= 50.0 {
        if configurations.verbose_mode {
            wrapped_print("Warning: The upper-bound exponent parameter (--exponent) must be less than 50.0.", 50);
        }
        upper_bound_exponent = 50.0; 
    }

    if !dem_file.contains(&sep) && !dem_file.contains("/") {
        dem_file = format!("{}{}", working_directory, dem_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if convergence_threshold <= 0f64 {
        convergence_threshold = f64::MAX;
    }

    /////////////////////////////////////////////////////
    // Read in the DEM and create a D8 pointer from it //
    /////////////////////////////////////////////////////
    let dem = Arc::new(Raster::new(&dem_file, "r")?);
    let header = dem.configs.clone();
    let rows = header.rows as isize;
    let columns = header.columns as isize;
    let num_cells = rows * columns;
    let dem_nodata = header.nodata;
    let cell_size_x = header.resolution_x;
    let cell_size_y = header.resolution_y;
    let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

    if dem.is_in_geographic_coordinates() && z_factor < 0.0 {
        // calculate a new z-conversion factor
        let mut mid_lat = (header.north - header.south) / 2.0;
        if mid_lat <= 90.0 && mid_lat >= -90.0 {
            mid_lat = mid_lat.to_radians();
            z_factor = 1.0 / (111320.0 * mid_lat.cos());
        }
    } else if z_factor < 0.0 {
        z_factor = 1.0;
    }

    let mut num_procs = num_cpus::get() as isize;
    let max_procs = configurations.max_procs;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    // calculate the number of inflowing cells
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let dem = dem.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let mut z: f64;
            let mut zn: f64;
            let mut count: i8;
            let mut interior_pit_found = false;
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data: Vec<i8> = vec![-1i8; columns as usize];
                for col in 0..columns {
                    z = dem.get_value(row, col);
                    if z != dem_nodata {
                        count = 0i8;
                        for i in 0..8 {
                            zn = dem.get_value(row + dy[i], col + dx[i]);
                            if zn > z && zn != dem_nodata {
                                count += 1;
                            }
                        }
                        data[col as usize] = count;
                        if count == 8 {
                            interior_pit_found = true;
                        }
                    }
                }
                tx.send((row, data, interior_pit_found))
                    .expect("Error sending data to thread.");
            }
        });
    }

    let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
    let mut stack = Vec::with_capacity(num_cells as usize);
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

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Num. inflowing neighbours: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let mut output = Raster::initialize_using_config(&output_file, &header);
    output.configs.data_type = DataType::F32;
    output.reinitialize_values(1f64);
    let dx = [1, 1, 1, 0, -1, -1, -1, 0];
    let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
    let (mut row, mut col): (isize, isize);
    let (mut row_n, mut col_n): (isize, isize);
    let (mut z, mut zn): (f64, f64);
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

    let contour_lengths = [
        0.354f64 * cell_size_x,
        0.5f64 * cell_size_x,
        0.354f64 * cell_size_x,
        0.5f64 * cell_size_x,
        0.354f64 * cell_size_x,
        0.5f64 * cell_size_x,
        0.354f64 * cell_size_x,
        0.5f64 * cell_size_x,
    ];
    let (mut max_slope, mut slope): (f64, f64);
    let mut dir: i8;
    let mut total_weights: f64;
    let mut f: f64;
    let mut is_converged: bool;
    let mut downslope: [bool; 8];
    let mut weights: [f64; 8];
    while !stack.is_empty() {
        let cell = stack.pop().expect("Error during pop operation.");
        row = cell.0;
        col = cell.1;
        z = dem.get_value(row, col) * z_factor;
        fa = output.get_value(row, col);
        num_inflowing.set_value(row, col, -1i8);

        max_slope = -1f64;
        downslope = [false; 8];
        dir = 0i8;
        for i in 0..8 {
            row_n = row + dy[i];
            col_n = col + dx[i];
            zn = dem.get_value(row_n, col_n);
            if zn < z && zn != dem_nodata {
                zn *= z_factor;
                slope = (z - zn) / grid_lengths[i];
                if slope > max_slope {
                    max_slope = slope;
                    dir = i as i8;
                }
                downslope[i] = true;
            }
        }

        weights = [0.0; 8];
        total_weights = 0.0;
        is_converged = fa >= convergence_threshold;
        if !is_converged {
            if max_slope > 0f64 {
                f = if upper_slope > 0f64 {
                    max_slope.min(upper_slope) / upper_slope * (upper_bound_exponent - lower_bound_exponent) + lower_bound_exponent
                } else {
                    upper_bound_exponent
                };
                for i in 0..8 {
                    row_n = row + dy[i];
                    col_n = col + dx[i];
                    zn = dem.get_value(row_n, col_n);
                    if zn < z && zn != dem_nodata {
                        zn *= z_factor;
                        slope = (z - zn) / grid_lengths[i];
                        weights[i] = contour_lengths[i] * slope.powf(f);
                        total_weights += weights[i];
                        downslope[i] = true;
                    }
                }
            }
        } else {
            if max_slope >= 0f64 {
                weights[dir as usize] = 1.0;
                total_weights = 1.0;
            }
        }

        if total_weights > 0.0 {
            for i in 0..8 {
                if downslope[i] {
                    row_n = row + dy[i];
                    col_n = col + dx[i];
                    output.increment(row_n, col_n, fa * (weights[i] / total_weights));
                    num_inflowing.decrement(row_n, col_n, 1i8);
                    if num_inflowing.get_value(row_n, col_n) == 0i8 {
                        stack.push((row_n, col_n));
                    }
                }
            }
        }

        if configurations.verbose_mode {
            num_solved_cells += 1;
            progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
            if progress != old_progress {
                println!("Flow accumulation: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let mut cell_area = cell_size_x * cell_size_y;
    let mut contour_length = (cell_size_x + cell_size_y) / 2.0;
    if out_type == "cells" {
        cell_area = 1.0;
        contour_length = 1.0;
    } else if out_type == "ca" {
        contour_length = 1.0;
    }

    if log_transform {
        for row in 0..rows {
            for col in 0..columns {
                z = dem.get_value(row, col);
                if z == dem_nodata {
                    output.set_value(row, col, dem_nodata);
                } else {
                    if out_type == "sca" {
                        contour_length = 0.0;
                        for i in 0..8 {
                            zn = dem.get_value(row + dy[i], col + dx[i]);
                            if zn < z && zn != dem_nodata {
                                contour_length += contour_lengths[i];
                            }
                        }
                    }
                    fa = output.get_value(row, col);
                    if contour_length > 0.0 {
                        output.set_value(row, col, (fa * cell_area / contour_length).ln());
                    } else {
                        output.set_value(row, col, (fa * cell_area / cell_size_x).ln());
                    }
                }
            }

            if configurations.verbose_mode {
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
                z = dem.get_value(row, col);
                if z == dem_nodata {
                    output.set_value(row, col, dem_nodata);
                } else {
                    if out_type == "sca" {
                        contour_length = 0.0;
                        for i in 0..8 {
                            zn = dem.get_value(row + dy[i], col + dx[i]);
                            if zn < z && zn != dem_nodata {
                                contour_length += contour_lengths[i];
                            }
                        }
                    }
                    fa = output.get_value(row, col);
                    if contour_length > 0.0 {
                        output.set_value(row, col, fa * cell_area / contour_length);
                    } else {
                        output.set_value(row, col, fa * cell_area / cell_size_x);
                    }
                }
            }

            if configurations.verbose_mode {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Correcting values: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    if clip_max {
        output.clip_display_max(1.0);
    }

    if configurations.verbose_mode {
        println!("Saving data...")
    };

    let _ = match output.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output file written")
            }
        }
        Err(e) => return Err(e),
    };

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "\n{}",
            &format!("Elapsed Time (Including I/O): {}", elapsed_time)
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
