/* 
Authors:  Dr. John Lindsay
Created: 05/03/2023
Last Modified: 05/03/2023
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_lidar::*;
use whitebox_raster::*;
use whitebox_common::structures::Point3D;
use whitebox_common::utils::get_formatted_elapsed_time;
use num_cpus;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to normalize a LiDAR point cloud. A normalized point cloud is one for which the point z-values
/// represent height above the ground surface rather than raw elevation values. Thus, a point that falls on the ground 
/// surface will have a z-value of zero and vegetation points, and points associated with other off-terrain objects, 
/// have positive, non-zero z-values. Point cloud normalization is an essential pre-processing method for many forms of
/// LiDAR data analysis, including the characterization of many forestry related metrics and individual tree mapping 
/// (`IndividualTreeDetection`). 
/// 
/// This tool works by measuring the elevation difference of each point in an input LiDAR file (`--input`) and the elevation
/// of an input raster digital terrain model (`--dtm`). A DTM is a bare-earth digital elevation model. Typically, the input
/// DTM is creating using the same input LiDAR data by interpolating the ground surface using only ground-classified points.
/// If the LiDAR point cloud does not contain ground-point classifications, you may wish to apply the `LidarGroundPointFilter` 
/// or `ClassifyLidar`tools before interpolating the DTM. While ground-point classification works well to identify the ground 
/// surface beneath vegetation cover, building points are sometimes left  It may also be necessary to remove other off-terrain 
/// objects like buildings. The `RemoveOffTerrainObjects` tool can be useful for this purpose, creating a final bare-earth DTM.
/// This tool outputs a normalized LiDAR point cloud (`--output`). If the `--no_negatives` parameter is true, any points that fall
/// beneath the surface elevation defined by the DTM, will have their z-value set to zero.
/// 
/// Note that the `LidarTophatTransform` tool similarly can be used to produce a type of normalized point cloud, although it 
/// does not require an input raster DTM. Rather, it attempts to model the ground surface within the point cloud by identifying
/// the lowest points within local neighbourhoods surrounding each point in the cloud. While this approach can produce satisfactory
/// results in some cases, the `NormalizeLidar` tool likely works better under more rugged topography and in areas with 
/// extensive building coverage, and provides greater control over the definition of the ground surface.
/// 
/// ![](../../doc_img/NormalizeLidar.png)
/// 
/// # See Also
/// `LidarTophatTransform`, `IndividualTreeDetection`, `LidarGroundPointFilter`, `ClassifyLidar`
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

    let exe_name = &format!("NormalizeLidar{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    normalize_lidar Help

    This tool normalizes LiDAR point clouds.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input     Name of the input LiDAR file.
    -o, --output    Name of the output vector points file.
    --dtm           Name of the input digital terrain model (DTM) file.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=points.laz -o=normalized.laz --dtm=dtm.tif
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "NormalizeLidar v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("NormalizeLidar") // This should be camel case and is a reference to the tool name.
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

    let mut num_procs = num_cpus::get() as isize;
    if configurations.max_procs > 0 && configurations.max_procs < num_procs {
        num_procs = configurations.max_procs;
    }

    // read the arguments
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut dtm_file: String = "".to_string();
    let mut no_negatives: bool = false;

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
        } else if flag_val == "-dtm" {
            dtm_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-no_negatives" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                no_negatives = true;
            }
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
    
    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }
    if !dtm_file.contains(&sep) && !dtm_file.contains("/") {
        dtm_file = format!("{}{}", working_directory, dtm_file);
    }

    let input = Arc::new(LasFile::new(&input_file, "r")?);
    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let dtm = Arc::new(Raster::new(&dtm_file, "r")?);
    let nodata = dtm.configs.nodata;
    
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs as usize {
        let input = input.clone();
        let dtm = dtm.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut p: Point3D;
            let mut pd: PointData;
            let mut row: isize;
            let mut col: isize;
            let mut z: f64;
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            for point_num in (0..n_points).filter(|t| t % num_procs as usize == tid) {
                pd = input[point_num];
                if !pd.withheld() && !pd.is_classified_noise() {
                    p = input.get_transformed_coords(point_num);
                    row = dtm.get_row_from_y(p.y);
                    col = dtm.get_column_from_x(p.x);
                    z = dtm.get_value(row, col);
                    if z != nodata {
                        z = if p.z < z && no_negatives {
                            0.0
                        } else {
                            p.z - z
                        };
                        tx.send((point_num, z)).unwrap();
                    } else {
                        // are any of the neighbouring cells valid?
                        z = p.z;
                        for n in 0..8 {
                            if dtm.get_value(row + dy[n], col + dx[n]) != nodata {
                                z = dtm.get_value(row + dy[n], col + dx[n]);
                                break;
                            }
                        }
                        z = if p.z < z && no_negatives {
                            0.0
                        } else {
                            p.z - z
                        };
                        tx.send((point_num, z)).unwrap();
                    }
                } else {
                    tx.send((point_num, 0f64)).unwrap();
                }
            }
        });
    }

    let mut residuals = vec![0f64; n_points];
    let mut progress: i32;
    let mut old_progress: i32 = -1;
    for i in 0..n_points {
        let (point_num, z) = rx.recv().expect("Error receiving data from thread.");
        residuals[point_num] = z;
        if configurations.verbose_mode {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    drop(dtm);

    // now output the data
    let mut output = LasFile::initialize_using_file(&output_file, &input);
    let mut pr2: LidarPointRecord;
    for point_num in 0..n_points {
        let pr = input.get_record(point_num);
        match pr {
            LidarPointRecord::PointRecord0 { mut point_data } => {
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
                pr2 = LidarPointRecord::PointRecord0 {
                    point_data: point_data,
                };
            }
            LidarPointRecord::PointRecord1 {
                mut point_data,
                gps_data,
            } => {
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
                pr2 = LidarPointRecord::PointRecord1 {
                    point_data: point_data,
                    gps_data: gps_data,
                };
            }
            LidarPointRecord::PointRecord2 {
                mut point_data,
                colour_data,
            } => {
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
                point_data.z = ((residuals[point_num] - input.header.z_offset) / input.header.z_scale_factor) as i32;
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
            progress = (100.0_f64 * point_num as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Saving data: {}%", progress);
                old_progress = progress;
            }
        }
    }

    drop(input);
    drop(residuals);

    let elapsed_time = get_formatted_elapsed_time(start);

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


    if configurations.verbose_mode {
        println!("Elapsed Time (excluding I/O): {}", elapsed_time);
    }
        
    Ok(())
}
