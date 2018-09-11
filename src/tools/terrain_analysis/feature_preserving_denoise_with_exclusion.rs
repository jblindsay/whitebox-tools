/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/09/2018
Last Modified: 06/09/2018
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
/// Sun, Rosin, Martin, and Langbein (2007) Fast and effective feature-preserving
/// mesh denoising that includes exclusion zones, typically for small drainage
/// features, such as ditches, rills, gullies, etc., which would otherwise
/// be smoothed over.
pub struct FeaturePreservingDenoiseWithExclusions {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FeaturePreservingDenoiseWithExclusions {
    pub fn new() -> FeaturePreservingDenoiseWithExclusions {
        // public constructor
        let name = "FeaturePreservingDenoiseWithExclusions".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm with exclusions for small drainage features.".to_string();

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
            name: "Input Exclusion Zones File".to_owned(),
            flags: vec!["--exclusions".to_owned()],
            description: "Input raster exclusion zones file.".to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --exclusions=gullies.tif -o=output.tif --filter=15 --norm_diff=20.0 --num_iter=4",
            short_exe, name
        ).replace("*", &sep);

        FeaturePreservingDenoiseWithExclusions {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FeaturePreservingDenoiseWithExclusions {
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
        let mut exclusions_file = String::new();
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
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-exclusion" || flag_val == "-exclusions" {
                if keyval {
                    exclusions_file = vec[1].to_string();
                } else {
                    exclusions_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-filter" {
                if keyval {
                    filter_size = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            } else if flag_val == "-norm_diff" {
                if keyval {
                    max_norm_diff = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_norm_diff = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-num_iter" {
                if keyval {
                    num_iter = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    num_iter = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            } else if flag_val == "-zfactor" {
                if keyval {
                    z_factor = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    z_factor = args[i + 1].to_string().parse::<f64>().unwrap();
                }
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
        // let threshold2 = (max_norm_diff / 4f64).to_radians().cos();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !exclusions_file.contains(&sep) && !exclusions_file.contains("/") {
            exclusions_file = format!("{}{}", working_directory, exclusions_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let exclusions = Arc::new(Raster::new(&exclusions_file, "r")?);

        let start = time::now();

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
            // let exclusions = exclusions.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let eight_grid_res = input.configs.resolution_x * 8f64;
                let mut z: f64;
                let mut zn: f64;
                // let mut exclusion: f64;
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
                        // exclusion = exclusions.get_value(row, col);
                        if z != nodata {
                            //&& exclusion == 0f64 {
                            for i in 0..8 {
                                zn = input.get_value(row + dy[i], col + dx[i]);
                                if zn != nodata {
                                    values[i] = zn * z_factor;
                                } else {
                                    values[i] = z * z_factor;
                                }
                            }
                            a = -(values[2] - values[4] + 2f64 * (values[1] - values[5]) + values[0]
                                - values[6]);
                            b = -(values[6] - values[4] + 2f64 * (values[7] - values[3]) + values[0]
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
            let exclusions = exclusions.clone();
            let nv = nv.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let num_pixels_in_filter = filter_size * filter_size;
                let mut dx = vec![0isize; num_pixels_in_filter];
                let mut dy = vec![0isize; num_pixels_in_filter];

                // fill the filter d_x and d_y values and the distance-weights
                let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize;
                let mut a = 0;
                for row in 0..filter_size {
                    for col in 0..filter_size {
                        dx[a] = col as isize - midpoint;
                        dy[a] = row as isize - midpoint;
                        a += 1;
                    }
                }
                let mut z: f64;
                let mut exclusion: f64;
                let mut threshold_adj: f64;
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
                        exclusion = exclusions.get_value(row, col);
                        threshold_adj = if exclusion < 0f64 {
                            (max_norm_diff / (exclusion.abs() * 10f64))
                                .to_radians()
                                .cos()
                        } else {
                            threshold
                        };
                        // if z != nodata && exclusion == 0f64 {
                        //     sum_w = 0f64;
                        //     a = 0f64;
                        //     b = 0f64;
                        //     c = 0f64;
                        //     for n in 0..num_pixels_in_filter {
                        //         xn = col + dx[n];
                        //         yn = row + dy[n];
                        //         if input.get_value(yn, xn) != nodata {
                        //             //     && exclusions.get_value(yn, xn) == 0f64
                        //             // {
                        //             diff =
                        //                 nv.get_value(row, col).angle_between(nv.get_value(yn, xn));
                        //             if diff > threshold {
                        //                 w = (diff - threshold) * (diff - threshold);
                        //                 sum_w += w;
                        //                 a += nv.get_value(yn, xn).a * w;
                        //                 b += nv.get_value(yn, xn).b * w;
                        //                 c += nv.get_value(yn, xn).c * w;
                        //             }
                        //         }
                        //     }

                        //     a /= sum_w;
                        //     b /= sum_w;
                        //     c /= sum_w;
                        //     data[col as usize] = Normal { a: a, b: b, c: c };
                        // } else if z != nodata {
                        if z != nodata {
                            sum_w = 0f64;
                            a = 0f64;
                            b = 0f64;
                            c = 0f64;
                            for n in 0..num_pixels_in_filter {
                                xn = col + dx[n];
                                yn = row + dy[n];
                                if input.get_value(yn, xn) != nodata {
                                    //     && exclusions.get_value(yn, xn) == 0f64
                                    // {
                                    diff =
                                        nv.get_value(row, col).angle_between(nv.get_value(yn, xn));
                                    if diff > threshold_adj {
                                        w = (diff - threshold_adj) * (diff - threshold_adj);
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
        let mut exclusion: f64;
        let mut threshold_adj: f64;
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
                    exclusion = exclusions.get_value(row, col);
                    // if z != nodata && exclusion == 0f64 {
                    //     sum_w = 0f64;
                    //     z = 0f64;
                    //     for n in 0..8 {
                    //         xn = col + dx[n];
                    //         yn = row + dy[n];
                    //         zn = output.get_value(yn, xn);
                    //         if zn != nodata {
                    //             //&& exclusions.get_value(yn, xn) == 0f64 {
                    //             diff = nv_smooth
                    //                 .get_value(row, col)
                    //                 .angle_between(nv_smooth.get_value(yn, xn));
                    //             if diff > threshold {
                    //                 w = (diff - threshold) * (diff - threshold);
                    //                 sum_w += w;
                    //                 z += -(nv_smooth.get_value(yn, xn).a * x[n]
                    //                     + nv_smooth.get_value(yn, xn).b * y[n]
                    //                     - nv_smooth.get_value(yn, xn).c * zn)
                    //                     / nv_smooth.get_value(yn, xn).c
                    //                     * w;
                    //             }
                    //         }
                    //     }
                    //     if sum_w > 0f64 {
                    //         // this is a division-by-zero safeguard and must be in place.
                    //         output.set_value(row, col, z / sum_w);
                    //     } else {
                    //         output.set_value(row, col, input.get_value(row, col));
                    //     }
                    // } else if z != nodata {
                    if z != nodata {
                        threshold_adj = if exclusion < 0f64 {
                            (max_norm_diff / (exclusion.abs() * 10f64))
                                .to_radians()
                                .cos()
                        } else {
                            threshold
                        };
                        sum_w = 0f64;
                        z = 0f64;
                        for n in 0..8 {
                            xn = col + dx[n];
                            yn = row + dy[n];
                            zn = output.get_value(yn, xn);
                            if zn != nodata {
                                //&& exclusions.get_value(yn, xn) == 0f64 {
                                diff = nv_smooth
                                    .get_value(row, col)
                                    .angle_between(nv_smooth.get_value(yn, xn));
                                if diff > threshold_adj {
                                    w = (diff - threshold_adj) * (diff - threshold_adj);
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

        let elapsed_time = time::now() - start;
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.configs.palette = input.configs.palette.clone();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("Input exclusions file: {}", exclusions_file));
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
