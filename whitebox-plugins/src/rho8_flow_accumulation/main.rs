/* 
Authors:  Dr. John Lindsay
Created: 25/08/2021
Last Modified: 29/08/2021
License: MIT
*/

extern crate rand;

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
use rand::Rng;
use whitebox_common::structures::{Array2D};
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;

/// This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Fairfield and Leymarie (1991) 
/// flow algorithm, often called Rho8. Like the D8 flow method, this algorithm is an examples of a single-flow-direction (SFD) method because the flow entering each
/// grid cell is routed to only one downslope neighbour, i.e. flow *divergence* is not permitted. The user must specify the
/// name of the input file (`--input`), which may be either a digital elevation model (DEM) or a Rho8 pointer file (see `Rho8Pointer`). If a DEM is input, it must have been hydrologically
/// corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using
/// either the `BreachDepressions` (also `BreachDepressionsLeastCost`) or `FillDepressions` tool. 
/// 
/// In addition to the input and output (`--output`)files, the user must also specify the output type (`--out_type`). The output flow-accumulation
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
/// If a Rho8 pointer is used as the input raster, the user must specify this (`--pntr`). Similarly, 
/// if a pointer input is used and the pointer follows the Esri pointer convention, rather than the 
/// default WhiteboxTools convension for pointer files, then this must also be specified (`--esri_pntr`).
///
/// # Reference
/// Fairfield, J., and Leymarie, P. 1991. Drainage networks from grid digital elevation models. *Water
/// Resources Research*, 27(5), 709-717.
///
/// # See Also
/// `Rho8Pointer`, `D8FlowAccumulation`, `QinFlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`, `MDInfFlowAccumulation`, `WetnessIndex`
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

    let exe_name = &format!("rho8_flow_accumulation{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    rho8_flow_accumulation Help

    This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Fairfield and Leymarie (1991) 
    flow algorithm, sometimes called Rho8.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --output       Name of the output raster file.
    --out_type     Output type; one of 'cells', 'specific contributing area' (default), and 'catchment area'.
    --log          Log-transform the output values?
    --clip         Optional flag to request clipping the display max by 1%.
    --pntr         Is the input raster a Rho8 flow pointer rather than a DEM?
    --esri_pntr    Does the input Rho8 pointer use the ESRI style scheme?
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=Rho8.tif --out_type='specific contributing area'
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "rho8_flow_accumulation v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("Rho8FlowAccumulation") // This should be camel case and is a reference to the tool name.
}

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    let sep: String = path::MAIN_SEPARATOR.to_string();

    // Read in the environment variables and get the necessary values
    let configurations = whitebox_common::configs::get_configs()?;
    let mut working_directory = configurations.working_directory.clone();
    if !working_directory.is_empty() && !working_directory.ends_with(&sep) {
        working_directory += &sep;
    }

    // read the arguments
    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut out_type = String::from("sca");
    let mut log_transform = false;
    let mut clip_max = false;
    let mut pntr_input = false;
    let mut esri_style = false;

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
        }
    }

    if configurations.verbose_mode {
        let tool_name = get_tool_name();
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

    if configurations.verbose_mode {
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
    // let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
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
       let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                // let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let (mut z, mut z_n, mut slope): (f64, f64, f64);
                let mut rng = rand::thread_rng();
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![-2i8; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            let mut dir = 0;
                            let mut max_slope = f64::MIN;
                            for i in 0..8 {
                                z_n = input[(row + d_y[i], col + d_x[i])];
                                if z_n != nodata {
                                    slope = match i {
                                        1 | 3 | 5 | 7 => z - z_n,
                                        _ => (z - z_n) / (2f64 - rng.gen_range(0f64, 1f64)), //between.ind_sample(&mut rng)),
                                    };
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i as i8;
                                    }
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1i8;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            flow_dir.set_row_data(data.0, data.1);

            if configurations.verbose_mode {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        // The input raster is a Rho8 flow pointer
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
            if configurations.verbose_mode {
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

        if configurations.verbose_mode {
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
        dir = flow_dir[(row, col)];
        if dir >= 0 {
            row_n = row + dy[dir as usize];
            col_n = col + dx[dir as usize];
            output.increment(row_n, col_n, fa);
            num_inflowing.decrement(row_n, col_n, 1i8);
            if num_inflowing.get_value(row_n, col_n) == 0i8 {
                stack.push((row_n, col_n));
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
                    let dir = flow_dir[(row, col)];
                    if dir >= 0 {
                        output[(row, col)] =
                            (output[(row, col)] * cell_area / flow_widths[dir as usize]).ln();
                    } else {
                        output[(row, col)] =
                            (output[(row, col)] * cell_area / flow_widths[3]).ln();
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
                // if input.get_value(row, col) == nodata {
                if flow_dir.get_value(row, col) == -2 {
                    output.set_value(row, col, out_nodata);
                } else {
                    let dir = flow_dir.get_value(row, col);
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

            if configurations.verbose_mode {
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
        get_tool_name()
    ));
    output.add_metadata_entry(format!("Input file: {}", input_file));
    output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
    if configurations.verbose_mode {
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
