/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: November 23, 2017
Last Modified: Dec. 15, 2017
License: MIT
*/

use num_cpus;
use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::ops::AddAssign;
use std::ops::SubAssign;
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use structures::Array2D;
use time;
use tools::*;

/// This tool implements a modified form of the algorithm described by
///  Sun, Rosin, Martin, and Langbein (2007) Fast and effective feature-preserving
///  mesh denoising.
pub struct FeaturePreservingDenoise {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FeaturePreservingDenoise {
    pub fn new() -> FeaturePreservingDenoise {
        // public constructor
        let name = "FeaturePreservingDenoise".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm.".to_string();

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
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Filter Size".to_owned(),
            flags: vec!["--filter".to_owned()],
            description: "Size of the filter kernel.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Normal Difference Threshold".to_owned(),
            flags: vec!["--norm_diff".to_owned()],
            description: "Maximum difference in normal vectors, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("8.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Iterations".to_owned(),
            flags: vec!["--num_iter".to_owned()],
            description: "Number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Z Conversion Factor".to_owned(),
            flags: vec!["--zfactor".to_owned()],
            description:
                "Optional multiplier for when the vertical and horizontal units are not the same."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --filter=15 --norm_diff=20.0 --num_iter=4",
            short_exe, name
        ).replace("*", &sep);

