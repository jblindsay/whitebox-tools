/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: 01/01/2021
Last Modified: 01/01/2021
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use whitebox_common::rendering::html::*;
use whitebox_common::rendering::LineGraph;
use whitebox_vector::{ShapeType, Shapefile};
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use whitebox_common::utils::get_formatted_elapsed_time;

/// This tool uses the fast Gaussian approximation algorithm to produce scaled land-surface parameter (LSP)
/// measurements from an input DEM (`--dem`). The algorithm iterates over scales
/// defined by an initial scale (`--sigma`), a step size (--step) and a
/// number of scales (--num_steps)/. After smoothing the input DEM to the target sigma,
/// a 3x3 window is used to calculate a variety of LSPs (`--lsp`).
/// LSP options include local derivatives (Elevation, Slope, Aspect, Eastness, Northness,
/// Mean curvature, Plan curvature, Profile curvature, Tangential curvature and Total curvature),
/// Hillshade, and Difference from mean elevation, all as defined in Wilson (2018), and Anisotropy
/// of topographic position (Newman et al., 2018), and Ruggedness (Riley et al., 1999). An initial
/// sigma value of 0 will compute the LSP without Gaussian smoothing. The step size can be and
/// positive number, however, sigam values < 0.5 and duplicated scales are skipped due to a
/// minimum filter size limitation, or the scale discretization of the fast gaussian approximation.
///
/// The LSP values are then transformed to z-scores using the population of values at a single
/// scale, are are evaluated to identify the optimal scale defined as the maximum absolute z-score
/// for each cell. The final outputs are three rasters: the first containing the z-score at the
/// optimal scale (z_opt), the sigma value at the optimal scale (g_opt), and the LSP value at the
/// optimal scale (v_opt). These all need to be specified using the (`--output_zscore`),
/// (`--output_scale`), and (`--output`) flags respectively. Additionally, a vector file of
/// points (`--points`) can optionally be provided to generate scale signatures for the provided point
/// locations.
///
/// Due to the use of the integral image, edge effects can be problematic; especially
/// when 'NoData' values are found. It is recommended that 'NoData' holes filled during
/// pre-processing. Irregular shaped data (i.e., non-rectangular DEMs) are buffered with a crude
/// check for 'NoData' values at the filter edge in the 8 cardinal directions to buffer the
/// edges. This should be adequate for most data, additional buffer masks may be required.
///
/// # Reference
/// Wilson, J. P. (2018). Environmental-applications-of-digital-terrain-modeling. Wiley Blackwell.
/// Newman, D. R., Lindsay, J. B., & Cockburn, J. M. H. (2018). Measuring Hyperscale Topographic
/// Anisotropy as a Continuous Landscape Property. *Geosciences*, 8(278).
/// https://doi.org/10.3390/geosciences8080278
/// 
/// Riley, S. J., DeGloria, S. D., and Elliot, R. (1999). Index that quantifies topographic
/// heterogeneity.*Intermountain Journal of Sciences*, 5(1-4), 23-27.
///
/// # See Also
/// `MaxDifferenceFromMean`, `MaxAnisotropyDev`, `ProfileCurvature`, `TangentialCurvature`, `RuggednessIndex`
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

    let exe_name = &format!("gaussian_scale_space{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    gaussian_scale_space Help

    This tool uses the fast Gaussian approximation algorithm to produce scaled land-surface parameter (LSP)
    measurements from an input DEM.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file.
    -o, --output   Name of the output raster file.
    --filter       Edge length of the filter kernel.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=slope.tif --output_zscore=slope_z.tif --output_scale=slope_scale.tif --sigma=0.5 --step=1.0 --num_steps=100 --lsp='Slope'
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "gaussian_scale_space v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("GaussianScaleSpace") // This should be camel case and is a reference to the tool name.
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

    let mut input_file = String::new();
    let mut points_file = String::new();
    let mut output_file = String::new();
    let mut output_zscore_file = String::new();
    let mut output_scale_file = String::new();
    let mut sigma_i = 0.5f64;
    let mut step = 0.5f64;
    let mut num_steps = 10isize;
    let mut lsp_fmt = "Slope".to_string();
    let mut z_factor = -1f64;
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
        if flag_val == "-d" || flag_val == "-dem" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i + 1].to_string();
            }
        } else if flag_val == "-p" || flag_val == "-points" {
            points_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-o" || flag_val == "-output" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i + 1].to_string();
            }
        } else if flag_val == "-output_scale" {
            if keyval {
                output_scale_file = vec[1].to_string();
            } else {
                output_scale_file = args[i + 1].to_string();
            }
        } else if flag_val == "-output_zscore" {
            if keyval {
                output_zscore_file = vec[1].to_string();
            } else {
                output_zscore_file = args[i + 1].to_string();
            }
        } else if flag_val == "-sigma" {
            sigma_i = if keyval {
                vec[1].to_string().parse::<f64>().unwrap()
            } else {
                args[i + 1].to_string().parse::<f64>().unwrap()
            };
        } else if flag_val == "-step" {
            step = if keyval {
                vec[1].to_string().parse::<f64>().unwrap()
            } else {
                args[i + 1].to_string().parse::<f64>().unwrap()
            };
        } else if flag_val == "-num_steps" {
            num_steps = if keyval {
                vec[1].to_string().parse::<isize>().unwrap()
            } else {
                args[i + 1].to_string().parse::<isize>().unwrap()
            };
        } else if flag_val == "-lsp" {
            lsp_fmt = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            }
        } else if flag_val == "-zfactor" {
            z_factor = if keyval {
                vec[1].to_string().parse::<f64>().unwrap()
            } else {
                args[i + 1].to_string().parse::<f64>().unwrap()
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

    let lsp_func = match &lsp_fmt.to_uppercase()[0..2] {
        "AN" => fn_anisotropy, // ANISOTROPYLTP
        "AS" => fn_aspect, // ASPECT
        "DM" => fn_dme, // DME
        "DI" => fn_dme, // DIFFERENCEMEANELEVATION
        "EA" => fn_eastness, // EASTNESS
        "EL" => fn_elevation, // ELEVATION
        "HI" => fn_hillshade, // HILLSHADE
        "ME" => fn_mean_curvature, // MEANCURVATURE
        "NO" => fn_northness, // NORTHNESS
        "PL" => fn_plan_curvature, // PLANCURVATURE
        "PR" => fn_prof_curvature, // PROFILECURVATURE
        "RU" => fn_ruggedness, // RUGGEDNESS
        "SL" => fn_slope, // SLOPE
        "TA" => fn_tan_curvature, // TANCURVATURE
        "TO" => fn_total_curvature, // TOTALCURVATURE
        _ => {
            eprintln!("Warning: Invalid LSP. Defaulting to Elevation.");
            fn_elevation
        }
    };

    let sep: String = path::MAIN_SEPARATOR.to_string();

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if points_file.len() > 0 {
        if !points_file.contains(&sep) && !points_file.contains("/") {
            points_file = format!("{}{}", working_directory, points_file);
        }
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }
    if !output_scale_file.contains(&sep) && !output_scale_file.contains("/") {
        output_scale_file = format!("{}{}", working_directory, output_scale_file);
    }
    if !output_zscore_file.contains(&sep) && !output_zscore_file.contains("/") {
        output_zscore_file = format!("{}{}", working_directory, output_zscore_file);
    }

    // LSP signature output file
    let p = path::Path::new(&output_file);
    let mut extension = String::from(".");
    let ext = p.extension().unwrap().to_str().unwrap();
    extension.push_str(ext);
    let output_points_file = output_file.replace(&extension, ".html");

    // LSP Z-score signature output file
    let p = path::Path::new(&output_zscore_file);
    let mut extension = String::from(".");
    let ext = p.extension().unwrap().to_str().unwrap();
    extension.push_str(ext);
    let output_points_zscore_file = output_zscore_file.replace(&extension, ".html");

    // Memory requirements: assuming f64 input
    let input_raster = Raster::new(&input_file, "r")?; //Memory requirements: 1x
    let is_geogrpahic_coords = input_raster.is_in_geographic_coordinates();
    let configs = input_raster.configs.clone();
    let rows = configs.rows as isize;
    let columns = configs.columns as isize;
    let res = configs.resolution_x;
    let res_y = configs.resolution_y;
    let input_north = configs.north;
    let nodata = configs.nodata;
    let nodata32 = nodata as f32;

    let num_procs = num_cpus::get() as isize;

    if sigma_i < 0f64 {
        eprintln!("Warning: Sigma must be >= 0. Value set to 0.0.");
        sigma_i = 0f64;
    }

    if step <= 0f64 {
        eprintln!("Warning: Step value must be greater than 0. Value set to 0.5.");
        step = 0.5f64;
    }

    if num_steps < 1 {
        eprintln!("Warning: Number of steps must be at least 1.");
        num_steps = 1;
    }

    let mut integral_n = Array2D::new(rows, columns, 0i32, -1i32)?; //Memory requirements: 1.5x
    let mut integral = Array2D::new(rows, columns, 0f64, nodata)?; //Memory requirements: 2.5x
    let mut lsp_data = Array2D::new(rows, columns, nodata, nodata)?; //Memory requirements: 3.5x
    let mut output_lsp = Array2D::new(rows, columns, nodata32, nodata32)?; //Memory requirements: 4.5x
    let mut output_scl = Array2D::new(rows, columns, nodata32, nodata32)?; //Memory requirements: 4.5x
    let mut output_zsc = Array2D::new(rows, columns, nodata32, nodata32)?; //Memory requirements: 5x

    let mut progress: usize;
    let mut old_progress: usize = 1;

    let start = Instant::now();

    let points: Shapefile;
    let mut signature_sites = vec![];
    let mut xdata = vec![];
    let mut ydata = vec![];
    let mut zdata = vec![];
    let mut series_names = vec![];
    let mut num_sites = 0usize;
    if points_file.len() > 0 {
        points = Shapefile::read(&points_file)?;

        // make sure the input vector file is of points type
        if points.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        for record_num in 0..points.num_records {
            let record = points.get_record(record_num);
            let row = input_raster.get_row_from_y(record.points[0].y);
            let col = input_raster.get_column_from_x(record.points[0].x);
            if row >= 0 && col >= 0 && row < rows && col < columns {
                signature_sites.push((row, col));
                xdata.push(vec![]);
                ydata.push(vec![]);
                zdata.push(vec![]);
                series_names.push(format!("Site {}", record_num + 1));
                num_sites += 1;
            }

            if configurations.verbose_mode {
                progress = (100.0_f64 * record_num as f64 / (points.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding site row/column values: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(points);
    }

    // Create the initial integral images.
    let mut val: f64;
    let (mut sum, mut sum_n): (f64, i32);
    let (mut i_prev, mut n_prev): (f64, i32);

    for row in 0..rows {
        sum = 0f64;
        sum_n = 0;
        for col in 0..columns {
            val = input_raster.get_value(row, col);
            if val == nodata {
                val = 0f64;
            } else {
                sum_n += 1;
            }
            sum += val;
            if row > 0 {
                i_prev = integral.get_value(row - 1, col);
                n_prev = integral_n.get_value(row - 1, col);
                integral.set_value(row, col, sum + i_prev);
                integral_n.set_value(row, col, sum_n + n_prev);
            } else {
                integral.set_value(row, col, sum);
                integral_n.set_value(row, col, sum_n);
            }
            if configurations.verbose_mode {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating integral images: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    let input = Arc::new(input_raster.get_data_as_f32_array2d()); // Memory requirements: 5.5x
    drop(input_raster); // Memory requirements: 4.5x

    let integral_n = Arc::new(integral_n);

    // start main analysis
    let mut sigma_prev = -1f64;
    let mut filter_size: isize;
    'scales: for s in 0..(num_steps as usize) {
        let mut sigma = sigma_i + (step * s as f64);
        sigma = sigma * (sigma >= 0.5f64) as usize as f64; // sigma if >= 0.5 else 0
        let mut sigma_actual = sigma;
        if sigma_actual == sigma_prev {
            continue 'scales; // skip if scale discretizes to the same as previous iteration
        }

        // Step 1: Smooth DEM
        let mut smoothed_elev = Array2D::new(rows, columns, nodata, nodata)?; //Memory requirements: 5.5x
        if sigma == 0f64 {
            // Do not smooth. Just read input data into smoothed_elev as f64
            filter_size = 0isize;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let inp = input.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut smoothed_data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            smoothed_data[col as usize] = inp.get_value(row, col) as f64
                        }
                        tx1.send((row, smoothed_data)).unwrap();
                    }
                });
            }

            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                smoothed_elev.set_row_data(data.0, data.1);
                if configurations.verbose_mode {
                    progress = (100f32 * row as f32 / (rows - 1) as f32) as usize;
                    if progress != old_progress {
                        println!("Loop {} of {}. Smoothing progress: {}%",s+1, num_steps, progress);
                        old_progress = progress;
                    }
                }
            }
        } else if sigma < 3f64 { // perform standard gaussian
            let recip_root_2_pi_times_sigma_d = 1f64 / ((2f64 * PI).sqrt() * sigma);
            let two_sigma_sqr_d = 2f64 * sigma * sigma;

            // figure out the size of the filter
            filter_size = 0isize;
            let mut weight: f64;
            // probably faster to just do +/-3*sigma for 99% coverage
            for i in 0..250 {
                weight =
                    recip_root_2_pi_times_sigma_d * (-1f64 * ((i * i) as f64) / two_sigma_sqr_d).exp();
                if weight <= 0.001 {
                    filter_size = i * 2 + 1;
                    break;
                }
            }

            // the filter dimensions must be odd numbers such that there is a middle pixel
            if filter_size % 2 == 0 {
                filter_size += 1;
            }
            if filter_size < 3 {
                filter_size = 3;
            }
            let num_pixels_in_filter = (filter_size * filter_size) as usize;
            let mut dx = vec![0isize; num_pixels_in_filter];
            let mut dy = vec![0isize; num_pixels_in_filter];
            let mut weights = vec![0f64; num_pixels_in_filter];

            // fill the filter d_x and d_y values and the distance-weights
            let midpoint = (filter_size as f64 / 2f64).floor() as isize; // + 1;
            let mut a = 0;
            let mut g_sum = 0f64;
            let (mut x, mut y): (isize, isize);
            for row in 0..filter_size {
                for col in 0..filter_size {
                    x = col as isize - midpoint;
                    y = row as isize - midpoint;
                    dx[a] = x;
                    dy[a] = y;
                    weight = recip_root_2_pi_times_sigma_d
                        * (-1f64 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                    weights[a] = weight;
                    g_sum += weight;
                    a += 1;
                }
            }

            for a in 0..num_pixels_in_filter {
                weights[a] /= g_sum;
            }

            let dx = Arc::new(dx);
            let dy = Arc::new(dy);
            let weights = Arc::new(weights);
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let inp = input.clone();
                let dx = dx.clone();
                let dy = dy.clone();
                let weights = weights.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut w, mut sum, mut z_final): (f64, f64, f64);
                    let mut zn: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut smoothed_data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            if inp.get_value(row, col) as f64 != nodata {
                                sum = 0f64;
                                z_final = 0f64;
                                for a in 0..num_pixels_in_filter {
                                    w = weights[a as usize];
                                    zn = inp.get_value(row + dy[a as usize], col + dx[a as usize]) as f64;
                                    if zn != nodata {
                                        sum += w;
                                        z_final += w * zn;
                                    }
                                }
                                smoothed_data[col as usize] = z_final / sum;
                            }
                        }
                        tx1.send((row, smoothed_data)).unwrap();
                    }
                });
            }

            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                smoothed_elev.set_row_data(data.0, data.1);
                if configurations.verbose_mode {
                    progress = (100f32 * row as f32 / (rows - 1) as f32) as usize;
                    if progress != old_progress {
                        println!("Loop {} of {}. Smoothing progress: {}%",s+1, num_steps, progress);
                        old_progress = progress;
                    }
                }
            }
        } else { //perform fast gaussian
            let n = 6;
            let w_ideal = (12f64 * sigma * sigma / n as f64 + 1f64).sqrt();
            let mut wl = w_ideal.floor() as isize;
            if wl % 2 == 0 { wl -= 1; } // must be an odd integer
            let wu = wl + 2;
            filter_size = wu;
            let m =
                ((12f64 * sigma * sigma - (n * wl * wl) as f64 - (4 * n * wl) as f64 - (3 * n) as f64)
                    / (-4 * wl - 4) as f64)
                    .round() as isize;

            sigma_actual =
                (((m * wl * wl) as f64 + ((n - m) as f64) * (wu * wu) as f64 - n as f64) / 12f64)
                    .sqrt();

            if sigma_actual == sigma_prev {
                continue 'scales; // skip if scale discretizes to the same as previous iteration
            }

            // avoid recalculating the first integral image
            let mut integral_mod = integral.duplicate(); //Memory requirements: 6.5x

            for iteration_num in 0..n {
                let midpoint = if iteration_num <= m {
                    (wl as f64 / 2f64).floor() as isize
                } else {
                        (wu as f64 / 2f64).floor() as isize
                };

                if iteration_num > 0 {
                    // update integral_mod based on smoothed_elev
                    let (mut val, mut sum, mut i_prev): (f64, f64, f64);
                    for row in 0..rows {
                        sum = 0f64;
                        for col in 0..columns {
                            val = smoothed_elev.get_value(row, col);
                            val = val * (val != nodata) as usize as f64; // val if val!=0 else 0
                            sum += val;
                            if row > 0 {
                                i_prev = integral_mod.get_value(row - 1, col);
                                integral_mod.set_value(row, col, sum + i_prev);
                            } else {
                                integral_mod.set_value(row, col, sum);
                            }
                        }
                    }
                }

                // Perform Filter
                let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                let (mut num_cells, mut sum): (i32, f64);
                for row in 0..rows {
                    y1 = row - midpoint - 1;
                    y1 = y1 * (y1 >= 0) as isize;
                    y2 = row + midpoint;
                    if y2 >= rows {
                        y2 = rows - 1;
                    }
                    for col in 0..columns {
                        if input.get_value(row, col) != nodata32 {
                            x1 = col - midpoint - 1;
                            x1 = x1 * (x1 >= 0) as isize;
                            x2 = col + midpoint;
                            if x2 >= columns {
                                x2 = columns - 1;
                            }

                            num_cells = integral_n.get_value(y2,x2)
                                + integral_n.get_value(y1,x1)
                                - integral_n.get_value(y1,x2)
                                - integral_n.get_value(y2,x1);
                            if num_cells > 0 {
                                sum = integral_mod.get_value(y2,x2)
                                    + integral_mod.get_value(y1,x1)
                                    - integral_mod.get_value(y1,x2)
                                    - integral_mod.get_value(y2,x1);
                                smoothed_elev.set_value(row, col, sum / num_cells as f64);
                            } else {
                                // should never reach this point
                                smoothed_elev.set_value(row, col, nodata);
                            }
                        }
                    }
                }
            }
            // Memory requirements: 5.5x -> integral_mod drops out of scope
        }
        sigma_prev = sigma_actual;
        let buffer = if sigma < 3f64 { // not fast gaussian, no buffer
            0isize
        } else { // is fast gaussian, buffer by upper window lenght
                filter_size+1//(filter_size as f64 / 2f64).floor() as isize + 1
        };

        // Step 2: Calculate LSP on smoothed DEM
        let smoothed_elev = Arc::new(smoothed_elev);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let inp = smoothed_elev.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut val: f64;
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut n: [f64; 9] = [nodata; 9];
                let mut z_factor_array = Vec::with_capacity(rows as usize);
                let mut is_valid: bool; // used to mask out edge effect from fast gaussian
                if is_geogrpahic_coords && z_factor < 0.0 {
                    // calculate a new z-conversion factor
                    for row in 0..rows {
                        let lat = get_y_from_row(input_north, res_y, row);
                        z_factor_array.push(1.0 / (111320.0 * lat.cos()));
                    }
                } else {
                    if z_factor < 0.0 {
                        z_factor = 1.0;
                    }
                    z_factor_array = vec![z_factor; rows as usize];
                }
                for row in (buffer..(rows-buffer)).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    let mut n_part = 0;
                    let mut s_part = 0f64;
                    let mut sq_part = 0f64;
                    for col in buffer..(columns-buffer) { // avoid including edge effect in LSP calculation
                        is_valid = true;
                        val = nodata;
                        n[8] = inp.get_value(row, col);
                        if n[8] != nodata {
                            n[8] *= z_factor_array[row as usize];
                            for c in 0..8 {
                                if sigma >= 3f64 && inp.get_value(row + (d_y[c] * buffer), col + (d_x[c] * buffer)) == nodata {
                                    // inp from fast gaussian and has nodata within midpoint
                                    is_valid = false;
                                    break
                                }
                                n[c] = inp.get_value(row + d_y[c], col + d_x[c]);
                                if n[c] != nodata {
                                    n[c] *= z_factor_array[row as usize];
                                } else if lsp_func != fn_dme && lsp_func != fn_ruggedness {
                                    n[c] = n[8];
                                }
                            }
                            if is_valid {
                                val = lsp_func(n, res, nodata);
                            } // else val stays nodata

                            if val != nodata {
                                n_part += 1isize;
                                s_part += val;
                                sq_part += val * val;
                                data[col as usize] = val
                            } // else do nothing


                        }
                    }
                    tx1.send((row, data, n_part, s_part, sq_part)).unwrap();
                }
            });
        }



        let mut num = 0;
        let mut sum = 0f64;
        let mut sumsqr = 0f64;
        for r in 0..rows-(2 * buffer) {
            let (row, data, n_part, s_part, sq_part) = rx.recv().expect("Error receiving data from thread.");
            lsp_data.set_row_data(row, data);
            num += n_part;
            sum += s_part;
            sumsqr += sq_part;

            if configurations.verbose_mode {
                progress = (100f64 * r as f64 / (rows - (2*buffer)) as f64) as usize;
                if progress != old_progress {
                    println!("Loop {} of {}. Analysis progress: {}%",s+1, num_steps, progress);
                    old_progress = progress;
                }
            }
        }
        let mean = sum / num as f64;
        let stddev = (sumsqr / num as f64 - (mean * mean)).sqrt();

        drop(smoothed_elev); // Memory requirements: 4.5x

        // Step 3: Update optimized outputs
        if stddev != 0f64 {
            if signature_sites.len() > 0 {
                let (mut lsp_val, mut zlsp, mut zmax): (f64, f64, f64);
                let mut tmp_zsc = Array2D::new(rows, columns, nodata32, nodata32)?; // Memory requirements: 5x
                for row in buffer..rows-buffer {
                    for col in buffer..columns-buffer {
                        lsp_val = lsp_data.get_value(row, col);
                        if lsp_val != nodata {
                            if lsp_func == fn_aspect {
                                zlsp = degrees_diff(lsp_val, mean) / stddev;
                            } else {
                                zlsp = (lsp_val - mean) / stddev;
                            }
                            tmp_zsc.set_value(row, col, zlsp as f32);
                            zmax = output_zsc.get_value(row, col) as f64;
                            if zmax != nodata {
                                if zlsp.abs() > zmax.abs() {
                                    output_lsp.set_value(row, col, lsp_val as f32);
                                    output_scl.set_value(row, col, sigma_actual as f32);
                                    output_zsc.set_value(row, col, zlsp as f32)
                                }
                            } else {
                                output_lsp.set_value(row, col, lsp_val as f32);
                                output_scl.set_value(row, col, sigma_actual as f32);
                                output_zsc.set_value(row, col, zlsp as f32)
                            }
                        }
                    }
                }
                for site_sig in 0..num_sites {
                    let (target_row, target_col) = signature_sites[site_sig];
                    xdata[site_sig].push(sigma_actual);
                    ydata[site_sig].push(lsp_data.get_value(target_row, target_col));
                    zdata[site_sig].push(tmp_zsc.get_value(target_row, target_col) as f64);
                }
            // tmp_zsc drops from memory -> Memory requirements: 4.5x
        } else { // no shapefile given
                let (mut lsp_val, mut zlsp, mut zmax): (f64, f64, f64);
                for row in buffer..rows-buffer {
                    for col in buffer..columns-buffer {
                        lsp_val = lsp_data.get_value(row, col);
                        if lsp_val != nodata {
                            if lsp_func == fn_aspect {
                                zlsp = degrees_diff(lsp_val, mean) / stddev;
                            } else {
                                zlsp = (lsp_val - mean) / stddev;
                            }
                            zmax = output_zsc.get_value(row, col) as f64;
                            if zmax != nodata {
                                if zlsp.abs() > zmax.abs() {
                                    output_lsp.set_value(row, col, lsp_val as f32);
                                    output_scl.set_value(row, col, sigma_actual as f32);
                                    output_zsc.set_value(row, col, zlsp as f32)
                                }
                            } else {
                                output_lsp.set_value(row, col, lsp_val as f32);
                                output_scl.set_value(row, col, sigma_actual as f32);
                                output_zsc.set_value(row, col, zlsp as f32)
                            }
                        }
                    }
                }
            }
        }
    }

    let elapsed_time = get_formatted_elapsed_time(start);

    drop(integral); // Memory requirements: 3.5x
    drop(integral_n); // Memory requirements: 3x
    drop(input); // Memory requirements: 2x

    if signature_sites.len() > 0 {
        let f = File::create(output_points_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>GSS Signature</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>GSS Signature</h1>"#
                .as_bytes(),
        )?;

        writer
            .write_all((format!("<p><strong>Input DEM</strong>: {}<br>", input_file)).as_bytes())?;

        writer.write_all(("</p>").as_bytes())?;

        let multiples = xdata.len() > 2 && xdata.len() < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Sigma (cells)".to_string(),
            y_axis_label: format!("{} value", lsp_fmt).to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        let f = File::create(output_points_zscore_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>GSS Z-score Signature</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>GSS Z-score Signature</h1>"#
                .as_bytes(),
        )?;

        writer
            .write_all((format!("<p><strong>Input DEM</strong>: {}<br>", input_file)).as_bytes())?;

        writer.write_all(("</p>").as_bytes())?;

        let multiples = xdata.len() > 2 && xdata.len() < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: zdata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Sigma (cells)".to_string(),
            y_axis_label: format!("{} (z-score)", lsp_fmt).to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();
    }

    // Update output configs
    let mut lsp_raster =
        Raster::initialize_from_array2d(&output_file, &configs, &output_lsp);
    drop(output_lsp);
    lsp_raster.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool", tool_name
    ));
    lsp_raster.add_metadata_entry(format!("Input file: {}", input_file));
    lsp_raster.add_metadata_entry(format!("Initial sigma: {}", sigma_i));
    lsp_raster.add_metadata_entry(format!("Step size: {}", step));
    lsp_raster.add_metadata_entry(format!("Number of steps: {}", num_steps));
    lsp_raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

    if configurations.verbose_mode {
        println!("Writing LSP data...")
    };
    let _ = match lsp_raster.write() {
        Ok(_) => (),
        Err(e) => return Err(e),
    };
    drop(lsp_raster);

    let mut scl_raster =
        Raster::initialize_from_array2d(&output_scale_file, &configs, &output_scl);
    drop(output_scl);
    scl_raster.configs.data_type = DataType::F32;
    scl_raster.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool", tool_name
    ));
    scl_raster.add_metadata_entry(format!("Input file: {}", input_file));
    scl_raster.add_metadata_entry(format!("Initial sigma: {}", sigma_i));
    scl_raster.add_metadata_entry(format!("Step size: {}", step));
    scl_raster.add_metadata_entry(format!("Number of steps: {}", num_steps));
    scl_raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

    if configurations.verbose_mode {
        println!("Writing scale data...")
    };
    let _ = match scl_raster.write() {
        Ok(_) => (),
        Err(e) => return Err(e),
    };
    drop(scl_raster);

    let mut zsc_raster =
        Raster::initialize_from_array2d(&output_zscore_file, &configs, &output_zsc);
    drop(output_zsc);
    zsc_raster.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool", tool_name
    ));
    zsc_raster.add_metadata_entry(format!("Input file: {}", input_file));
    zsc_raster.add_metadata_entry(format!("Initial sigma: {}", sigma_i));
    zsc_raster.add_metadata_entry(format!("Step size: {}", step));
    zsc_raster.add_metadata_entry(format!("Number of steps: {}", num_steps));
    zsc_raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

    if configurations.verbose_mode {
        println!("Writing z-score data...")
    };
    let _ = match zsc_raster.write() {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    if configurations.verbose_mode {
        println!("Output files written.");
        println!("{}",&format!("Elapsed Time (excluding I/O): {}", elapsed_time));
    }

    Ok(())
}

