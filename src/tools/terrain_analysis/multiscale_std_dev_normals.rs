/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 05/06/2019
Last Modified: 05/06/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

pub struct MultiscaleStdDevNormals {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultiscaleStdDevNormals {
    pub fn new() -> MultiscaleStdDevNormals {
        // public constructor
        let name = "MultiscaleStdDevNormals".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates surface roughness over a range of spatial scales.".to_string();

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
            name: "Output Roughness Magnitude File".to_owned(),
            flags: vec!["--out_mag".to_owned()],
            description: "Output raster roughness magnitude file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Roughness Scale File".to_owned(),
            flags: vec!["--out_scale".to_owned()],
            description: "Output raster roughness scale file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Search Neighbourhood Radius (grid cells)".to_owned(),
            flags: vec!["--min_scale".to_owned()],
            description: "Minimum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Search Neighbourhood Radius (grid cells)".to_owned(),
            flags: vec!["--max_scale".to_owned()],
            description: "Maximum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Step Size".to_owned(),
            flags: vec!["--step".to_owned()],
            description: "Step size as any positive non-zero integer.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --out_mag=roughness_mag.tif --out_scale=roughness_scale.tif --min_scale=1 --max_scale=1000 --step=5", short_exe, name).replace("*", &sep);

        MultiscaleStdDevNormals {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultiscaleStdDevNormals {
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
        let mut output_mag_file = String::new();
        let mut output_scale_file = String::new();
        let mut min_scale = 1isize;
        let mut max_scale = 100isize;
        let mut step = 1isize;
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            } else if flag_val == "-out_mag" {
                output_mag_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-out_scale" {
                output_scale_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-min_scale" {
                min_scale = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
                if min_scale < 1 {
                    min_scale = 1;
                }
            } else if flag_val == "-max_scale" {
                max_scale = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            } else if flag_val == "-step" {
                step = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            }
        }

        if max_scale < min_scale {
            let ms = min_scale;
            min_scale = max_scale;
            max_scale = ms;
        }

        if max_scale == min_scale {
            max_scale += 1;
        }