        FeaturePreservingDenoise {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FeaturePreservingDenoise {
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
        let mut max_norm_diff = 8f64;
        let mut num_iter = 5;
        let mut z_factor = 1f64;

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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-filter" {
                filter_size = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
            } else if flag_val == "-norm_diff" {
                max_norm_diff = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-num_iter" {
                num_iter = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
            } else if flag_val == "-zfactor" {
                z_factor = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if filter_size < 3 {
            filter_size = 3;
        }
        if num_iter < 1 {
            num_iter = 1;
        }
        if max_norm_diff > 90f64 {
            max_norm_diff = 90f64;
        }
        let threshold = max_norm_diff.to_radians().cos();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

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

        let start = time::now();

        // max_norm_diff = max_norm_diff.to_radians();

        if input.is_in_geographic_coordinates() {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (113200.0 * mid_lat.cos());
                println!("It appears that the DEM is in geographic coordinates. The z-factor has been updated: {}.", z_factor);
            }
        }

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        ///////////////////////////////
        // Create the normal vectors //
        ///////////////////////////////
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let eight_grid_res = input.configs.resolution_x * 8f64;
                let mut z: f64;
                let mut zn: f64;
                let (mut a, mut b): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![
                        Normal {
                            a: 0f64,
                            b: 0f64,
                            c: 0f64
                        };
                        columns as usize
                    ];
                    let mut values = [0f64; 9];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            for i in 0..8 {
                                zn = input.get_value(row + dy[i], col + dx[i]);
                                if zn != nodata {
                                    values[i] = zn * z_factor;
                                } else {
                                    values[i] = z * z_factor;
                                }
                            }
                            a = -(values[2] - values[4]
                                + 2f64 * (values[1] - values[5])
                                + values[0]
                                - values[6]);
                            b = -(values[6] - values[4]
                                + 2f64 * (values[7] - values[3])
                                + values[0]
                                - values[2]);
                            data[col as usize] = Normal {
                                a: a,
                                b: b,
                                c: eight_grid_res,
                            };
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let zero_vector = Normal {
            a: 0f64,
            b: 0f64,
            c: 0f64,
        };
        let mut nv: Array2D<Normal> = Array2D::new(rows, columns, zero_vector, zero_vector)?;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            nv.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating normal vectors: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let t1 = time::now();
        println!(
            "{}",
            format!("Calculating normal vectors: {}", t1 - start).replace("PT", "")
        );

        //////////////////////////////////////////////////////////
        // Smooth the normal vector field of the fitted planes. //
        //////////////////////////////////////////////////////////
        let nv = Arc::new(nv);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let nv = nv.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let num_pixels_in_filter = filter_size * filter_size;
                let mut dx = vec![0isize; num_pixels_in_filter];
                let mut dy = vec![0isize; num_pixels_in_filter];

                // fill the filter d_x and d_y values and the distance-weights
                let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
                let mut a = 0;
                for row in 0..filter_size {
                    for col in 0..filter_size {
                        dx[a] = col as isize - midpoint;
                        dy[a] = row as isize - midpoint;
                        a += 1;
                    }
                }
                let mut z: f64;
                let (mut xn, mut yn): (isize, isize);
                let (mut a, mut b, mut c): (f64, f64, f64);
                let mut diff: f64;
                let mut w: f64;
                let mut sum_w: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![
                        Normal {
                            a: 0f64,
                            b: 0f64,
                            c: 0f64
                        };
                        columns as usize
                    ];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            sum_w = 0f64;
                            a = 0f64;
                            b = 0f64;
                            c = 0f64;
                            for n in 0..num_pixels_in_filter {
                                xn = col + dx[n];
                                yn = row + dy[n];
                                if input.get_value(yn, xn) != nodata {
                                    diff =
                                        nv.get_value(row, col).angle_between(nv.get_value(yn, xn));
                                    if diff > threshold {
                                        w = (diff - threshold) * (diff - threshold);
                                        sum_w += w;
                                        a += nv.get_value(yn, xn).a * w;
                                        b += nv.get_value(yn, xn).b * w;
                                        c += nv.get_value(yn, xn).c * w;
                                    }
                                }
                            }

                            a /= sum_w;
                            b /= sum_w;
                            c /= sum_w;
                            data[col as usize] = Normal { a: a, b: b, c: c };
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut nv_smooth: Array2D<Normal> = Array2D::new(rows, columns, zero_vector, zero_vector)?;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            nv_smooth.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Smoothing normal vectors: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let t2 = time::now();
        println!(
            "{}",
            format!("Smoothing normal vectors: {}", t2 - t1).replace("PT", "")
        );

        ///////////////////////////////////////////////////////////////////////////
        // Update the elevations of the DEM based on the smoothed normal vectors //
        ///////////////////////////////////////////////////////////////////////////
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let res_x = input.configs.resolution_x;
        let res_y = input.configs.resolution_y;
        let x = [-res_x, -res_x, -res_x, 0f64, res_x, res_x, res_x, 0f64];
        let y = [-res_y, 0f64, res_y, res_y, res_y, 0f64, -res_y, -res_y];
        let mut w: f64;
        let mut sum_w: f64;
        let mut diff: f64;
        let mut z: f64;
        let (mut xn, mut yn): (isize, isize);
        let mut zn: f64;
        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.set_data_from_raster(&input)?;
        println!("Updating elevations...");
        for loop_num in 0..num_iter {
            println!("Iteration {} of {}...", loop_num + 1, num_iter);

            for row in 0..rows {
                for col in 0..columns {
                    z = output.get_value(row, col);
                    if z != nodata {
                        sum_w = 0f64;
                        z = 0f64;
                        for n in 0..8 {
                            xn = col + dx[n];
                            yn = row + dy[n];
                            zn = output.get_value(yn, xn);
                            if zn != nodata {
                                diff = nv_smooth
                                    .get_value(row, col)
                                    .angle_between(nv_smooth.get_value(yn, xn));
                                if diff > threshold {
                                    w = (diff - threshold) * (diff - threshold);
                                    sum_w += w;
                                    z += -(nv_smooth.get_value(yn, xn).a * x[n]
                                        + nv_smooth.get_value(yn, xn).b * y[n]
                                        - nv_smooth.get_value(yn, xn).c * zn)
                                        / nv_smooth.get_value(yn, xn).c
                                        * w;
                                }
                            }
                        }
                        if sum_w > 0f64 {
                            // this is a division-by-zero safeguard and must be in place.
                            output.set_value(row, col, z / sum_w);
                        } else {
                            output.set_value(row, col, input.get_value(row, col));
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Updating DEM elevations (Loop {} of {}): {}%",
                            loop_num + 1,
                            num_iter,
                            progress
                        );
                        old_progress = progress;
                    }
                }
            }
        }

        // let intercell_break_slope = 60f64.to_radians(); // make user-specified.
        // let res_x = input.configs.resolution_x;
        // let res_y = input.configs.resolution_y;
        // let max_z_diff_ew = intercell_break_slope.tan() * res_x;
        // let max_z_diff_ns = intercell_break_slope.tan() * res_y;
        // let max_z_diff_diag = intercell_break_slope.tan() * (res_x*res_x + res_y*res_y).sqrt();

        // /////////////////////////////////////////////
        // // Fit planes to each grid cell in the DEM //
        // /////////////////////////////////////////////

        // let norm_nodata = Plane { a: -32768f64, b: -32768f64, c: -32768f64, d: -32768f64 };
        // let mut plane_data: Array2D<Plane> = Array2D::new(rows, columns, norm_nodata, norm_nodata)?;

        // let num_procs = num_cpus::get() as isize;
        // let (tx, rx) = mpsc::channel();
        // for tid in 0..num_procs {
        //     let input = input.clone();
        //     let tx = tx.clone();
        //     thread::spawn(move || {
        //         let dx = [ 0, 1, 1, 1, 0, -1, -1, -1, 0 ];
        //         let dy = [ 0, -1, 0, 1, 1, 1, 0, -1, -1 ];

        //         let max_z_diff = [ max_z_diff_ns, max_z_diff_diag, max_z_diff_ew, max_z_diff_diag, max_z_diff_ns, max_z_diff_diag, max_z_diff_ew, max_z_diff_diag, max_z_diff_ns ];

        //         let mut z: f64;
        //         let (mut xn, mut yn, mut zn): (f64, f64, f64);
        //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //             // y = input.get_y_from_row(row);
        //             let mut data = vec![Plane { a: -32768f64, b: -32768f64, c: -32768f64, d: -32768f64 }; columns as usize];
        //             for col in 0..columns {
        //                 // x = input.get_x_from_column(col);
        //                 z = input.get_value(row, col);
        //                 if z != nodata {
        //                     z *= z_factor;
        //                     let mut pt_data: Vec<Vector3<f64>> = Vec::with_capacity(9);
        //                     for i in 0..dx.len() {
        //                         yn = input.get_y_from_row(row + dy[i]);
        //                         xn = input.get_x_from_column(col + dx[i]);
        //                         zn = input.get_value(row + dy[i], col + dx[i]);
        //                         if zn != nodata {
        //                             zn *= z_factor;
        //                         } else {
        //                             zn = z;
        //                         }

        //                         if (zn - z).abs() > max_z_diff[i] {
        //                             // This indicates a very steep inter-cell slope.
        //                             // Don't use this neighbouring cell value to
        //                             // calculate the plane.
        //                             zn = z;
        //                         }

        //                         pt_data.push(Vector3 { x: xn, y: yn, z: zn });
        //                     }
        //                     data[col as usize] = plane_from_points(&pt_data);
        //                 }
        //             }
        //             tx.send((row, data)).unwrap();
        //         }
        //     });
        // }

        // for row in 0..rows {
        //     let data = rx.recv().unwrap();
        //     plane_data.set_row_data(data.0, data.1);

        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Fitting planes: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // let t1 = time::now();
        // println!("{}", format!("Fitting planes: {}", t1 - start).replace("PT", ""));

        // //////////////////////////////////////////////////////////
        // // Smooth the normal vector field of the fitted planes. //
        // //////////////////////////////////////////////////////////
        // let plane_data = Arc::new(plane_data);
        // let (tx, rx) = mpsc::channel();
        // for tid in 0..num_procs {
        //     let input = input.clone();
        //     let plane_data = plane_data.clone();
        //     let tx = tx.clone();
        //     thread::spawn(move || {
        //         let num_pixels_in_filter = filter_size * filter_size;
        //         let mut dx = vec![0isize; num_pixels_in_filter];
        //         let mut dy = vec![0isize; num_pixels_in_filter];

        //         // fill the filter d_x and d_y values and the distance-weights
        //         let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        //         let mut a = 0;
        //         for row in 0..filter_size {
        //             for col in 0..filter_size {
        //                 dx[a] = col as isize - midpoint;
        //                 dy[a] = row as isize - midpoint;
        //                 a += 1;
        //             }
        //         }
        //         let (mut x, mut y, mut z): (f64, f64, f64);
        //         let mut zn: f64;
        //         let mut norm_diff: f64;
        //         let mut p: Plane;
        //         let mut pn: Plane;
        //         let mut p_avg: Plane;
        //         let mut w: f64;
        //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //             y = input.get_y_from_row(row);
        //             let mut data = vec![norm_nodata; columns as usize];
        //             for col in 0..columns {
        //                 x = input.get_x_from_column(col);
        //                 z = input.get_value(row, col);
        //                 if z != nodata {
        //                     p = plane_data.get_value(row, col);
        //                     w = 0f64;
        //                     p_avg = Plane{ a: 0f64, b: 0f64, c: 0f64, d: 0f64 };
        //                     for i in 0..num_pixels_in_filter {
        //                         zn = input.get_value(row + dy[i], col + dx[i]);
        //                         if zn != nodata {
        //                             pn = plane_data.get_value(row + dy[i], col + dx[i]);
        //                             norm_diff = p.angle_between(pn);
        //                             if norm_diff < max_norm_diff {
        //                                 p_avg += pn;
        //                                 w += 1f64;
        //                             }
        //                         }
        //                     }
        //                     if w > 0f64 {
        //                         p_avg /= w;
        //                         p_avg.d = -(p_avg.a * x + p_avg.b * y + p_avg.c * z*z_factor);
        //                         data[col as usize] = p_avg;
        //                     } else {
        //                         data[col as usize] = p;
        //                     }
        //                 }
        //             }
        //             tx.send((row, data)).unwrap();
        //         }
        //     });
        // }

        // let mut smoothed_plane_data: Array2D<Plane> = Array2D::new(rows, columns, norm_nodata, norm_nodata)?;
        // for row in 0..rows {
        //     let data = rx.recv().unwrap();
        //     smoothed_plane_data.set_row_data(data.0, data.1);

        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Smoothing normal vectors: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // let t2 = time::now();
        // println!("{}", format!("Smoothing normal vectors: {}", t2 - t1).replace("PT", ""));

        // /////////////////////
        // // Smooth the DEM. //
        // /////////////////////
        // let mut output = Raster::initialize_using_file(&output_file, &input);

        // // let smoothed_plane_data = Arc::new(smoothed_plane_data);
        // // let mut z: f64;
        // // let (mut fx, mut fy): (f64, f64);
        // // let (mut tan_slope, mut aspect): (f64, f64);
        // // let (mut term1, mut term2, mut term3): (f64, f64, f64);
        // // let mut azimuth = 315.0f64;
        // // let mut altitude = 30.0f64;
        // // azimuth = (azimuth - 90f64).to_radians();
        // // altitude = altitude.to_radians();
        // // let sin_theta = altitude.sin();
        // // let cos_theta = altitude.cos();
        // // let mut hillshade;
        // // let mut p: Plane;
        // // for row in 0..rows {
        // //     for col in 0..columns {
        // //         p = smoothed_plane_data.get_value(row, col);
        // //         fx = -p.a / p.c;
        // //         fy = -p.b / p.c;
        // //         if fx != 0f64 {
        // //             tan_slope = (fx * fx + fy * fy).sqrt();
        // //             aspect = (180f64 - ((fy / fx).atan()).to_degrees() + 90f64 * (fx / (fx).abs())).to_radians();
        // //             term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
        // //             term2 = sin_theta / tan_slope;
        // //             term3 = cos_theta * (azimuth - aspect).sin();
        // //             hillshade = term1 * (term2 - term3);
        // //         } else {
        // //             hillshade = 0.5;
        // //         }
        // //         if hillshade < 0f64 {
        // //             hillshade = 0f64;
        // //         }
        // //         z = hillshade * 255f64;
        // //         output.set_value(row, col, z);
        // //     }
        // // }

        // let dx = [ 0, 1, 1, 1, 0, -1, -1, -1, 0 ];
        // let dy = [ 0, -1, 0, 1, 1, 1, 0, -1, -1 ];
        // let max_z_diff = [ max_z_diff_ns, max_z_diff_diag, max_z_diff_ew, max_z_diff_diag, max_z_diff_ns, max_z_diff_diag, max_z_diff_ew, max_z_diff_diag, max_z_diff_ns ];
        // for loop_num in 0..num_iter {
        //     let (mut x, mut y, mut z): (f64, f64, f64);
        //     let mut z0: f64;
        //     let mut zn: f64;
        //     let mut weights = vec![0.0; dx.len()];
        //     let mut values = vec![0.0; dx.len()];
        //     let mut weight_sum: f64;
        //     let mut norm_diff: f64;
        //     let mut p: Plane;
        //     let mut pn: Plane;
        //     let mut total_elev_change = 0f64;
        //     // let mut num_changed_cells = 0;
        //     for row in 0..rows {
        //         y = input.get_y_from_row(row);
        //         for col in 0..columns {
        //             x = input.get_x_from_column(col);
        //             z = input.get_value(row, col);
        //             if z != nodata {
        //                 p = smoothed_plane_data.get_value(row, col);
        //                 z0 = p.estimate_z(x, y); //z;
        //                 weight_sum = 0f64;
        //                 for i in 0..dx.len() {
        //                     if input.get_value(row + dy[i], col + dx[i]) != nodata {
        //                         pn = smoothed_plane_data.get_value(row + dy[i], col + dx[i]);
        //                         zn = pn.estimate_z(x, y);
        //                         norm_diff = p.angle_between(pn);
        //                         if norm_diff < max_norm_diff && (zn - z0).abs() < max_z_diff[i] {
        //                             weights[i] = 1f64 - (norm_diff / max_norm_diff);
        //                             weight_sum += weights[i];
        //                             values[i] = zn;
        //                         } else {
        //                             weights[i] = 0f64;
        //                             values[i] = 0f64;
        //                         }
        //                     } else {
        //                         weights[i] = 0f64;
        //                         values[i] = 0f64;
        //                     }
        //                 }
        //                 if weight_sum > 1f64 {
        //                     z = 0f64;
        //                     for i in 0..dx.len() {
        //                         z += weights[i] / weight_sum * values[i];
        //                     }
        //                     smoothed_plane_data.set_value(row, col, Plane{ a: p.a, b: p.b, c: p.c, d: -(p.a * x + p.b * y + p.c * z) });
        //                     total_elev_change += (z - z0).abs();
        //                     // if (z - z0).abs() > 0.0001f64 {
        //                     //     num_changed_cells += 1;
        //                     // }
        //                 }

        //                 if loop_num == num_iter-1 {
        //                     // fx = -p.a / p.c;
        //                     // fy = -p.b / p.c;
        //                     // if fx != 0f64 {
        //                     //     tan_slope = (fx * fx + fy * fy).sqrt();
        //                     //     aspect = (180f64 - ((fy / fx).atan()).to_degrees() + 90f64 * (fx / (fx).abs())).to_radians();
        //                     //     term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
        //                     //     term2 = sin_theta / tan_slope;
        //                     //     term3 = cos_theta * (azimuth - aspect).sin();
        //                     //     hillshade = term1 * (term2 - term3);
        //                     // } else {
        //                     //     hillshade = 0.5;
        //                     // }
        //                     // if hillshade < 0f64 {
        //                     //     hillshade = 0f64;
        //                     // }
        //                     // z = hillshade * 255f64;
        //                     output.set_value(row, col, z);
        //                 }
        //             }
        //         }
        //         if verbose {
        //             progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Updating DEM elevations (Loop {} of {}): {}%", loop_num+1, num_iter, progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }

        //     println!("Iteration {} elevation change: {}", loop_num+1, total_elev_change);
        //     // println!("Iteration {}: {} grid cell elevations modified", loop_num+1, num_changed_cells);

        //     // let mut total_elev_change = 0f64;
        //     // let smoothed_plane_data2 = Arc::new(smoothed_plane_data);
        //     // let (tx, rx) = mpsc::channel();
        //     // for tid in 0..num_procs {
        //     //     let input = input.clone();
        //     //     let smoothed_plane_data2 = smoothed_plane_data2.clone();
        //     //     let tx = tx.clone();
        //     //     thread::spawn(move || {
        //     //         let (mut x, mut y, mut z): (f64, f64, f64);
        //     //         let mut z0: f64;
        //     //         let mut weights = vec![0.0; dx.len()];
        //     //         let mut values = vec![0.0; dx.len()];
        //     //         let mut weight_sum: f64;
        //     //         let mut norm_diff: f64;
        //     //         let mut p: Plane;
        //     //         let mut pn: Plane;
        //     //         let mut thread_elev_change = 0f64;
        //     //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //     //             y = input.get_y_from_row(row);
        //     //             let mut data = vec![nodata; columns as usize];
        //     //             let mut plane_data = vec![Plane{ a: nodata, b: nodata, c: nodata, d: nodata}; columns as usize];
        //     //             for col in 0..columns {
        //     //                 x = input.get_x_from_column(col);
        //     //                 z = input.get_value(row, col);
        //     //                 if z != nodata {
        //     //                     p = smoothed_plane_data2.get_value(row, col);
        //     //                     weight_sum = 0f64;
        //     //                     for i in 0..dx.len() {
        //     //                         if input.get_value(row + dy[i], col + dx[i]) != nodata {
        //     //                             pn = smoothed_plane_data2.get_value(row + dy[i], col + dx[i]);
        //     //                             norm_diff = p.angle_between(pn);
        //     //                             if norm_diff < max_norm_diff {
        //     //                                 weights[i] = 1f64 - (norm_diff / max_norm_diff);
        //     //                                 values[i] = smoothed_plane_data2.get_value(row + dy[i], col + dx[i]).estimate_z(x, y);
        //     //                                 weight_sum += weights[i];
        //     //                             } else {
        //     //                                 weights[i] = 0f64;
        //     //                                 values[i] = 0f64;
        //     //                             }
        //     //                         } else {
        //     //                             weights[i] = 0f64;
        //     //                             values[i] = 0f64;
        //     //                         }
        //     //                     }
        //     //                     if weight_sum > 0f64 {
        //     //                         z0 = z;
        //     //                         z = 0f64;
        //     //                         for i in 0..dx.len() {
        //     //                             z += weights[i] / weight_sum * values[i];
        //     //                         }
        //     //                         plane_data[col as usize] = Plane{ a: p.a, b: p.b, c: p.c, d: -(p.a * x + p.b * y + p.c * z) };
        //     //                         thread_elev_change += (z - z0).abs();
        //     //                     }
        //     //                     data[col as usize] = z;
        //     //                 }
        //     //             }
        //     //             tx.send((row, data, plane_data, thread_elev_change)).unwrap();
        //     //         }
        //     //     });
        //     // }

        //     // let mut updated_planes: Array2D<Plane> = Array2D::new(rows, columns, norm_nodata, norm_nodata)?;
        //     // for row in 0..rows {
        //     //     let data = rx.recv().unwrap();
        //     //     output.set_row_data(data.0, data.1);
        //     //     updated_planes.set_row_data(data.0, data.2);
        //     //     total_elev_change += data.3;
        //     //     if verbose {
        //     //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //     //         if progress != old_progress {
        //     //             println!("Smoothing the DEM (Loop {} of {}: {}%", loop_num+1, num_iter, progress);
        //     //             old_progress = progress;
        //     //         }
        //     //     }
        //     // }

        //     // if loop_num < num_iter {

        //     //     let mut smoothed_plane_data = smoothed_plane_data2.try_unwrap().unwrap();

        //     //     //let mut smoothed_plane_data: Array2D<Plane> = Array2D::new(rows, columns, norm_nodata, norm_nodata)?;
        //     //     for row in 0..rows {
        //     //         smoothed_plane_data.set_row_data(row, updated_planes.get_row_data(row));
        //     //     }
        //     // }

        //     // println!("Iteration {} elevation change: {}", loop_num+1, (total_elev_change - prev_elev_change));
        //     // prev_elev_change = total_elev_change;

        //     //     let smoothed_plane_data = Arc::new(smoothed_plane_data);
        //     //     let (tx, rx) = mpsc::channel();
        //     //     for tid in 0..num_procs {
        //     //         let input = input.clone();
        //     //         let smoothed_plane_data = smoothed_plane_data.clone();
        //     //         let tx = tx.clone();
        //     //         thread::spawn(move || {
        //     //             let num_pixels_in_filter = filter_size * filter_size;
        //     //             let mut dx = vec![0isize; num_pixels_in_filter];
        //     //             let mut dy = vec![0isize; num_pixels_in_filter];

        //     //             // fill the filter d_x and d_y values and the distance-weights
        //     //             let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        //     //             let mut a = 0;
        //     //             for row in 0..filter_size {
        //     //                 for col in 0..filter_size {
        //     //                     dx[a] = col as isize - midpoint;
        //     //                     dy[a] = row as isize - midpoint;
        //     //                     a += 1;
        //     //                 }
        //     //             }
        //     //             // let (mut x, mut y, mut z): (f64, f64, f64);
        //     //             let mut z: f64;
        //     //             //let (mut a, mut b, mut c): (f64, f64, f64);

        //     //             // let (mut fx, mut fy): (f64, f64);
        //     //             // let (mut tan_slope, mut aspect): (f64, f64);
        //     //             // let (mut term1, mut term2, mut term3): (f64, f64, f64);
        //     //             // let mut azimuth = 315.0f64;
        //     //             // let mut altitude = 30.0f64;
        //     //             // azimuth = (azimuth - 90f64).to_radians();
        //     //             // altitude = altitude.to_radians();
        //     //             // let sin_theta = altitude.sin();
        //     //             // let cos_theta = altitude.cos();
        //     //             // let mut hillshade;

        //     //             let mut zn: f64;
        //     //             // let mut weights = vec![0.0; num_pixels_in_filter];
        //     //             // let mut values = vec![0.0; num_pixels_in_filter];
        //     //             //let mut weight_sum: f64;
        //     //             let mut w: f64;
        //     //             let mut z_hat: f64;
        //     //             let mut norm_diff: f64;
        //     //             let mut p: Plane;
        //     //             let mut pn: Plane;
        //     //             for row in (0..rows).filter(|r| r % num_procs == tid) {
        //     //                 // y = input.get_y_from_row(row);
        //     //                 let mut data = vec![nodata; columns as usize];
        //     //                 for col in 0..columns {
        //     //                     // x = input.get_x_from_column(col);
        //     //                     z = input.get_value(row, col);
        //     //                     if z != nodata {
        //     //                         p = smoothed_plane_data.get_value(row, col);
        //     //                         // weight_sum = 0f64;
        //     //                         w = 0f64;
        //     //                         z_hat = 0f64;
        //     //                         for i in 0..num_pixels_in_filter {
        //     //                             zn = input.get_value(row + dy[i], col + dx[i]);
        //     //                             if zn != nodata {
        //     //                                 pn = smoothed_plane_data.get_value(row + dy[i], col + dx[i]);
        //     //                                 norm_diff = p.angle_between(pn);
        //     //                                 if norm_diff < max_norm_diff {
        //     //                                     // weights[i] = 1f64 - (norm_diff / max_norm_diff);
        //     //                                     //values[i] = input.get_value(row + dy[i], col + dx[i]);
        //     //                                     //weight_sum += weights[i];
        //     //                                     w += 1f64;
        //     //                                     z_hat += input.get_value(row + dy[i], col + dx[i]);
        //     //                                     // p_avg += pn;
        //     //                                     //z_hat += plane_data.get_value(row + dy[i], col + dx[i]).estimate_z(x, y);
        //     //                                     // z_hat[i] = plane_data.get_value(row + dy[i], col + dx[i]).estimate_z(x, y);
        //     //                                 // } else {
        //     //                                 //     weights[i] = 0f64;
        //     //                                 //     values[i] = 0f64;
        //     //                                 }
        //     //                             // } else {
        //     //                             //     weights[i] = 0f64;
        //     //                             //     values[i] = 0f64;
        //     //                             }
        //     //                         }
        //     //                         if w > 0f64 {
        //     //                             // z = 0f64;
        //     //                             // for i in 0..num_pixels_in_filter {
        //     //                             //     z += weights[i] / weight_sum * values[i];
        //     //                             // }
        //     //                             data[col as usize] = z_hat / w;
        //     //                             // p_avg /= w;
        //     //                             // p_avg.d = -(p_avg.x * x + p_avg.y * y + p_avg.z * z);
        //     //                             // fx = -p_avg.a / p_avg.c;
        //     //                             // fy = -p_avg.b / p_avg.c;
        //     //                             // if fx != 0f64 {
        //     //                             //     tan_slope = (fx * fx + fy * fy).sqrt();
        //     //                             //     aspect = (180f64 - ((fy / fx).atan()).to_degrees() + 90f64 * (fx / (fx).abs())).to_radians();
        //     //                             //     term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
        //     //                             //     term2 = sin_theta / tan_slope;
        //     //                             //     term3 = cos_theta * (azimuth - aspect).sin();
        //     //                             //     hillshade = term1 * (term2 - term3);
        //     //                             // } else {
        //     //                             //     hillshade = 0.5;
        //     //                             // }
        //     //                             // if hillshade < 0f64 {
        //     //                             //     hillshade = 0f64;
        //     //                             // }
        //     //                             // data[col as usize] = hillshade * 255f64;
        //     //                             // let mult = match p_avg.d < p.d {
        //     //                             //     true => 1.0,
        //     //                             //     false => -1.0,
        //     //                             // };
        //     //                             // data[col as usize] = p_avg.angle_between(p).to_degrees() * mult;
        //     //                             // p_avg.d = p.d;
        //     //                             // data[col as usize] = p_avg.estimate_z(x, y);
        //     //                         } else {
        //     //                             data[col as usize] = z;
        //     //                         }
        //     //                     }
        //     //                 }
        //     //                 tx.send((row, data)).unwrap();
        //     //             }
        //     //         });
        //     //     }

        //     // let (tx, rx) = mpsc::channel();
        //     // for tid in 0..num_procs {
        //     //     let input = input.clone();
        //     //     let smoothed_plane_data = smoothed_plane_data.clone();
        //     //     let tx = tx.clone();
        //     //     thread::spawn(move || {
        //     //         let dx = [ 0, 1, 1, 1, 0, -1, -1, -1, 0 ];
        //     //         let dy = [ 0, -1, 0, 1, 1, 1, 0, -1, -1 ];
        //     //         let (mut x, mut y, mut z): (f64, f64, f64);
        //     //         // let mut z: f64;
        //     //         let mut zn: f64;
        //     //         let mut weights = vec![0.0; dx.len()];
        //     //         let mut values = vec![0.0; dx.len()];
        //     //         let mut weight_sum: f64;
        //     //         // let mut w: f64;
        //     //         // let mut z_hat: f64;
        //     //         let mut norm_diff: f64;
        //     //         let mut p: Plane;
        //     //         let mut pn: Plane;
        //     //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //     //             y = input.get_y_from_row(row);
        //     //             let mut data = vec![nodata; columns as usize];
        //     //             for col in 0..columns {
        //     //                 x = input.get_x_from_column(col);
        //     //                 z = input.get_value(row, col);
        //     //                 if z != nodata {
        //     //                     p = smoothed_plane_data.get_value(row, col);
        //     //                     weight_sum = 0f64;
        //     //                     // w = 0f64;
        //     //                     // z_hat = 0f64;
        //     //                     for i in 0..dx.len() {
        //     //                         zn = input.get_value(row + dy[i], col + dx[i]);
        //     //                         if zn != nodata {
        //     //                             pn = smoothed_plane_data.get_value(row + dy[i], col + dx[i]);
        //     //                             norm_diff = p.angle_between(pn);
        //     //                             if norm_diff < max_norm_diff {
        //     //                                 weights[i] = 1f64 - (norm_diff / max_norm_diff);
        //     //                                 values[i] = smoothed_plane_data.get_value(row + dy[i], col + dx[i]).estimate_z(x, y);
        //     //                                 weight_sum += weights[i];
        //     //                             } else {
        //     //                                 weights[i] = 0f64;
        //     //                                 values[i] = 0f64;
        //     //                             }
        //     //                         } else {
        //     //                             weights[i] = 0f64;
        //     //                             values[i] = 0f64;
        //     //                         }
        //     //                     }
        //     //                     if weight_sum > 0f64 {
        //     //                         z = 0f64;
        //     //                         for i in 0..dx.len() {
        //     //                             z += weights[i] / weight_sum * values[i];
        //     //                         }
        //     //                         data[col as usize] = z;
        //     //                         // p_avg.d = -(p_avg.x * x + p_avg.y * y + p_avg.z * z);
        //     //                     } else {
        //     //                         data[col as usize] = z;
        //     //                     }
        //     //                 }
        //     //             }
        //     //             tx.send((row, data)).unwrap();
        //     //         }
        //     //     });
        //     // }

        //     // let mut output = Raster::initialize_using_file(&output_file, &input);
        //     // for row in 0..rows {
        //     //     let data = rx.recv().unwrap();
        //     //     output.set_row_data(data.0, data.1);

        //     //     if verbose {
        //     //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //     //         if progress != old_progress {
        //     //             println!("Smoothing the DEM (Loop {} of {}: {}%", loop_num+1, max_loop, progress);
        //     //             old_progress = progress;
        //     //         }
        //     //     }
        //     // }
        // }

        // // let t3 = time::now();
        // // println!("{}", format!("Updating DEM: {}", t3 - t2).replace("PT", ""));

        let elapsed_time = time::now() - start;
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.configs.palette = input.configs.palette.clone();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size: {}", filter_size));
        output.add_metadata_entry(format!("Normal difference threshold: {}", max_norm_diff));
        output.add_metadata_entry(format!("Iterations: {}", num_iter));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
        output.add_metadata_entry(
            format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
        );

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
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

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
// #[inline]
// fn plane_from_points(points: &Vec<Vector3<f64>>) -> Plane {
//     let n = points.len();

//     let mut sum = Vector3{ x: 0.0, y: 0.0, z: 0.0 };
//     for p in points {
//         sum = sum + *p;
//     }
//     let centroid = sum * (1.0 / (n as f64));

//     // Calc full 3x3 covariance matrix, excluding symmetries:
//     let mut xx = 0.0; let mut xy = 0.0; let mut xz = 0.0;
//     let mut yy = 0.0; let mut yz = 0.0; let mut zz = 0.0;

//     for p in points {
//         let r = p - &centroid;
//         xx += r.x * r.x;
//         xy += r.x * r.y;
//         xz += r.x * r.z;
//         yy += r.y * r.y;
//         yz += r.y * r.z;
//         zz += r.z * r.z;
//     }

//     let det_x = yy*zz - yz*yz;
//     let det_y = xx*zz - xz*xz;
//     let det_z = xx*yy - xy*xy;

//     let det_max = det_x.max(det_y).max(det_z);

//     // Pick path with best conditioning:
//     let dir =
//         if det_max == det_x {
//             let a = (xz*yz - xy*zz) / det_x;
//             let b = (xy*yz - xz*yy) / det_x;
//             Vector3{ x: 1.0, y: a, z: b }
//         } else if det_max == det_y {
//             let a = (yz*xz - xy*zz) / det_y;
//             let b = (xy*xz - yz*xx) / det_y;
//             Vector3{ x: a, y: 1.0, z: b }
//         } else {
//             let a = (yz*xy - xz*yy) / det_z;
//             let b = (xz*xy - yz*xx) / det_z;
//             Vector3{ x: a, y: b, z: 1.0 }
//         };

//     let normal = normalize(dir); // return a unit normal vector
//     let d = -(normal.x * points[0].x + normal.y * points[0].y + normal.z * points[0].z);
//     Plane::new(normal, d)
// }

// #[inline]
// fn normalize(v: Vector3<f64>) -> Vector3<f64> {
//     let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
//     Vector3 { x: v.x/norm, y: v.y/norm, z: v.z/norm }
// }

// #[derive(Clone, Copy, Debug)]
// struct Plane {
//     a: f64,
//     b: f64,
//     c: f64,
//     d: f64,
//     // normal: Vector3<f64>,
//     // d: f64,
// }

// impl Plane {
//     fn new(v: Vector3<f64>, d: f64) -> Plane {
//         if v.x == 0f64 && v.y == 0f64 && v.z == 0f64 {
//             return Plane { a: 0.0000001, b: 0f64, c: 0f64, d: d }; // angle_between won't work with perfectly flat planes so add a small delta.
//         }
//         Plane { a: v.x, b: v.y, c: v.z, d: d }
//     }

//     fn angle_between(self, other: Plane) -> f64 {
//         let denom = ((self.a * self.a + self.b * self.b + self.c * self.c) * (other.a * other.a + other.b * other.b + other.c * other.c)).sqrt();
//         if denom != 0f64 {
//             let numerator = self.a * other.a + self.b * other.b + self.c * other.c;
//             return (numerator / denom).acos();
//         }
//         NEG_INFINITY
//     }

//     fn estimate_z(self, x: f64, y: f64) -> f64 {
//         // ax + by + cz + d = 0
//         // z = -(ax + by + d) / c
//         -(self.a * x + self.b * y + self.d) / self.c
//     }
// }

// impl AddAssign for Plane {
//     fn add_assign(&mut self, other: Plane) {
//         *self = Plane {
//             a: self.a + other.a,
//             b: self.b + other.b,
//             c: self.c + other.c,
//             d: self.d + other.d,
//         };
//     }
// }

// impl SubAssign for Plane {
//     fn sub_assign(&mut self, other: Plane) {
//         *self = Plane {
//             a: self.a - other.a,
//             b: self.b - other.b,
//             c: self.c - other.c,
//             d: self.d - other.d,
//         };
//     }
// }

// impl DivAssign<f64> for Plane {
//     fn div_assign(&mut self, value: f64) {
//         self.a /= value;
//         self.b /= value;
//         self.c /= value;
//         self.d /= value;
//     }
// }

#[derive(Clone, Copy, Debug)]
struct Normal {
    a: f64,
    b: f64,
    c: f64,
}

impl Normal {
    #[inline]
    fn angle_between(self, other: Normal) -> f64 {
        /*
         Note that this is actually not the angle between the vectors but
         rather the cosine of the angle between the vectors. This improves
         the performance considerably. Also note that we do not need to worry
         about checking for division by zero here because 'c' will always be 
         non-zero and therefore the vector magnitude cannot be zero.
        */
        let denom = ((self.a * self.a + self.b * self.b + self.c * self.c)
            * (other.a * other.a + other.b * other.b + other.c * other.c))
            .sqrt();
        (self.a * other.a + self.b * other.b + self.c * other.c) / denom
    }
}

impl AddAssign for Normal {
    fn add_assign(&mut self, other: Normal) {
        *self = Normal {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c + other.c,
        };
    }
}

impl SubAssign for Normal {
    fn sub_assign(&mut self, other: Normal) {
        *self = Normal {
            a: self.a - other.a,
            b: self.b - other.b,
            c: self.c - other.c,
        };
    }
}
