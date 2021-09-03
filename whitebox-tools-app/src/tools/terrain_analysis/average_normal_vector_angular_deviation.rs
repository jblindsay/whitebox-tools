/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/01/2019
Last Modified: 03/09/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool characterizes the spatial distribution of the average normal vector angular deviation, a measure of
/// surface roughness. Working in the field of 3D printing, Ko et al. (2016) defined a measure of surface roughness
/// based on quantifying the angular deviations in the direction of the normal vector of a real surface from its ideal
/// (i.e. smoothed) form. This measure of surface complexity is therefore in units of degrees. Specifically, roughness
/// is defined in this study as the neighborhood-averaged difference in the normal vectors of the original DEM and a
/// smoothed DEM surface. Smoothed surfaces are derived by applying a Gaussian blur of the same size as the
/// neighborhood (`--filter`).
///
/// The `MultiscaleRoughness` tool calculates the same measure of surface roughness, except that it is designed to
/// work with multiple spatial scales.
///
/// # Reference
/// Ko, M., Kang, H., ulrim Kim, J., Lee, Y., & Hwang, J. E. (2016, July). How to measure quality of affordable 3D
/// printing: Cultivating quantitative index in the user community. In International Conference on Human-Computer
/// Interaction (pp. 116-121). Springer, Cham.
///
/// Lindsay, J. B., & Newman, D. R. (2018). Hyper-scale analysis of surface roughness. PeerJ Preprints, 6, e27110v1.
///
/// # See Also
/// `MultiscaleRoughness`, `SphericalStdDevOfNormals`, `CircularVarianceOfAspect`
pub struct AverageNormalVectorAngularDeviation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AverageNormalVectorAngularDeviation {
    pub fn new() -> AverageNormalVectorAngularDeviation {
        // public constructor
        let name = "AverageNormalVectorAngularDeviation".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the circular variance of aspect at a scale for a DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Raster File".to_owned(),
            flags: vec!["--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Filter Dimension".to_owned(),
            flags: vec!["--filter".to_owned()],
            description: "Size of the filter kernel.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --out_mag=roughness_mag.tif --out_scale=roughness_scale.tif --min_scale=1 --max_scale=1000 --step=5", short_exe, name).replace("*", &sep);

        AverageNormalVectorAngularDeviation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AverageNormalVectorAngularDeviation {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut filter_size = 11usize;
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
            } else if flag_val == "-filter" {
                filter_size = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if filter_size < 3 {
            filter_size = 3;
        }

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size as f64 / 2f64).floor() == (filter_size as f64 / 2f64) {
            filter_size += 1;
        }

        let midpoint = (filter_size as f64 / 2f64).floor() as isize;
        let mut progress: usize;
        let mut old_progress: usize = 1;

        if input_file.is_empty() || output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Either the input or output file were not specified correctly.",
            ));
        }

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let configs = input.configs.clone();
        let rows = configs.rows as isize;
        let columns = configs.columns as isize;
        let nodata = configs.nodata;
        let is_geographic_coords = input.is_in_geographic_coordinates();

        // Smooth the DEM
        let mut smoothed_dem = input.get_data_as_array2d();
        if verbose {
            println!("Smoothing the input DEM...");
        }
        let mut sigma = (midpoint as f64 + 0.5) / 3f64;
        if sigma < 1.0 {
            sigma = 1.0;
        }
        if sigma < 1.8 {
            //&& filter_size > 3 {
            let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * f64::consts::PI).sqrt() * sigma);
            let two_sigma_sqr_d = 2.0 * sigma * sigma;

            // figure out the size of the filter
            let mut filter_size_smooth = 0;
            let mut weight: f64;
            for i in 0..250 {
                weight = recip_root_2_pi_times_sigma_d
                    * (-1.0 * ((i * i) as f64) / two_sigma_sqr_d).exp();
                if weight <= 0.001 {
                    filter_size_smooth = i * 2 + 1;
                    break;
                }
            }

            // the filter dimensions must be odd numbers such that there is a middle pixel
            if filter_size_smooth % 2 == 0 {
                filter_size_smooth += 1;
            }

            if filter_size_smooth < 3 {
                filter_size_smooth = 3;
            }

            let num_pixels_in_filter = filter_size_smooth * filter_size_smooth;
            let mut d_x = vec![0isize; num_pixels_in_filter];
            let mut d_y = vec![0isize; num_pixels_in_filter];
            let mut weights = vec![0.0; num_pixels_in_filter];

            // fill the filter d_x and d_y values and the distance-weights
            let midpoint_smoothed: isize = (filter_size_smooth as f64 / 2f64).floor() as isize + 1;
            let mut a = 0;
            let (mut x, mut y): (isize, isize);
            for row in 0..filter_size {
                for col in 0..filter_size {
                    x = col as isize - midpoint_smoothed;
                    y = row as isize - midpoint_smoothed;
                    d_x[a] = x;
                    d_y[a] = y;
                    weight = recip_root_2_pi_times_sigma_d
                        * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                    weights[a] = weight;
                    a += 1;
                }
            }

            let d_x = Arc::new(d_x);
            let d_y = Arc::new(d_y);
            let weights = Arc::new(weights);

            let mut num_procs = num_cpus::get() as isize;
            let configs = whitebox_common::configs::get_configs()?;
            let max_procs = configs.max_procs;
            if max_procs > 0 && max_procs < num_procs {
                num_procs = max_procs;
            }
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let d_x = d_x.clone();
                let d_y = d_y.clone();
                let weights = weights.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut sum, mut z_final): (f64, f64);
                    let mut z: f64;
                    let mut zn: f64;
                    let (mut x, mut y): (isize, isize);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input.get_value(row, col);
                            if z != nodata {
                                sum = 0.0;
                                z_final = 0.0;
                                for a in 0..num_pixels_in_filter {
                                    x = col + d_x[a];
                                    y = row + d_y[a];
                                    zn = input.get_value(y, x);
                                    if zn != nodata {
                                        sum += weights[a];
                                        z_final += weights[a] * zn;
                                    }
                                }
                                if sum > 0.0 {
                                    data[col as usize] = z_final / sum;
                                }
                            }
                        }

                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            for _ in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                smoothed_dem.set_row_data(data.0, data.1);
            }
        } else {
            // use a fast almost Gaussian filter for larger smoothing operations.
            let n = 4;
            let w_ideal = (12f64 * sigma * sigma / n as f64 + 1f64).sqrt();
            let mut wl = w_ideal.floor() as isize;
            if wl % 2 == 0 {
                wl -= 1;
            } // must be an odd integer
            let wu = wl + 2;
            let m = ((12f64 * sigma * sigma
                - (n * wl * wl) as f64
                - (4 * n * wl) as f64
                - (3 * n) as f64)
                / (-4 * wl - 4) as f64)
                .round() as isize;

            let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
            let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1)?;
            let mut val: f64;

            let mut sum: f64;
            let mut sum_n: i32;
            let mut i_prev: f64;
            let mut n_prev: i32;
            let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
            let mut num_cells: i32;

            for iteration_num in 0..n {
                if verbose {
                    println!("Loop {} of {}", iteration_num + 1, n);
                }

                let midpoint = if iteration_num <= m {
                    (wl as f64 / 2f64).floor() as isize
                } else {
                    (wu as f64 / 2f64).floor() as isize
                };

                if iteration_num == 0 {
                    // First iteration
                    // Create the integral images.
                    for row in 0..rows {
                        sum = 0f64;
                        sum_n = 0;
                        for col in 0..columns {
                            val = input.get_value(row, col);
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
                        }
                    }
                } else {
                    // Create the integral image based on previous iteration output.
                    // We don't need to recalculate the num_cells integral image.
                    for row in 0..rows {
                        sum = 0f64;
                        for col in 0..columns {
                            val = smoothed_dem.get_value(row, col);
                            if val == nodata {
                                val = 0f64;
                            }
                            sum += val;
                            if row > 0 {
                                i_prev = integral.get_value(row - 1, col);
                                integral.set_value(row, col, sum + i_prev);
                            } else {
                                integral.set_value(row, col, sum);
                            }
                        }
                    }
                }

                // Perform Filter
                for row in 0..rows {
                    y1 = row - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    y2 = row + midpoint;
                    if y2 >= rows {
                        y2 = rows - 1;
                    }

                    for col in 0..columns {
                        if input.get_value(row, col) != nodata {
                            x1 = col - midpoint - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }
                            x2 = col + midpoint;
                            if x2 >= columns {
                                x2 = columns - 1;
                            }

                            num_cells = integral_n[(y2, x2)] + integral_n[(y1, x1)]
                                - integral_n[(y1, x2)]
                                - integral_n[(y2, x1)];
                            if num_cells > 0 {
                                sum = integral[(y2, x2)] + integral[(y1, x1)]
                                    - integral[(y1, x2)]
                                    - integral[(y2, x1)];
                                smoothed_dem.set_value(row, col, sum / num_cells as f64);
                            } else {
                                // should never hit here since input(row, col) != nodata above, therefore, num_cells >= 1
                                smoothed_dem.set_value(row, col, input.get_value(row, col));
                            }
                        }
                    }
                }
            }
        }

        // drop(input);

        // Calculate the difference in local normal vectors from the original and smoothed DEM.
        let eight_grid_res = configs.resolution_x * 8.0;
        let mut z_factor = 1f64;
        if is_geographic_coords {
            // calculate a new z-conversion factor
            let mut mid_lat = (configs.north - configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (111320.0 * mid_lat.cos());
            }
        }

        let mut num_procs = num_cpus::get() as isize;
        let configurations = whitebox_common::configs::get_configs()?;
        let max_procs = configurations.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let smoothed_dem = Arc::new(smoothed_dem);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let smoothed_dem = smoothed_dem.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut n1: [f64; 8] = [0.0; 8];
                let mut n2: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let mut dot_product: f64;
                let (mut a1, mut b1): (f64, f64);
                let (mut a2, mut b2): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![0f64; columns as usize];
                    for col in 0..columns {
                        z = smoothed_dem.get_value(row, col);
                        if z != nodata {
                            for c in 0..8 {
                                n1[c] = input.get_value(row + dy[c], col + dx[c]);
                                if n1[c] != nodata {
                                    n1[c] = n1[c] * z_factor;
                                } else {
                                    n1[c] = input.get_value(row, col) * z_factor;
                                }
                                n2[c] = smoothed_dem.get_value(row + dy[c], col + dx[c]);
                                if n2[c] != nodata {
                                    n2[c] = n2[c] * z_factor;
                                } else {
                                    n2[c] = z * z_factor;
                                }
                            }
                            a1 = -(n1[2] - n1[4] + 2f64 * (n1[1] - n1[5]) + n1[0] - n1[6])
                                / eight_grid_res;
                            b1 = -(n1[6] - n1[4] + 2f64 * (n1[7] - n1[3]) + n1[0] - n1[2])
                                / eight_grid_res;

                            a2 = -(n2[2] - n2[4] + 2f64 * (n2[1] - n2[5]) + n2[0] - n2[6])
                                / eight_grid_res;
                            b2 = -(n2[6] - n2[4] + 2f64 * (n2[7] - n2[3]) + n2[0] - n2[2])
                                / eight_grid_res;

                            dot_product = a1 * a2 + b1 * b2 + 1.0;

                            data[col as usize] = (dot_product
                                / ((a1 * a1 + b1 * b1 + 1.0) * (a2 * a2 + b2 * b2 + 1.0)).sqrt())
                            .acos()
                            .to_degrees();
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut angular_diff: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?;
        // let mut output = Raster::initialize_using_config(&output_file, &configs);
        // output.configs.data_type = DataType::F32;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            angular_diff.set_row_data(data.0, data.1);
            // output.set_row_data(data.0, angular_diff.get_row_data(data.0));
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating angular deviation data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // convert to integral images
        let mut i_n: Array2D<u32> = Array2D::new(rows, columns, 1, 0)?;
        let mut sum: f64;
        let mut sumn: u32;
        for row in 0..rows {
            if row > 0 {
                sum = 0f64;
                sumn = 0u32;
                for col in 0..columns {
                    sum += angular_diff.get_value(row, col);
                    if smoothed_dem.get_value(row, col) == nodata {
                        // it's either nodata or a flag cell in the DEM.
                        i_n.decrement(row, col, 1);
                    }
                    sumn += i_n.get_value(row, col);
                    angular_diff.set_value(row, col, sum + angular_diff.get_value(row - 1, col));
                    i_n.set_value(row, col, sumn + i_n.get_value(row - 1, col));
                }
            } else {
                if smoothed_dem.get_value(0, 0) == nodata {
                    i_n.set_value(0, 0, 0);
                }
                for col in 1..columns {
                    angular_diff.increment(row, col, angular_diff.get_value(row, col - 1));
                    i_n.increment(row, col, i_n.get_value(row, col - 1));
                    if smoothed_dem.get_value(row, col) == nodata {
                        // it's either nodata or a flag cell in the DEM.
                        i_n.decrement(row, col, 1);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating integral images: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let angular_diff = Arc::new(angular_diff);
        let i_n = Arc::new(i_n);
        let (tx2, rx2) = mpsc::channel();
        for tid in 0..num_procs {
            let angular_diff = angular_diff.clone();
            let smoothed_dem = smoothed_dem.clone();
            let i_n = i_n.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                let mut n: f64;
                let mut sum: f64;
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    y1 = row - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    if y1 >= rows {
                        y1 = rows - 1;
                    }

                    y2 = row + midpoint;
                    if y2 < 0 {
                        y2 = 0;
                    }
                    if y2 >= rows {
                        y2 = rows - 1;
                    }
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = smoothed_dem.get_value(row, col);
                        if z != nodata {
                            x1 = col - midpoint - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }
                            if x1 >= columns {
                                x1 = columns - 1;
                            }

                            x2 = col + midpoint;
                            if x2 < 0 {
                                x2 = 0;
                            }
                            if x2 >= columns {
                                x2 = columns - 1;
                            }
                            n = (i_n.get_value(y2, x2) + i_n.get_value(y1, x1)
                                - i_n.get_value(y1, x2)
                                - i_n.get_value(y2, x1)) as f64;
                            if n > 0f64 {
                                sum = angular_diff.get_value(y2, x2)
                                    + angular_diff.get_value(y1, x1)
                                    - angular_diff.get_value(y1, x2)
                                    - angular_diff.get_value(y2, x1);
                                data[col as usize] = sum / n;
                            }
                            // data[col as usize] = angular_diff.get_value(row, col).to_degrees();
                        }
                    }

                    match tx2.send((row, data)) {
                        Ok(_) => {}
                        Err(_) => {
                            println!(
                                "Error sending data from thread {} processing row {}.",
                                tid, row
                            );
                        }
                    }
                }
            });
        }

        let mut output = Raster::initialize_using_config(&output_file, &configs);
        output.configs.data_type = DataType::F32;

        for row in 0..rows {
            match rx2.recv() {
                Ok(data) => {
                    output.set_row_data(data.0, data.1);
                    // output.set_row_data(data.0, smoothed_dem.get_row_data(data.0));
                    // for col in 0..columns {
                    //     output.set_value(data.0, col, angular_diff.get_value(data.0, col) as f64);
                    // }
                }
                Err(_) => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Error in receiving data from thread.",
                    ));
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "muted_spectrum.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size: {}", filter_size));
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
}
