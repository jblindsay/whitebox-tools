/* 
Authors:  Dr. John Lindsay
Created: 19/01/2022
Last Modified: 19/01/2022
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_lidar::*;
use whitebox_common::utils::get_formatted_elapsed_time;

/// This tool can be used to shift the x,y,z coordinates of points within a LiDAR file. The user must specify 
/// the name of the input file (`--input`) and the output file (`--output`). Additionally, the user must specify
/// the x,y,z shift values (`x_shift`, `y_shift`, `z_shift`). At least one non-zero shift value is needed
/// to run the tool.
/// 
/// # See Also
/// `LidarElevationSlice`, `HeightAboveGround`
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

    let exe_name = &format!("lidar_shift{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    lidar_shift Help

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
        "lidar_shift v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("LidarShift") // This should be camel case and is a reference to the tool name.
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
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut x_shift = 0f64;
    let mut y_shift = 0f64;
    let mut z_shift = 0f64;

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
        } else if flag_val == "-x_shift" || flag_val == "-x" {
            x_shift = if keyval {
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
        } else if flag_val == "-y_shift" || flag_val == "-y" {
            y_shift = if keyval {
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
        } else if flag_val == "-z_shift" || flag_val == "-z" {
            z_shift = if keyval {
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

    let start = Instant::now();

    if configurations.verbose_mode {
        let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
        // 28 = length of the 'Powered by' by statement.
        println!("{}", "*".repeat(welcome_len));
        println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
        println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
        println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
        println!("{}", "*".repeat(welcome_len));
    }

    if x_shift == 0f64 && y_shift == 0f64 && z_shift == 0f64 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "At least one non-zero shift parameter must be specified.",
        ));
    }

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    let mut progress: usize;
    let mut old_progress: usize = 1;

    let input = LasFile::new(&input_file, "r")?;
    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let mut output = LasFile::initialize_using_file(&output_file, &input);
    let mut pr: LidarPointRecord;
    let x_shift_transformed = ((x_shift - input.header.x_offset) / input.header.x_scale_factor) as i32;
    let y_shift_transformed = ((y_shift - input.header.y_offset) / input.header.y_scale_factor) as i32;
    let z_shift_transformed = ((z_shift - input.header.z_offset) / input.header.z_scale_factor) as i32;
    for p in 0..n_points {
        pr = input.get_record(p);
        let pr2: LidarPointRecord;
        match pr {
            LidarPointRecord::PointRecord0 { mut point_data } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord0 {
                    point_data: point_data,
                };
            }
            LidarPointRecord::PointRecord1 {
                mut point_data,
                gps_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord1 {
                    point_data: point_data,
                    gps_data: gps_data,
                };
            }
            LidarPointRecord::PointRecord2 {
                mut point_data,
                colour_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord2 {
                    point_data: point_data,
                    colour_data: colour_data,
                };
            }
            LidarPointRecord::PointRecord3 {
                mut point_data,
                gps_data,
                colour_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord3 {
                    point_data: point_data,
                    gps_data: gps_data,
                    colour_data: colour_data,
                };
            }
            LidarPointRecord::PointRecord4 {
                mut point_data,
                gps_data,
                wave_packet,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord4 {
                    point_data: point_data,
                    gps_data: gps_data,
                    wave_packet: wave_packet,
                };
            }
            LidarPointRecord::PointRecord5 {
                mut point_data,
                gps_data,
                colour_data,
                wave_packet,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord5 {
                    point_data: point_data,
                    gps_data: gps_data,
                    colour_data: colour_data,
                    wave_packet: wave_packet,
                };
            }
            LidarPointRecord::PointRecord6 {
                mut point_data,
                gps_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord6 {
                    point_data: point_data,
                    gps_data: gps_data,
                };
            }
            LidarPointRecord::PointRecord7 {
                mut point_data,
                gps_data,
                colour_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord7 {
                    point_data: point_data,
                    gps_data: gps_data,
                    colour_data: colour_data,
                };
            }
            LidarPointRecord::PointRecord8 {
                mut point_data,
                gps_data,
                colour_data,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord8 {
                    point_data: point_data,
                    gps_data: gps_data,
                    colour_data: colour_data,
                };
            }
            LidarPointRecord::PointRecord9 {
                mut point_data,
                gps_data,
                wave_packet,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord9 {
                    point_data: point_data,
                    gps_data: gps_data,
                    wave_packet: wave_packet,
                };
            }
            LidarPointRecord::PointRecord10 {
                mut point_data,
                gps_data,
                colour_data,
                wave_packet,
            } => {
                point_data.x += x_shift_transformed;
                point_data.y += y_shift_transformed;
                point_data.z += z_shift_transformed;
                pr2 = LidarPointRecord::PointRecord10 {
                    point_data: point_data,
                    gps_data: gps_data,
                    colour_data: colour_data,
                    wave_packet: wave_packet,
                };
            }
        }
        output.add_point_record(pr2);

        if configurations.verbose_mode {
            progress = (100.0_f64 * p as f64 / num_points) as usize;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // output.header.x_offset += x_shift;
    // output.header.y_offset += y_shift;
    // output.header.z_offset += z_shift;

    println!("");
    if configurations.verbose_mode {
        println!("Writing output LAS file...");
    }
    let _ = match output.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Complete!")
            }
        }
        Err(e) => println!("error while writing: {:?}", e),
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
