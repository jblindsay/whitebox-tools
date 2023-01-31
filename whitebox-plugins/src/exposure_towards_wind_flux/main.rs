/* 
Authors: Whitebox Geospatial Inc. (c)
Developer: Dr. John Lindsay
Created: 20/07/2021
Last Modified: 20/07/2021
License: Whitebox Geospatial Inc. License Agreement
*/

use std::env;
use std::f64;
use std::f32::consts::PI;
// use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use whitebox_common::utils::{ get_formatted_elapsed_time, wrapped_print };
use whitebox_common::structures::Array2D;
use whitebox_raster::*;
use num_cpus;

/// This tool creates a new raster in which each grid cell is assigned the exposure of the land-surface to 
/// a hypothetical wind flux. It can be conceptualized as the angle between a plane orthogonal to the wind 
/// and a plane that represents the local topography at a grid cell (Bohner and Antonic, 2007). The user must specify 
/// the names of the input digital elevation model (`--dem`) and output file (`--output`), as well as the
/// dominant wind azimuth (`--azimuth`) and a maximum search distance (`--max_dist`) used to calclate the horizon
/// angle. Notice that the specified azimuth represents a regional average wind direction. 
///
/// Exposure towards the sloped wind flux essentially combines the relative terrain aspect and the maximum upwind 
/// slope (i.e. horizon angle). This terrain attribute accounts for land-surface orientation, relative to the wind, 
/// and shadowing effects of distant topographic features but does not account for deflection of the wind by 
/// topography. This tool should not be used on very extensive areas over which Earth's curvature must be taken into 
/// account. DEMs in projected coordinate systems are preferred.
///
/// **Algorithm Description:**
///
/// Exposure is measured based on the equation presented in Antonic and Legovic (1999):
///
/// > cos(*E*) = cos(*S*) sin(*H*) + sin(*S*) cos(*H*) cos(*Az* - *A*)
///
///
/// Where, *E* is angle between a plane defining the local terrain and a plane orthogonal to the wind flux, *S* 
/// is the terrain slope, *A* is the terrain aspect, *Az* is the azimuth of the wind flux, and *H* is the horizon 
/// angle of the wind flux, which is zero when only the horizontal component of the wind flux is accounted for.
///
/// Exposure images are best displayed using a greyscale or bipolar palette to distinguish between the positive 
/// and negative values that are present in the output.
///
/// # References
/// Antonić, O., & Legović, T. 1999. Estimating the direction of an unknown air pollution source using a digital 
/// elevation model and a sample of deposition. *Ecological modelling*, 124(1), 85-95.
///
/// Böhner, J., & Antonić, O. 2009. Land-surface parameters specific to topo-climatology. Developments in Soil 
/// Science, 33, 195-226.
///
/// # See Also
/// `RelativeAspect`
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

    let exe_name = &format!("exposure_towards_wind_flux{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    exposure_towards_wind_flux Help

    This tool performs a Canny edge-detection filter on an input image. 

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem     Name of the input DEM raster file.
    -o, --output  Name of the output raster file.
    --azimuth     Wind azimuth, in degrees.
    --max_dist    Optional maximum search distance. Minimum value is 5 x cell size.
    --z_factor    Optional multiplier for when the vertical and horizontal units are not the same.
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=input.tif -o=new.tif --sigma=0.25 --low=0.1 --high=0.2

    Note: Use of this tool requires a valid license. To obtain a license,
    contact Whitebox Geospatial Inc. (support@whiteboxgeo.com).
    "#
            .replace("*", &sep)
            .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "exposure_towards_wind_flux v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("ExposureTowardsWindFlux")
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

    // read the arguments
    let mut input_file: String = String::new();
    let mut output_file: String = String::new();
    let mut azimuth = 0f32;
    let mut max_dist = f32::INFINITY;
    let mut z_factor = 1f32;

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
        } else if flag_val == "-azimuth" {
            azimuth = if keyval {
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
            azimuth = azimuth % 360f32;
        } else if flag_val == "-max_dist" {
            if keyval {
                max_dist = vec[1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val));
            } else {
                max_dist = args[i + 1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val));
            }
        } else if flag_val == "-zfactor" {
            if keyval {
                z_factor = vec[1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val));
            } else {
                z_factor = args[i + 1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val));
            }
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

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if azimuth < 0.0 {
        if configurations.verbose_mode {
            wrapped_print("Warning: Azimuth should be between 0 and 360.", 50);
        }
        azimuth = 0.0;
    } else if azimuth > 360.0 {
        if configurations.verbose_mode {
            wrapped_print("Warning: Azimuth should be between 0 and 360.", 50);
        }
        azimuth = 360.0;
    }

    let line_slope: f32 = if azimuth < 180f32 {
        (90f32 - azimuth).to_radians().tan()
    } else {
        (270f32 - azimuth).to_radians().tan()
    };

    // azimuth = azimuth.to_radians();

    // Read in the input raster
    let inputf64 = Raster::new(&input_file, "r")?;
    let configs = inputf64.configs.clone();
    let rows = configs.rows as isize;
    let columns = configs.columns as isize;
    let nodata = configs.nodata;
    let nodata_f32 = nodata as f32;
    let cell_size_x = configs.resolution_x as f32;
    let cell_size_y = configs.resolution_y as f32;
    let eight_grid_res = configs.resolution_x as f32 * 8.0;

    let mut z_factor_array = Vec::with_capacity(rows as usize);
    if inputf64.is_in_geographic_coordinates() && z_factor < 0.0 {
        // calculate a new z-conversion factor
        for row in 0..rows {
            let lat = inputf64.get_y_from_row(row);
            z_factor_array.push(1.0 / (111320.0 * lat.cos()) as f32);
        }
    } else {
        z_factor_array = vec![z_factor as f32; rows as usize];
    }

    let z_factor_array = Arc::new(z_factor_array);

    if max_dist <= 5f32 * cell_size_x {
        panic!("The maximum search distance parameter (--max_dist) must be larger than 5 x cell size.");
    }

    // The longest that max_dist ever needs to be is the raster diagonal length.
    let diag_length = ((configs.north - configs.south) * (configs.north - configs.south)
        + (configs.east - configs.west) * (configs.east - configs.west))
        .sqrt() as f32;
    if max_dist > diag_length {
        max_dist = diag_length;
    }

    let input = inputf64.get_data_as_f32_array2d();

    drop(inputf64);


    ////////////////////////////////////
    // Calculate the slope and aspect //
    ////////////////////////////////////
    let mut num_procs = num_cpus::get() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let z_factor_array = z_factor_array.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let mut n: [f32; 8] = [0.0; 8];
            let mut z: f32;
            let (mut fx, mut fy): (f32, f32);
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut aspect_data = vec![nodata_f32; columns as usize];
                let mut slope_data = vec![nodata_f32; columns as usize];
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata_f32 {
                        for c in 0..8 {
                            n[c] = input.get_value(row + dy[c], col + dx[c]);
                            if n[c] != nodata_f32 {
                                n[c] = n[c] * z_factor_array[row as usize];
                            } else {
                                n[c] = z * z_factor_array[row as usize];
                            }
                        }
                        fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                        if fx == 0f32 {
                            fx = 0.00001;
                        }
                        
                        fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                        aspect_data[col as usize] = (180f32 - ((fy / fx).atan()).to_degrees() + 90f32 * (fx / (fx).abs())) as f32;
                        slope_data[col as usize] = (fx * fx + fy * fy).sqrt().atan() as f32;
                    }
                }
                tx.send((row, aspect_data, slope_data)).unwrap();
            }
        });
    }

    let mut aspect: Array2D<f32> = Array2D::new(rows, columns, nodata_f32, nodata_f32)?;
    let mut slope: Array2D<f32> = Array2D::new(rows, columns, nodata_f32, nodata_f32)?;
    for r in 0..rows {
        let (row, aspect_data, slope_data) = rx.recv().expect("Error receiving data from thread.");
        aspect.set_row_data(row, aspect_data);
        slope.set_row_data(row, slope_data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Calculating slope and aspect: {}%", progress);
                old_progress = progress;
            }
        }
    }

    /////////////////////////////
    // Calculate horizon angle //
    /////////////////////////////
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            // The ray-tracing operation can be viewed as a linear maximum filter. The first step
            // is to create the filter offsets and calculate the offset distances.

            let x_step: isize;
            let y_step: isize;
            if azimuth > 0f32 && azimuth <= 90f32 {
                x_step = 1;
                y_step = 1;
            } else if azimuth <= 180f32 {
                x_step = 1;
                y_step = -1;
            } else if azimuth <= 270f32 {
                x_step = -1;
                y_step = -1;
            } else {
                x_step = -1;
                y_step = 1;
            }

            let mut flag: bool;
            let (mut delta_x, mut delta_y): (f32, f32);
            let (mut x, mut y): (f32, f32);
            let (mut x1, mut y1): (isize, isize);
            let (mut x2, mut y2): (isize, isize);
            let (mut z1, mut z2): (f32, f32);
            let mut dist: f32;
            let mut weight: f32;
            let mut offsets = vec![];

            // Find all of the horizontal grid intersections.
            if line_slope != 0f32 {
                // Otherwise, there are no horizontal intersections.
                y = 0f32;
                flag = true;
                while flag {
                    y += y_step as f32;
                    x = y / line_slope;

                    // calculate the distance
                    delta_x = x * cell_size_x;
                    delta_y = -y * cell_size_y;
                    dist = delta_x.hypot(delta_y);
                    if dist <= max_dist {
                        x1 = x.floor() as isize;
                        x2 = x1 + 1;
                        y1 = -y as isize;
                        weight = x - x1 as f32;
                        offsets.push((x1, y1, x2, y1, weight, dist));
                    } else {
                        flag = false;
                    }
                }
            }

            // Find all of the vertical grid intersections.
            x = 0f32;
            flag = true;
            while flag {
                x += x_step as f32;
                y = -(line_slope * x); // * -1f32;

                // calculate the distance
                delta_x = x * cell_size_x;
                delta_y = y * cell_size_y;

                dist = delta_x.hypot(delta_y);
                if dist <= max_dist {
                    y1 = y.floor() as isize;
                    y2 = y1 + 1; // - y_step;
                    x1 = x as isize;
                    weight = y - y1 as f32;
                    offsets.push((x1, y1, x1, y2, weight, dist));
                } else {
                    flag = false;
                }
            }

            // Sort by distance.
            offsets.sort_by(|a, b| a.5.partial_cmp(&b.5).unwrap());

            let num_offsets = offsets.len();
            let mut z: f32;
            let mut slope: f32;
            let early_stopping_slope = 80f32.to_radians().tan();
            let mut current_elev: f32;
            let mut current_max_slope: f32;
            let mut current_max_elev: f32;
            let a_small_value = -9999999f32;
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data: Vec<f32> = vec![nodata_f32; columns as usize];
                for col in 0..columns {
                    current_elev = input.get_value(row, col);
                    if current_elev != nodata_f32 {
                        // Run down the offsets of the ray
                        current_max_slope = a_small_value;
                        current_max_elev = a_small_value;
                        for i in 0..num_offsets {
                            // Where are we on the grid?
                            x1 = col + offsets[i].0;
                            y1 = row + offsets[i].1;
                            x2 = col + offsets[i].2;
                            y2 = row + offsets[i].3;

                            // What is the elevation?
                            z1 = input.get_value(y1, x1);
                            z2 = input.get_value(y2, x2);

                            if z1 == nodata_f32 && z2 == nodata_f32 {
                                break; // We're likely off the grid.
                            } else if z1 == nodata_f32 {
                                z1 = z2;
                            } else if z2 == nodata_f32 {
                                z2 = z2;
                            }

                            z = z1 + offsets[i].4 * (z2 - z1);

                            // All previous cells are nearer, and so if this isn't a higher
                            // cell than the current highest, it can't be the horizon cell.
                            if z > current_max_elev {
                                current_max_elev = z;

                                // Calculate the slope
                                slope = (z - current_elev) / offsets[i].5;
                                if slope > current_max_slope {
                                    current_max_slope = slope;
                                    if slope > early_stopping_slope {
                                        break; // we're unlikely to find a farther horizon cell.
                                    }
                                }
                            }
                        }

                        if current_max_slope == a_small_value {
                            data[col as usize] = 0f32; // It's a zero-length scan. We didn't encounter any valid cells.
                        } else {
                            data[col as usize] = current_max_slope.atan();
                        }
                    }
                }
                tx.send((row, data)).unwrap();
            }
        });
    }

    let mut horizon_angle: Array2D<f32> = Array2D::new(rows, columns, nodata_f32, nodata_f32)?;
    for r in 0..rows {
        let (row, data) = rx.recv().expect("Error receiving data from thread.");
        horizon_angle.set_row_data(row, data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Calculating horizon angle: {}%", progress);
                old_progress = progress;
            }
        }
    }


    //////////////////////////////////////////
    // Calculate exposure towards wind flux //
    //////////////////////////////////////////
    let aspect = Arc::new(aspect);
    let slope = Arc::new(slope);
    let horizon_angle = Arc::new(horizon_angle);
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let aspect = aspect.clone();
        let slope = slope.clone();
        let horizon_angle = horizon_angle.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut relative_aspect: f32;
            let (mut slope_val, mut aspect_val, mut ha_val): (f32, f32, f32);
            let mut val: f64;
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data = vec![nodata; columns as usize];
                for col in 0..columns {
                    aspect_val = aspect.get_value(row, col);
                    relative_aspect = (azimuth - aspect_val).abs().to_radians();
                    if relative_aspect > PI {
                        relative_aspect = 2f32 * PI - relative_aspect;
                    }
                    if aspect_val != nodata_f32 {
                        slope_val = slope.get_value(row, col);
                        ha_val = horizon_angle.get_value(row, col).max(0f32);
                        // cosγW =sinφ·cosβ+cosφ·sinβ·cosαr
                        val = (ha_val.sin() * slope_val.cos() + ha_val.cos() * slope_val.sin() * relative_aspect.cos()) as f64;
                        data[col as usize] = val;
                    }
                }
                tx.send((row, data)).unwrap();
            }
        });
    }

    let mut output = Raster::initialize_from_array2d(&output_file, &configs, &input);
    for r in 0..rows {
        let (row, data) = rx.recv().expect("Error receiving data from thread.");
        output.set_row_data(row, data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Calculating index: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // let mut relative_aspect: f32;
    // for row in 0..rows {
    //     for col in 0..columns {
    //         if input.get_value(row, col) != nodata_f32 {
    //             relative_aspect = (azimuth - aspect.get_value(row, col)).abs().to_radians();
    //             if relative_aspect > PI {
    //                 relative_aspect = 2f32 * PI - relative_aspect;
    //             }
    //             output.set_value(row, col, relative_aspect.to_degrees() as f64);
    //         } else {
    //             output.set_value(row, col, nodata);
    //         }
    //     }
    // }

    drop(input);

    
    //////////////////////
    // Output the image //
    //////////////////////

    let elapsed_time = get_formatted_elapsed_time(start);
    output.configs.palette = "grey.plt".to_string();
    output.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool",
        tool_name
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

    Ok(())
}
