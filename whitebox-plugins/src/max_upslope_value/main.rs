/* 
Authors:  Dr. John Lindsay
Created: 26/02/2022
Last Modified: 26/02/2022
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
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;

/// This tool is used to can be used calculate the maximum upslope value, based on the values within an
/// input values raster (`--values`), along flow-paths, as calculated using the D8 flow method. The user must
/// specify the names of the input digital elevation model (DEM) file (`--dem`), from which the D8 flow 
/// direction data will be calculated internally, and the output file (`--output`).
/// 
/// # See Also
/// `D8FlowAccumulation`, `D8Pointer`
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

    let exe_name = &format!("max_upslope_value{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    max_upslope_value Help

    This tool calculates the maximum upslope value.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --values       Name of the input values raster file.
    --output       Name of the output raster file.
    
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
        "max_upslope_value v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("MaxUpslopeValue") // This should be camel case and is a reference to the tool name.
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
    let mut values_file: String = String::new();
    let mut output_file: String = String::new();
    
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
        } else if flag_val == "-values" {
            values_file = if keyval {
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
    if !values_file.contains(&sep) && !values_file.contains("/") {
        values_file = format!("{}{}", working_directory, values_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    // println!("{dem_file}");
    // println!("{values_file}");
    // println!("{output_file}");
    

    let input = Arc::new(Raster::new(&dem_file, "r")?);

    let rows = input.configs.rows as isize;
    let columns = input.configs.columns as isize;
    let num_cells = rows * columns;
    let nodata = input.configs.nodata;
    let cell_size_x = input.configs.resolution_x;
    let cell_size_y = input.configs.resolution_y;
    let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
    let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -2, -2)?;
    let mut interior_pit_found = false;
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
        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Flow directions: {}%", progress);
                old_progress = progress;
            }
        }
    }

    drop(input);

    let values = Arc::new(Raster::new(&values_file, "r")?);
    
    let mut output = Raster::initialize_using_file(&output_file, &values);
    let out_nodata = -32768f64;
    output.configs.nodata = out_nodata;
    output.configs.photometric_interp = PhotometricInterpretation::Continuous; // if the input is a pointer, this may not be the case by default.
    output.configs.data_type = DataType::F32;
    for row in 0..rows {
        output.set_row_data(row, values.get_row_data(row));
    }
    drop(values);

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
        fa = output.get_value(row, col);
        num_inflowing.decrement(row, col, 1i8);
        dir = flow_dir.get_value(row, col);
        if dir >= 0 {
            row_n = row + dy[dir as usize];
            col_n = col + dx[dir as usize];
            if fa > output.get_value(row_n, col_n) {
                output.set_value(row_n, col_n, fa);
            }
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

    let elapsed_time = get_formatted_elapsed_time(start);
    output.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool",
        tool_name
    ));
    output.add_metadata_entry(format!("Input file: {}", dem_file));
    output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

    if configurations.verbose_mode {
        println!("Saving data...")
    };

    output.write().expect("Error writing output file.");

    let elapsed_time = get_formatted_elapsed_time(start);
    if configurations.verbose_mode {
        println!(
            "{}",
            &format!("Elapsed Time (including I/O): {}", elapsed_time)
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