///////////////
// 6 | 7 | 0 //
// 5 | 8 | 1 //
// 4 | 3 | 2 //
///////////////

fn fn_anisotropy(n: [f64; 9], _r:f64, nodata:f64) -> f64 {
    let (full_delta, ns_delta, ew_delta, nesw_delta, nwse_delta): (f64, f64, f64, f64, f64);
    let (mut nn, mut s) = (0usize, 0f64);
    for i in 0..n.len() {
        if n[i] != nodata {
            nn += 1;
            s += n[i];
        }
    }
    if nn > 0 {
        full_delta = n[8] - (s / (nn as f64));
        // N-S pane
        s = n[8] + (n[7] * (n[7]!=nodata) as usize as f64) + (n[3] * (n[3]!=nodata) as usize as f64);
        nn = 1 + (1 * (n[7]!=nodata) as usize) + (1 * (n[3]!=nodata) as usize);
        ns_delta = n[8] - (s / (nn as f64)) - full_delta;
        // E-W pane
        s = n[8] + (n[5] * (n[5]!=nodata) as usize as f64) + (n[1] * (n[1]!=nodata) as usize as f64);
        nn = 1 + (1 * (n[5]!=nodata) as usize) + (1 * (n[1]!=nodata) as usize);
        ew_delta = n[8] - (s / (nn as f64)) - full_delta;
        // NE-SW pane
        s = n[8] + (n[0] * (n[0]!=nodata) as usize as f64) + (n[4] * (n[4]!=nodata) as usize as f64);
        nn = 1 + (1 * (n[0]!=nodata) as usize) + (1 * (n[4]!=nodata) as usize);
        nesw_delta = n[8] - (s / (nn as f64)) - full_delta;
        // NW-SE pane
        s = n[8] + (n[6] * (n[6]!=nodata) as usize as f64) + (n[2] * (n[2]!=nodata) as usize as f64);
        nn = 1 + (1 * (n[6]!=nodata) as usize) + (1 * (n[2]!=nodata) as usize);
        nwse_delta = n[8] - (s / (nn as f64)) - full_delta;

        //final value
        (((ns_delta*ns_delta) + (ew_delta*ew_delta) + (nesw_delta*nesw_delta) + (nwse_delta*nwse_delta)) / 4f64).sqrt()

    } else {
        nodata
    }
}

