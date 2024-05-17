/* 
Authors:  Dr. John Lindsay
Created: 08/05/2024
Last Modified: 08/05/2024
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
use whitebox_common::utils::{
    get_formatted_elapsed_time, 
    haversine_distance,
    vincenty_distance
};
use whitebox_raster::*;

/// This tool calculates the convergence index (<it>C</it>), described by Koethe and Lehmeier (1996) and Kiss (2004), for each grid cell
/// in an input digital elevation model (DEM). The convergence index measures the average amount by which the aspect value
/// of each of the eight neighbours in a 3x3 kernel deviates from an aspect aligned with the direction towards 
/// the center cell. As such the index measures the degree to which the surrounding topography converges on the center cell.
/// 
/// <it>C</it> = 1 / 8 &Sigma;|&Phi; - Az<sub>0</sub>| - 90 
/// 
/// Where &Phi; is the aspect of a neighbour of the center cell and Az<sub>0</sub> is the azimuth
/// from the neighbour directed towards the center cell. Note, -90 < <it>C</it> < 90, where highly convergent areas have 
/// values near -90 and highly divergent areas have values near 90. Therefore, in actuality, <it>C</it> is more properly 
/// an index of divergence rather than a convergence index, despite its name.
/// 
/// ![](../../doc_img/ConvergenceIndex.png)
/// 
/// The user must specify the name of the input DEM (`dem`) and the 
/// output raster (`output`). The Z conversion factor (`zfactor`) is only important when the vertical and 
/// horizontal units are not the same in the DEM, and the DEM is in a projected coordinate system. When this is the case, the algorithm will multiply each elevation 
/// in the DEM by the Z Conversion Factor to perform the unit conversion. 
/// 
/// For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate
/// Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit
/// of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more
/// robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems
/// (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also
/// described by Florinsky (2016).
/// 
/// # Reference
/// Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press.
/// 
/// Kiss, R. (2004). Determination of drainage network in digital elevation models, utilities and 
/// limitations. Journal of Hungarian geomathematics, 2, 17-29.
/// 
/// Koethe, R. and Lehmeier, F. (1996): SARA - System zur Automatischen Relief-Analyse. User Manual, 
/// 2. Edition [Dept. of Geography, University of Goettingen, unpublished]
/// 
/// # See Also
/// `Aspect`
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

    let exe_name = &format!("convergence_index{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    convergence_index Help

    This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Qin et al. (2007) 
    flow algorithm.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --output       Name of the output raster file.
    --z_factor     Optional multiplier for when the vertical and horizontal units are not the same.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=convergence.tif --z_factor=1.0
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "convergence_index v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("ConvergenceIndex") // This should be camel case and is a reference to the tool name.
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
    let mut z_factor = 1f64;
    
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

    let mut num_procs = num_cpus::get() as isize;
    let max_procs = configurations.max_procs;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    /////////////////////////////////////////////////////
    // Read in the DEM and create a D8 pointer from it //
    /////////////////////////////////////////////////////
    let input = Arc::new(Raster::new(&dem_file, "r")?);
    let rows = input.configs.rows as isize;
    let columns = input.configs.columns as isize;
    let nodata = input.configs.nodata;
    let resx = input.configs.resolution_x;
    let resy = input.configs.resolution_y;
    let res = (resx + resy) / 2.;

    // println!("{rows}, {columns}, {nodata}, {resx}, {resy}, {res}");

    let (tx, rx) = mpsc::channel();
    if !input.is_in_geographic_coordinates() {
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z12: f64;
                let mut p: f64;
                let mut q: f64;
                let mut sign_p: f64;
                let mut sign_q: f64;
                const PI: f64 = std::f64::consts::PI;
                let offsets = [
                    [-2, -2], [-1, -2], [0, -2], [1, -2], [2, -2], 
                    [-2, -1], [-1, -1], [0, -1], [1, -1], [2, -1], 
                    [-2, 0], [-1, 0], [0, 0], [1, 0], [2, 0], 
                    [-2, 1], [-1, 1], [0, 1], [1, 1], [2, 1], 
                    [-2, 2], [-1, 2], [0, 2], [1, 2], [2, 2]
                ];
                let mut z = [0f64; 25];
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z12 = input.get_value(row, col);
                        if z12 != nodata {
                            for n in 0..25 {
                                z[n] = input.get_value(row + offsets[n][1], col + offsets[n][0]);
                                if z[n] != nodata {
                                    z[n] *= z_factor;
                                } else {
                                    z[n] = z12 * z_factor;
                                }
                            }

                            /* 
                            The following equations have been taken from Florinsky (2016) Principles and Methods
                            of Digital Terrain Modelling, Chapter 4, pg. 117.
                            */
                            p = 1. / (420. * res) * (44. * (z[3] + z[23] - z[1] - z[21]) + 31. * (z[0] + z[20] - z[4] - z[24]
                                + 2. * (z[8] + z[18] - z[6] - z[16])) + 17. * (z[14] - z[10] + 4. * (z[13] - z[11]))
                                + 5. * (z[9] + z[19] - z[5] - z[15]));

                            q = 1. / (420. * res) * (44. * (z[5] + z[9] - z[15] - z[19]) + 31. * (z[20] + z[24] - z[0] - z[4]
                                + 2. * (z[6] + z[8] - z[16] - z[18])) + 17. * (z[2] - z[22] + 4. * (z[7] - z[17]))
                                + 5. * (z[1] + z[3] - z[21] - z[23]));

                            if p != 0f64 { // slope is greater than zero
                                // data[col as usize] = 180f64 - (q / p).atan().to_degrees() + 90f64 * (p / p.abs());
                                sign_p = if p != 0. { p.signum() } else { 0. };
                                sign_q = if q != 0. { q.signum() } else { 0. };
                                data[col as usize] = -90.*(1. - sign_q)*(1. - sign_p.abs()) + 180.*(1. + sign_p) - 180. / PI * sign_p * (-q / (p*p + q*q).sqrt()).acos();
                            } else {
                                data[col as usize] = -1f64; // undefined for flat surfaces
                            }
                        }
                    }

                    tx.send((row, data)).unwrap();
                }
            });
        }
    } else { // geographic coordinates

        let phi1 = input.get_y_from_row(0);
        let lambda1 = input.get_x_from_column(0);

        let phi2 = phi1;
        let lambda2 = input.get_x_from_column(-1);

        let linear_res = vincenty_distance((phi1, lambda1), (phi2, lambda2));
        let lr2 =  haversine_distance((phi1, lambda1), (phi2, lambda2)); 
        let diff = 100. * (linear_res - lr2).abs() / linear_res;
        let use_haversine = diff < 0.5; // if the difference is less than 0.5%, use the faster haversine method to calculate distances.

        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z4: f64;
                let mut p: f64;
                let mut q: f64;
                let mut a: f64;
                let mut b: f64;
                let mut c: f64;
                let mut d: f64;
                let mut e: f64;
                let mut phi1: f64;
                let mut lambda1: f64;
                let mut phi2: f64;
                let mut lambda2: f64;
                let offsets = [
                    [-1, -1], [0, -1], [1, -1], 
                    [-1, 0], [0, 0], [1, 0], 
                    [-1, 1], [0, 1], [1, 1]
                ];
                let mut z = [0f64; 25];
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z4 = input.get_value(row, col);
                        if z4 != nodata {
                            for n in 0..9 {
                                z[n] = input.get_value(row + offsets[n][1], col + offsets[n][0]);
                                if z[n] != nodata {
                                    z[n] *= z_factor;
                                } else {
                                    z[n] = z4 * z_factor;
                                }
                            }

                            // Calculate a, b, c, d, and e.
                            phi1 = input.get_y_from_row(row);
                            lambda1 = input.get_x_from_column(col);

                            phi2 = phi1;
                            lambda2 = input.get_x_from_column(col-1);

                            b = if use_haversine {
                                haversine_distance((phi1, lambda1), (phi2, lambda2))
                            } else {
                                vincenty_distance((phi1, lambda1), (phi2, lambda2))
                            };

                            phi2 = input.get_y_from_row(row+1);
                            lambda2 = lambda1;

                            d = if use_haversine {
                                haversine_distance((phi1, lambda1), (phi2, lambda2))
                            } else {
                                vincenty_distance((phi1, lambda1), (phi2, lambda2))
                            };

                            phi2 = input.get_y_from_row(row-1);
                            lambda2 = lambda1;

                            e = if use_haversine {
                                haversine_distance((phi1, lambda1), (phi2, lambda2))
                            } else {
                                vincenty_distance((phi1, lambda1), (phi2, lambda2))
                            };

                            phi1 = input.get_y_from_row(row+1);
                            lambda1 = input.get_x_from_column(col);

                            phi2 = phi1;
                            lambda2 = input.get_x_from_column(col-1);

                            a = if use_haversine {
                                haversine_distance((phi1, lambda1), (phi2, lambda2))
                            } else {
                                vincenty_distance((phi1, lambda1), (phi2, lambda2))
                            };

                            phi1 = input.get_y_from_row(row-1);
                            lambda1 = input.get_x_from_column(col);

                            phi2 = phi1;
                            lambda2 = input.get_x_from_column(col-1);

                            c = if use_haversine {
                                haversine_distance((phi1, lambda1), (phi2, lambda2))
                            } else {
                                vincenty_distance((phi1, lambda1), (phi2, lambda2))
                            };

                            /* 
                            The following equations have been taken from Florinsky (2016) Principles and Methods
                            of Digital Terrain Modelling, Chapter 4, pg. 117.
                            */

                            p = (a * a * c * d * (d + e) * (z[2] - z[0]) + b * (a * a * d * d + c * c * e * e) * (z[5] - z[3]) + a * c * c * e * (d + e) * (z[8] - z[6]))
                            / (2. * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

                            q = 1. / (3. * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4))) 
                            * ((d * d * (a.powi(4) + b.powi(4) + b * b * c * c) + c * c * e * e * (a * a - b * b)) * (z[0] + z[2])
                            - (d * d * (a.powi(4) + c.powi(4) + b * b * c * c) - e * e * (a.powi(4) + c.powi(4) + a * a * b * b)) * (z[3] + z[5])
                            - (e * e * (b.powi(4) + c.powi(4) + a * a * b * b) - a * a * d * d * (b * b - c * c)) * (z[6] + z[8])
                            + d * d * (b.powi(4) * (z[1] - 3. * z[4]) + c.powi(4) * (3. * z[1] - z[4]) + (a.powi(4) - 2. * b * b * c * c) * (z[1] - z[4]))
                            + e * e * (a.powi(4) * (z[4] - 3. * z[7]) + b.powi(4) * (3. * z[4] - z[7]) + (c.powi(4) - 2. * a * a * b * b) * (z[4] - z[7]))
                            - 2. * (a * a * d * d * (b * b - c * c) * z[7] + c * c * e * e * (a * a - b * b) * z[1]));
                            
                            if p != 0f64 { // slope is greater than zero
                                data[col as usize] = 180f64 - (q / p).atan().to_degrees() + 90f64 * (p / p.abs());
                            } else {
                                data[col as usize] = -1f64; // undefined for flat surfaces
                            }
                        }
                    }

                    tx.send((row, data)).unwrap();
                }
            });
        }
    }

    let mut aspect = Raster::initialize_using_file(&"aspect.tif", &input);
    
    for row in 0..rows {
        let (r, data) = rx.recv().expect("Error receiving data from thread.");
        aspect.set_row_data(r, data);
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    drop(input);

    let aspect = Arc::new(aspect);
    for tid in 0..num_procs {
        let aspect = aspect.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut z: f64;
            let mut sum: f64;
            let mut relative_aspect: f64;
            let mut num_neighbours: f64;
            let offsets = [
                [-1, -1], [0, -1], [1, -1], 
                [-1, 0],           [1, 0], 
                [-1, 1],  [0, 1],  [1, 1]
            ];
            let azimuth = [
                135f64, 180f64, 225f64, 
                90f64,          270f64, 
                45f64,  0f64,   315f64
            ];
            
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data = vec![nodata; columns as usize];
                for col in 0..columns {
                    if aspect.get_value(row, col) != nodata {
                        sum = 0f64;
                        num_neighbours = 0f64;
                        for n in 0..8 {
                            z = aspect.get_value(row + offsets[n][1], col + offsets[n][0]);
                            if z != nodata {
                                relative_aspect = (z - azimuth[n]).abs();
                                if relative_aspect > 180.0 {
                                    relative_aspect = 360.0 - relative_aspect;
                                }
                                sum += relative_aspect;
                                num_neighbours += 1.0;
                            }
                        }
                        data[col as usize] = sum / num_neighbours - 90f64;
                    }
                }

                tx.send((row, data)).unwrap();
            }
        });
    }

    let mut output = Raster::initialize_using_file(&output_file, &aspect);
    for row in 0..rows {
        let (r, data) = rx.recv().expect("Error receiving data from thread.");
        output.set_row_data(r, data);
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress: {}%", progress);
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