        if step < 1 {
            step = 1;
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_mag_file.contains(&sep) && !output_mag_file.contains("/") {
            output_mag_file = format!("{}{}", working_directory, output_mag_file);
        }
        if !output_scale_file.contains(&sep) && !output_scale_file.contains("/") {
            output_scale_file = format!("{}{}", working_directory, output_scale_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut z_factor = 1f64;
        if input.is_in_geographic_coordinates() {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (113200.0 * mid_lat.cos());
            }
        }

        let num_procs = num_cpus::get() as isize;

        let mut output_mag = Raster::initialize_using_file(&output_mag_file, &input);
        if output_mag.configs.data_type != DataType::F32 && output_mag.configs.data_type != DataType::F64 {
            output_mag.configs.data_type = DataType::F32;
        }   
        let mut output_scale = Raster::initialize_using_file(&output_scale_file, &input);
        output_scale.configs.data_type = DataType::I16;

        let count = (min_scale..max_scale).filter(|s| (s - min_scale) % step == 0).count();
        let mut loop_num = 1;
        for midpoint in (min_scale..max_scale).filter(|s| (s - min_scale) % step == 0) {
            // .step_by(step) { once step_by is stabilized
            println!("Loop {} / {}", loop_num, count);

            let filter_size = midpoint * 2 + 1;

            ////////////////////////////////////////
            // Smooth the DEM using Gaussian blur //
            ////////////////////////////////////////
            let mut smoothed_dem = input.get_data_as_array2d();
            let sigma = (midpoint as f64 - 0.5) / 3f64;
            if sigma < 1.8 && filter_size > 3 {
                let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * f64::consts::PI).sqrt() * sigma);
                let two_sigma_sqr_d = 2.0 * sigma * sigma;

                // figure out the size of the filter
                let mut filter_size_smooth = 0;
                let mut weight: f64;
                for i in 0..250 {
                    weight =
                        recip_root_2_pi_times_sigma_d * (-1.0 * ((i * i) as f64) / two_sigma_sqr_d).exp();
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
                                    data[col as usize] = z_final / sum;
                                }
                            }

                            tx1.send((row, data)).unwrap();
                        }
                    });
                }

                for _ in 0..rows {
                    let data = rx.recv().unwrap();
                    smoothed_dem.set_row_data(data.0, data.1);
                }
            } else if filter_size > 3 {
                // use a fast almost Gaussian filter for larger smoothing operations.
                let n = 4;
                let w_ideal = (12f64 * sigma * sigma / n as f64 + 1f64).sqrt();
                let mut wl = w_ideal.floor() as isize;
                if wl % 2 == 0 {
                    wl -= 1;
                } // must be an odd integer
                let wu = wl + 2;
                let m =
                    ((12f64 * sigma * sigma - (n * wl * wl) as f64 - (4 * n * wl) as f64 - (3 * n) as f64)
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
                    // Warning: midpoint shadows the loop iterator name here.
                    let midpoint = if iteration_num < m {
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
                                    smoothed_dem.set_value(row, col, 0f64);
                                }
                            }
                        }
                    }
                }
            }



            ///////////////////////////
            // Calculate the normals //
            ///////////////////////////
            let resx = input.configs.resolution_x;
            let resy = input.configs.resolution_y;
            let smoothed_dem = Arc::new(smoothed_dem);
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let smoothed_dem = smoothed_dem.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                    let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                    let mut n: [f64; 8] = [0.0; 8];
                    let mut z: f64;
                    let (mut fx, mut fy): (f64, f64);
                    let fz = 1f64;
                    let fz_sqrd = fz * fz;
                    let mut magnitude: f64;
                    let resx8 = resx * 8f64;
                    let resy8 = resy * 8f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut xdata = vec![0f64; columns as usize];
                        let mut ydata = vec![0f64; columns as usize];
                        let mut zdata = vec![0f64; columns as usize];
                        for col in 0..columns {
                            z = smoothed_dem.get_value(row, col);
                            if z != nodata {
                                for c in 0..8 {
                                    n[c] = smoothed_dem[(row + dy[c], col + dx[c])];
                                    if n[c] != nodata {
                                        n[c] = n[c] * z_factor;
                                    } else {
                                        n[c] = z * z_factor;
                                    }
                                }
                                fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / resx8;
                                fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / resy8;
                                magnitude = (fx * fx + fy * fy + fz_sqrd).sqrt();
                                if magnitude > 0f64 {
                                    xdata[col as usize] = fx / magnitude;
                                    ydata[col as usize] = fy / magnitude;
                                    zdata[col as usize] = fz / magnitude;
                                } else {
                                    xdata[col as usize] = 0f64;
                                    ydata[col as usize] = 0f64;
                                    zdata[col as usize] = 1f64;
                                }
                            }
                        }
                        tx.send((row, xdata, ydata, zdata)).unwrap();
                    }
                });
            }

            let mut xc: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?;
            let mut yc: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?;
            let mut zc: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?; 
            for _ in 0..rows {
                let data = rx.recv().unwrap();
                xc.set_row_data(data.0, data.1);
                yc.set_row_data(data.0, data.2);
                zc.set_row_data(data.0, data.3);
            }



            ////////////////////////////////////////
            // Convert normals to integral images //
            ////////////////////////////////////////
            let mut i_n: Array2D<u32> = Array2D::new(rows, columns, 1, 0)?;
            let (mut sumx, mut sumy, mut sumz): (f64, f64, f64);
            let mut sumn: u32;
            for row in 0..rows {
                if row > 0 {
                    sumx = 0f64;
                    sumy = 0f64;
                    sumz = 0f64;
                    sumn = 0u32;
                    for col in 0..columns {
                        sumx += xc.get_value(row, col);
                        sumy += yc.get_value(row, col);
                        sumz += zc.get_value(row, col);
                        if smoothed_dem.get_value(row, col) == nodata || 
                            (xc.get_value(row, col) == 0f64 && yc.get_value(row, col) == 0f64) {
                            // it's either nodata or a flag cell in the DEM.
                            i_n.decrement(row, col, 1);
                        }
                        sumn += i_n.get_value(row, col);
                        xc.set_value(row, col, sumx + xc.get_value(row-1, col));
                        yc.set_value(row, col, sumy + yc.get_value(row-1, col));
                        zc.set_value(row, col, sumz + zc.get_value(row-1, col));
                        i_n.set_value(row, col, sumn + i_n.get_value(row-1, col));
                    }
                } else {
                    if smoothed_dem.get_value(0, 0) == nodata {
                        i_n.set_value(0, 0, 0);
                    }
                    for col in 1..columns {
                        xc.increment(row, col, xc.get_value(row, col-1));
                        yc.increment(row, col, yc.get_value(row, col-1));
                        zc.increment(row, col, zc.get_value(row, col-1));
                        i_n.increment(row, col, i_n.get_value(row, col-1));
                        if smoothed_dem.get_value(row, col) == nodata || 
                            (xc.get_value(row, col) == 0f64 && yc.get_value(row, col) == 0f64) {
                            // it's either nodata or a flag cell in the DEM.
                            i_n.decrement(row, col, 1);
                        }
                    }
                }
            }



            ////////////////////////////////////////////////////////////////
            // Calculate the spherical standard deviations of the normals //
            ////////////////////////////////////////////////////////////////
            let xc = Arc::new(xc);
            let yc = Arc::new(yc);
            let zc = Arc::new(zc);
            let i_n = Arc::new(i_n);
            let (tx2, rx2) = mpsc::channel();
            for tid in 0..num_procs {
                let xc = xc.clone();
                let yc = yc.clone();
                let zc = zc.clone();
                let smoothed_dem = smoothed_dem.clone();
                let i_n = i_n.clone();
                let tx2 = tx2.clone();
                thread::spawn(move || {
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let mut n: f64;
                    let (mut sumx, mut sumy, mut sumz): (f64, f64, f64);
                    let mut mean: f64;
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
                                    sumx = xc.get_value(y2, x2) + xc.get_value(y1, x1)
                                        - xc.get_value(y1, x2)
                                        - xc.get_value(y2, x1);
                                    sumy = yc.get_value(y2, x2) + yc.get_value(y1, x1)
                                        - yc.get_value(y1, x2)
                                        - yc.get_value(y2, x1);
                                    sumz = zc.get_value(y2, x2) + zc.get_value(y1, x1)
                                        - zc.get_value(y1, x2)
                                        - zc.get_value(y2, x1);
                                    mean = (sumx * sumx + sumy * sumy + sumz * sumz).sqrt() / n;
                                    if mean > 1f64 { 
                                        mean = 1f64; 
                                    }
                                    // data[col as usize] = (2f64 * (1f64 - mean)).sqrt().to_degrees();
                                    data[col as usize] = (-2f64 * mean.ln()).sqrt().to_degrees();
                                }
                            }
                        }

                        match tx2.send((row, data)) {
                            Ok(_) => {},
                            Err(_) => { println!("Error sending data from thread {} processing row {}.", tid, row); },
                        }
                    }
                });
            }

            for _ in 0..rows {
                match rx2.recv() {
                    Ok((row, data)) => {
                        for col in 0..columns {
                            if data[col as usize] > output_mag.get_value(row, col) {
                                output_mag.set_value(row, col, data[col as usize]);
                                output_scale.set_value(row, col, midpoint as f64);
                            }
                        }
                    },
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Error in receiving data from thread.",
                        ));
                    }
                }
            }

            // Update progress
            if verbose {
                progress = (loop_num as f64 / count as f64 * 100f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }

            loop_num += 1;
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output_mag.configs.palette = "blue_white_red.plt".to_string();
        output_mag.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_mag.add_metadata_entry(format!("Input file: {}", input_file));
        output_mag.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_mag.add_metadata_entry(format!("Maximum neighbourhood radius: {}", max_scale));
        output_mag.add_metadata_entry(format!("Step size y: {}", step));
        output_mag.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving magnitude data...")
        };
        let _ = match output_mag.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        output_scale.configs.palette = "spectrum.plt".to_string();
        output_scale.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_scale.add_metadata_entry(format!("Input file: {}", input_file));
        output_scale.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_scale.add_metadata_entry(format!("Maximum neighbourhood radius: {}", max_scale));
        output_scale.add_metadata_entry(format!("Step size: {}", step));
        output_scale.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving scale data...")
        };
        let _ = match output_scale.write() {
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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
