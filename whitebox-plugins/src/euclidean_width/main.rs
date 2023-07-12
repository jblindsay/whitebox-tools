use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {},
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

    let exe_name = &format!("euclidean_width{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    euclidean_width Help

    This tool calculates width of the free space between obstacles. The width is a diameter
    of the largest circle which covers the pixel and is inscribed in a set of obstacles.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input   Name of the input raster file.
    -o, --output  Name of the output raster file.
    --max_width   Maximum width for high-precision calculation (optional).

    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME -r=EuclideanWidth -i=input.tif -o=width.tif --max_width=1000.0

    "#
        .replace("*", &sep)
        .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "exposure_towards_wind_flux v{} by Dr. Timofey E. Samsonov and Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("EuclideanWidth")
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
    let max_procs = configurations.max_procs as isize;

    let mut input_file: String = String::new();
    let mut output_file: String = String::new();
    let mut max_width = f32::INFINITY;

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
        if flag_val == "-i" || flag_val == "-input" {
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
        } else if flag_val == "-max_width" {
            max_width = if keyval {
                vec[1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val))
            } else {
                args[i + 1]
                    .to_string()
                    .parse::<f32>()
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

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if max_width < 0.0 {
        if configurations.verbose_mode {
            wrapped_print("Warning: Maximum width should be non-negative", 50);
        }
        max_width = 0.0;
    }

    if verbose {
        println!("Reading data...")
    };

    let input = Raster::new(&input_file, "r")?;

    let nodata = input.configs.nodata;
    let rows = input.configs.rows as isize;
    let columns = input.configs.columns as isize;

    let start = Instant::now();

    let mut rx: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
    let mut ry: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

    let mut output = Raster::initialize_using_file(&output_file, &input);
    output.configs.data_type = DataType::F32;

    let mut h: f64;
    let mut which_cell: usize;
    let inf_val = f64::INFINITY;
    let dx = [-1, -1, 0, 1, 1, 1, 0, -1];
    let dy = [0, -1, -1, -1, 0, 1, 1, 1];
    let gx = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
    let gy = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
    let (mut x, mut y): (isize, isize);
    let (mut z, mut z2, mut z_min): (f64, f64, f64);

    for row in 0..rows {
        for col in 0..columns {
            z = input.get_value(row, col);
            if z != 0.0 && z != nodata {
                output.set_value(row, col, 0.0);
            } else {
                output.set_value(row, col, inf_val);
            }
        }
        if verbose {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Initializing Rasters: {}%", progress);
                old_progress = progress;
            }
        }
    }

    for row in 0..rows {
        for col in 0..columns {
            z = output.get_value(row, col);
            if z != 0.0 {
                z_min = inf_val;
                which_cell = 0;
                for i in 0..4 {
                    x = col + dx[i];
                    y = row + dy[i];
                    z2 = output.get_value(y, x);
                    if z2 != nodata {
                        h = match i {
                            0 => 2.0 * rx.get_value(y, x) + 1.0,
                            1 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                            2 => 2.0 * ry.get_value(y, x) + 1.0,
                            _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 3
                        };
                        z2 += h;
                        if z2 < z_min {
                            z_min = z2;
                            which_cell = i;
                        }
                    }
                }
                if z_min < z {
                    output.set_value(row, col, z_min);
                    x = col + dx[which_cell];
                    y = row + dy[which_cell];
                    rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                    ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                }
            }
        }
        if verbose {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (1 of 3): {}%", progress);
                old_progress = progress;
            }
        }
    }

    for row in (0..rows).rev() {
        for col in (0..columns).rev() {
            z = output.get_value(row, col);
            if z != 0.0 {
                z_min = inf_val;
                which_cell = 0;
                for i in 4..8 {
                    x = col + dx[i];
                    y = row + dy[i];
                    z2 = output.get_value(y, x);
                    if z2 != nodata {
                        h = match i {
                            5 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                            4 => 2.0 * rx.get_value(y, x) + 1.0,
                            6 => 2.0 * ry.get_value(y, x) + 1.0,
                            _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 7
                        };
                        z2 += h;
                        if z2 < z_min {
                            z_min = z2;
                            which_cell = i;
                        }
                    }
                }
                if z_min < z {
                    output[(row, col)] = z_min;
                    x = col + dx[which_cell];
                    y = row + dy[which_cell];
                    rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                    ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                }
            }
        }
        if verbose {
            progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (2 of 3): {}%", progress);
                old_progress = progress;
            }
        }
    }

    let cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
    for row in 0..rows {
        for col in 0..columns {
            if input.get_value(row, col) != nodata {
                output.set_value(row, col, output.get_value(row, col).sqrt() * cell_size);
            } else {
                output.set_value(row, col, nodata);
            }
        }
        if verbose {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (3 of 3): {}%", progress);
                old_progress = progress;
            }
        }
    }

    let elapsed_time = get_formatted_elapsed_time(start);
    output.configs.palette = "grey.plt".to_string();
    output.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool",
        tool_name
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

    Ok(())

}