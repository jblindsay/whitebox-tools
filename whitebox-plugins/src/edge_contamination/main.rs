/* 
Authors:  Dr. John Lindsay
Created: 23/07/2021
Last Modified: 23/07/2021
License: MIT
*/

use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_common::structures::Array2D;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;

/// This tool identifs grid cells in a DEM for which the upslope area extends beyond the raster data extent, so-called
/// 'edge-contamined cells'. If a significant number of edge contaminated cells intersect with your area of interest,
/// it is likely that any estimate of upslope area (i.e. flow accumulation) will be under-estimated. 
/// 
/// The user must specify the  name (`--dem`) of the input digital elevation model (DEM) and the 
/// output file (`--output`). The DEM must have been hydrologically corrected to remove all spurious depressions and 
/// flat areas. DEM pre-processing is usually achieved using either the `BreachDepressions` (also `BreachDepressionsLeastCost`) 
/// or `FillDepressions` tool. 
///
/// Additionally, the user must specify the type of flow algorithm used for the analysis (`-flow_type`), which must be 
/// one of 'd8', 'mfd', or 'dinf', based on each of the `D8FlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`
/// methods respectively.
///
/// # See Also
/// `D8FlowAccumulation`, `FD8FlowAccumulation`, `DInfFlowAccumulation`
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

    let exe_name = &format!("edge_contamination{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    edge_contamination Help

    This tool identifs grid cells in a DEM for which the upslope area extends beyond the raster data extent, so-called
    'edge-contamined cells'.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --output       Name of the output raster file..
    --flow_type    Flow algorithm type, one of 'd8', 'mfd', or 'dinf'
    --z_factor     Optional multiplier for when the vertical and horizontal units are not the same.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=edge_cont.tif --flow_type='dinf'
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "edge_contamination v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("EdgeContamination") // This should be camel case and is a reference to the tool name.
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
    let mut d8 = false;
    let mut mfd = false;
    let mut dinf = false;
    let mut z_factor = -1.0;
    
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
        } else if flag_val == "-o" || flag_val == "-output" {
            output_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-flow_type" {
            let flowtype = if keyval {
                vec[1].to_string().to_lowercase()
            } else {
                args[i + 1].to_string().to_lowercase()
            };
            if flowtype.contains("fd8") || flowtype.contains("mfd") {
                d8 = false;
                mfd = true;
                dinf = false;
            } else if flowtype.contains("d8") {
                d8 = true;
                mfd = false;
                dinf = false;
            } else if flowtype.contains("dinf") || flowtype.contains("d-inf") {
                d8 = false;
                mfd = false;
                dinf = true;
            }
            // if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
            //     multidirection = true;
            // }
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

    if !dem_file.contains(&sep) && !dem_file.contains("/") {
        dem_file = format!("{}{}", working_directory, dem_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    /////////////////////////////////////////////////////
    // Read in the DEM and create a D8 pointer from it //
    /////////////////////////////////////////////////////
    let dem = Raster::new(&dem_file, "r")?;
    let rows = dem.configs.rows as isize;
    let columns = dem.configs.columns as isize;
    let num_cells = rows * columns;
    let nodata = dem.configs.nodata;
    let cell_size_x = dem.configs.resolution_x;
    let cell_size_y = dem.configs.resolution_y;
    let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

    if dem.is_in_geographic_coordinates() && z_factor < 0.0 {
        // calculate a new z-conversion factor
        let mut mid_lat = (dem.configs.north - dem.configs.south) / 2.0;
        if mid_lat <= 90.0 && mid_lat >= -90.0 {
            mid_lat = mid_lat.to_radians();
            z_factor = 1.0 / (111320.0 * mid_lat.cos());
        }
    } else if z_factor < 0.0 {
        z_factor = 1.0;
    }

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

    
    // let mut output = Raster::initialize_using_config(&output_file, &dem.configs);
    let mut output = Raster::initialize_using_file(&output_file, &dem);
    output.configs.data_type = DataType::U8;
    output.configs.nodata = 0.0;
    output.reinitialize_values(0.0);


    let mut stack = Vec::with_capacity(num_cells as usize);
    let mut edge_stack = Vec::with_capacity(num_cells as usize);
    let mut visited: Array2D<u8> = Array2D::new(rows, columns, 1, 0)?;
    for row in 0..rows {
        if dem.get_value(row, 0) != nodata {
            edge_stack.push((row, 0));
        } else {
            stack.push((row, 0));
        }
        visited.set_value(row, 0, 2);

        if dem.get_value(row, columns - 1) != nodata {
            edge_stack.push((row, columns - 1));
        } else {
            stack.push((row, columns - 1));
        }
        visited.set_value(row, columns - 1, 2);
    }

    for col in 0..columns {
        if dem.get_value(0, col) != nodata {
            edge_stack.push((0, col));
        } else {
            stack.push((0, col));
        }
        visited.set_value(0, col, 2);

        if dem.get_value(rows - 1, col) != nodata {
            edge_stack.push((rows - 1, col));
        } else {
            stack.push((rows - 1, col));
        }
        visited.set_value(rows - 1, col, 2);
    }

    let dx = [1, 1, 1, 0, -1, -1, -1, 0];
    let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
    let (mut row, mut col): (isize, isize);
    let (mut row_n, mut col_n): (isize, isize);
    let (mut z, mut zn): (f64, f64);
    let mut num_solved_cells = 0;
    while !stack.is_empty() {
        let cell = stack.pop().expect("Error during pop operation.");
        row = cell.0;
        col = cell.1;

        for i in 0..8 {
            row_n = row + dy[i];
            col_n = col + dx[i];
            if visited.get_value(row_n, col_n) == 1 {
                zn = dem.get_value(row_n, col_n);
                if zn != nodata {
                    edge_stack.push((row_n, col_n));
                    visited.set_value(row_n, col_n, 2);
                } else {
                    stack.push((row_n, col_n));
                    visited.set_value(row_n, col_n, 2);
                }
            }
        }

        if configurations.verbose_mode {
            num_solved_cells += 1;
            progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
            if progress != old_progress {
                println!("Processing: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let (mut max_slope, mut slope): (f64, f64);
    let mut dir: i8;

    let grid_res = (cell_size_x + cell_size_y) / 2.0;
    // let mut dir_dinf: f64;
    // let mut af: f64;
    // let mut ac: f64;
    let (mut e1, mut r, mut s1, mut s2, mut s, mut e2): (
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    );
    // let ac_vals = [0f64, 1f64, 1f64, 2f64, 2f64, 3f64, 3f64, 4f64];
    // let af_vals = [1f64, -1f64, 1f64, -1f64, 1f64, -1f64, 1f64, -1f64];
    let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
    let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];
    let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
    let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];
    let atanof1 = 1.0f64.atan();
    // const HALF_PI: f64 = PI / 2f64;
    let (mut a1, mut b1, mut a2, mut b2) = (0isize, 0isize, 0isize, 0isize);

    while !edge_stack.is_empty() {
        let cell = edge_stack.pop().expect("Error during pop operation.");
        row = cell.0;
        col = cell.1;
        output.set_value(row, col, 1.0);
        z = dem.get_value(row, col) * z_factor;
        
        max_slope = if dinf {
            f64::MIN
        } else {
            -1f64
        };
        dir = -1i8;
        // dir_dinf = 360.0;
        for i in 0..8 {

            if dinf {
                e1 = dem.get_value(row + e1_row[i], col + e1_col[i]);
                e2 = dem.get_value(row + e2_row[i], col + e2_col[i]);
                if e1 != nodata && e2 != nodata {
                    e1 *= z_factor;
                    e2 *= z_factor;
                    if z > e1 && z > e2 {
                        s1 = (z - e1) / grid_res;
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
                        if r < 0.0 {
                            s = s1;
                        } else if r > atanof1 {
                            s = (z - e2) / diag_cell_size;
                        }
                        if s >= max_slope { // && s != 0.00001 {
                            max_slope = s;
                            a1 = row + e1_row[i];
                            b1 = col + e1_col[i];
                            a2 = row + e2_row[i];
                            b2 = col + e2_col[i];
                        }
                    } else if z > e1 || z > e2 {
                        s = if z > e1 {
                            (z - e1) / grid_res
                        } else {
                            (z - e2) / diag_cell_size
                        };
                        if s >= max_slope { // && s != 0.00001 {
                            max_slope = s;
                            if z > e1 {
                                a1 = row + e1_row[i];
                                b1 = col + e1_col[i];
                                a2 = -1;
                                b2 = -1;
                            } else {
                                a1 = -1;
                                b1 = -1;
                                a2 = row + e2_row[i];
                                b2 = col + e2_col[i];
                            }    
                        }
                    }
                }
            }
            
            if d8 || mfd {
                row_n = row + dy[i];
                col_n = col + dx[i];
                zn = dem.get_value(row_n, col_n);
                if zn < z && zn != nodata {
                    if d8 {
                        zn *= z_factor;
                        slope = (z - zn) / grid_lengths[i];
                        if slope > max_slope {
                            max_slope = slope;
                            dir = i as i8;
                        }
                    }
                    if visited.get_value(row_n, col_n) == 1 && mfd {
                        edge_stack.push((row_n, col_n));
                        visited.set_value(row_n, col_n, 2);
                    }
                }
            }
        }

        if d8 && dir != -1 {
            row_n = row + dy[dir as usize];
            col_n = col + dx[dir as usize];
            if visited.get_value(row_n, col_n) == 1 {
                edge_stack.push((row_n, col_n));
                visited.set_value(row_n, col_n, 2);
            }
        }

        if dinf && max_slope > 0f64 {
            row_n = a1;
            col_n = b1;
            if visited.get_value(row_n, col_n) == 1 {
                edge_stack.push((row_n, col_n));
                visited.set_value(row_n, col_n, 2);
            }
            row_n = a2;
            col_n = b2;
            if visited.get_value(row_n, col_n) == 1 {
                edge_stack.push((row_n, col_n));
                visited.set_value(row_n, col_n, 2);
            }
        }
        
        if configurations.verbose_mode {
            num_solved_cells += 1;
            progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
            if progress != old_progress {
                println!("Processing: {}%", progress);
                old_progress = progress;
            }
        }
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

    Ok(())
}