fn fn_aspect(n: [f64; 9], r: f64, nodata:f64) -> f64 {
    let eight_grid_res = r * 8f64;
    let mut fx = (n[2] - n[4] + 2f64 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
    let fy = (n[6] - n[4] + 2f64 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;

    if fx + fy != 0f64 { // slope is greater than zero
        if fx == 0f64 {
            fx = 0.00001f64;
        }
        180f64 - ((fy / fx).atan()).to_degrees()
            + 90f64 * (fx / (fx).abs())
    } else {
        nodata
    }
}

fn fn_dme(n: [f64; 9], _r: f64, nodata: f64) -> f64 {
    let (mut nn, mut s) = (0usize, 0f64);
    for i in 0..n.len() {
        if n[i] != nodata {
            nn += 1;
            s += n[i];
        }
    }
    n[8] - (s / nn as f64)
}

fn fn_eastness(n: [f64; 9], r: f64, nodata: f64) -> f64 {
    let a = fn_aspect(n, r, nodata);
    if a != nodata {
        a.to_radians().sin()
    } else {
        nodata
    }
}

fn fn_elevation(n: [f64; 9], _r: f64, _nodata: f64) -> f64 {
    n[8]
}

fn fn_hillshade(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let eight_grid_res = r * 8f64;
    let fy = (n[6] - n[4] + 2f64 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
    let fx = (n[2] - n[4] + 2f64 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
    let mut tan_slope = (fx * fx + fy * fy).sqrt();
    if tan_slope < 0.00017f64 {
        tan_slope = 0.00017f64;
    }
    let aspect = if fx != 0f64 {
        PI - ((fy / fx).atan()) + (PI / 2f64) * (fx / (fx).abs())
    } else {
        PI
    };
    // sin_theta = 30f64.to_radians().sin();
    // cos_theta = 30f64.to_radians().sin();
    // azimuth = (315f64-90f64).to_radians();
    let term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
    let term2 = 0.49999999999999994f64 / tan_slope;
    let term3 = 0.8660254037844387f64 * (3.9269908169872414f64 - aspect).sin();
    let mut v = (term1 * (term2 - term3)) * 32767f64;
    v = v * (v > 0f64) as usize as f64;
    v.round()
}

fn fn_mean_curvature(n: [f64;9], r: f64, _nodata: f64) -> f64 {
    let cell_size_times2 = r * 2f64;
    let cell_size_sqrd = r * r;
    let four_times_cell_size_sqrd = cell_size_sqrd * 4f64;
    let zx = (n[1] - n[5]) / cell_size_times2;
    let zy = (n[7] - n[3]) / cell_size_times2;
    let zxx = (n[1] - 2f64 * n[8] + n[5]) / cell_size_sqrd;
    let zyy = (n[7] - 2f64 * n[8] + n[3]) / cell_size_sqrd;
    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
    let zx2 = zx * zx;
    let zy2 = zy * zy;
    let p = zx2 + zy2;
    let q = p + 1f64;
    if p > 0f64 {
        (((zxx * zx2 + 2f64 * zxy * zx * zy + zyy * zy2) / (p * q.powf(1.5f64))) * 100f64)
        +
        (((zxx * zy2 - 2.0f64 * zxy * zx * zy + zyy * zx2) / (p * q.sqrt())) * 100f64)
        / 2f64
    } else {
        0f64
    }
}

fn fn_northness(n: [f64; 9], r: f64, nodata:f64) -> f64 {
    let a = fn_aspect(n, r, nodata);
    if a != nodata {
        a.to_radians().cos()
    } else {
        nodata
    }
}

fn fn_plan_curvature(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let cell_size_times2 = r * 2f64;
    let cell_size_sqrd = r * r;
    let four_times_cell_size_sqrd = cell_size_sqrd * 4f64;
    let zx = (n[1] - n[5]) / cell_size_times2;
    let zy = (n[7] - n[3]) / cell_size_times2;
    let zxx = (n[1] - 2f64 * n[8] + n[5]) / cell_size_sqrd;
    let zyy = (n[7] - 2f64 * n[8] + n[3]) / cell_size_sqrd;
    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
    let zx2 = zx * zx;
    let zy2 = zy * zy;
    let p = zx2 + zy2;
    if p > 0f64 {
        ((zxx * zy2 - 2f64 * zxy * zx * zy + zyy * zx2)
            / p.powf(1.5f64)) * 100f64
    } else {
        0f64
    }
}

fn fn_prof_curvature(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let cell_size_times2 = r * 2f64;
    let cell_size_sqrd = r * r;
    let four_times_cell_size_sqrd = cell_size_sqrd * 4f64;
    let zx = (n[1] - n[5]) / cell_size_times2;
    let zy = (n[7] - n[3]) / cell_size_times2;
    let zxx = (n[1] - 2f64 * n[8] + n[5]) / cell_size_sqrd;
    let zyy = (n[7] - 2f64 * n[8] + n[3]) / cell_size_sqrd;
    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
    let zx2 = zx * zx;
    let zy2 = zy * zy;
    let p = zx2 + zy2;
    let q = p + 1f64;
    if p > 0f64 {
        ((zxx * zx2 + 2f64 * zxy * zx * zy + zyy * zy2)
                / (p * q.powf(1.5f64))) * 100f64
    } else {
        0f64
    }
}

fn fn_ruggedness(n: [f64; 9], _r: f64, nodata: f64) -> f64 {
    let mut nn = 0usize;
    let mut ss = 0f64;
    for i in 0..8 {
        if n[i] != nodata {
            ss += (n[8] - n[i]) * (n[8] - n[i]);
            nn += 1;
        }
    }
    if nn > 0 {
        (ss / nn as f64).sqrt()
    } else {
        0f64
    }
}

fn fn_slope(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let eight_grid_res = r * 8f64;
    let fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
    let fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
    (fx * fx + fy * fy).sqrt() * 100f64
}

fn fn_tan_curvature(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let cell_size_times2 = r * 2f64;
    let cell_size_sqrd = r * r;
    let four_times_cell_size_sqrd = cell_size_sqrd * 4f64;
    let zx = (n[1] - n[5]) / cell_size_times2;
    let zy = (n[7] - n[3]) / cell_size_times2;
    let zxx = (n[1] - 2f64 * n[8] + n[5]) / cell_size_sqrd;
    let zyy = (n[7] - 2f64 * n[8] + n[3]) / cell_size_sqrd;
    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
    let zx2 = zx * zx;
    let zy2 = zy * zy;
    let p = zx2 + zy2;
    let q = p + 1f64;
    if p > 0f64 {
        ((zxx * zy2 + 2f64 * zxy * zx * zy + zyy * zx2)
            / (p * q.sqrt())) * 100f64
    } else {
        0f64
    }
}

fn fn_total_curvature(n: [f64; 9], r: f64, _nodata: f64) -> f64 {
    let cell_size_sqrd = r * r;
    let four_times_cell_size_sqrd = cell_size_sqrd * 4f64;
    let zxx = (n[1] - 2f64 * n[8] + n[5]) / cell_size_sqrd;
    let zyy = (n[7] - 2f64 * n[8] + n[3]) / cell_size_sqrd;
    let zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
    (zxx * zxx + 2.0f64 * zxy * zxy + zyy * zyy) * 100f64
}

fn degrees_diff(a1:f64, a2:f64) -> f64 {
    let dmin = (a1-a2).min(a2-a1);
    dmin.abs().min((dmin+360f64).abs())
}

fn get_y_from_row(north:f64, res_y:f64, row: isize) -> f64 {
    north - (res_y / 2f64) - (row as f64 * res_y)
}
