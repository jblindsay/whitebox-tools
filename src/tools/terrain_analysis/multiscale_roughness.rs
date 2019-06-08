/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/02/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::ops::AddAssign;
use std::ops::SubAssign;
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

pub struct MultiscaleRoughness {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultiscaleRoughness {
    pub fn new() -> MultiscaleRoughness {
        // public constructor
        let name = "MultiscaleRoughness".to_string();
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

        MultiscaleRoughness {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultiscaleRoughness {
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
                            z *= z_factor;
                            for i in 0..8 {
                                zn = input.get_value(row + dy[i], col + dx[i]);
                                if zn != nodata {
                                    values[i] = zn * z_factor;
                                } else {
                                    values[i] = z;
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

        ///////////////////////////////
        // Create the integral images /
        ///////////////////////////////
        let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1)?;

        let mut val: f64;
        let mut sum: f64;
        let mut sum_n: i32;
        let mut i_prev: f64;
        let mut n_prev: i32;
        for row in 0..rows {
            sum = 0f64;
            sum_n = 0;
            for col in 0..columns {
                val = input[(row, col)];
                if val == nodata {
                    val = 0f64;
                } else {
                    sum_n += 1;
                }
                sum += val;
                if row > 0 {
                    i_prev = integral[(row - 1, col)];
                    n_prev = integral_n[(row - 1, col)];
                    integral[(row, col)] = sum + i_prev;
                    integral_n[(row, col)] = sum_n + n_prev;
                } else {
                    integral[(row, col)] = sum;
                    integral_n[(row, col)] = sum_n;
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

        let i = Arc::new(integral); // wrap integral in an Arc
        let i_n = Arc::new(integral_n); // wrap integral_n in an Arc

        let num_procs = num_cpus::get() as isize;

        let mut output_mag = Raster::initialize_using_file(&output_mag_file, &input);
        let mut output_scale = Raster::initialize_using_file(&output_scale_file, &input);

        // let num_loops = (max_scale - min_scale) / step;
        // let mut loop_num = 0;
        for midpoint in (min_scale..max_scale).filter(|s| (s - min_scale) % step == 0) {
            // .step_by(step) { once step_by is stabilized
            // loop_num += 1;

            println!("Loop {} / {}", midpoint - min_scale, max_scale - min_scale);

            ////////////////////////////////////////////////////////////////////////////
            // Use the integral image to smooth the DEM at a scale of the filter size //
            ////////////////////////////////////////////////////////////////////////////
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let i = i.clone();
                let i_n = i_n.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let mut n: i32;
                    let mut sum: f64;
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        y1 = row - midpoint - 1;
                        if y1 < 0 {
                            y1 = 0;
                        }

                        y2 = row + midpoint;
                        if y2 >= rows {
                            y2 = rows - 1;
                        }
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input_data[(row, col)];
                            if z != nodata {
                                x1 = col - midpoint - 1;
                                if x1 < 0 {
                                    x1 = 0;
                                }

                                x2 = col + midpoint;
                                if x2 >= columns {
                                    x2 = columns - 1;
                                }
                                n = i_n[(y2, x2)] + i_n[(y1, x1)] - i_n[(y1, x2)] - i_n[(y2, x1)];
                                if n > 0 {
                                    sum = i[(y2, x2)] + i[(y1, x1)] - i[(y1, x2)] - i[(y2, x1)];
                                    data[col as usize] = sum / n as f64;
                                }
                            }
                        }

                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut smoothed: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
            for _ in 0..rows {
                let (row, data) = rx.recv().unwrap();
                smoothed.set_row_data(row, data);
            }

            ///////////////////////////////////////////////////////////////////////////
            // Calculate the deviations in the normals of the unsmoothed and smoothed
            // DEMs, placing the values in an integral image.
            ///////////////////////////////////////////////////////////////////////////
            let mut i_diff_nv: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let eight_grid_res = input.configs.resolution_x * 8f64;
            let mut z: f64;
            let mut zn: f64;
            let (mut a, mut b): (f64, f64);
            let mut diff: f64;
            let mut values = [0f64; 9];
            for row in 0..rows {
                sum = 0f64;
                for col in 0..columns {
                    if input.get_value(row, col) != nodata {
                        z = smoothed.get_value(row, col) * z_factor;
                        for i in 0..8 {
                            zn = smoothed.get_value(row + dy[i], col + dx[i]);
                            if zn != nodata {
                                values[i] = zn * z_factor;
                            } else {
                                values[i] = z;
                            }
                        }

                        a = -(values[2] - values[4] + 2f64 * (values[1] - values[5]) + values[0]
                            - values[6]);
                        b = -(values[6] - values[4] + 2f64 * (values[7] - values[3]) + values[0]
                            - values[2]);
                        diff = (nv.get_value(row, col).angle_between(Normal {
                            a: a,
                            b: b,
                            c: eight_grid_res,
                        }))
                        .acos()
                        .to_degrees();
                    } else {
                        diff = 0f64;
                    }

                    // output_mag.set_value(row, col, diff);

                    sum += diff;
                    if row > 0 {
                        z = i_diff_nv.get_value(row - 1, col);
                        i_diff_nv.set_value(row, col, sum + z);
                    } else {
                        i_diff_nv.set_value(row, col, sum);
                    }
                }
            }

            ///////////////////////////////////////////////////////////////////////////
            // Calcuate the average deviation within the local kernels and output it //
            ///////////////////////////////////////////////////////////////////////////
            let i_diff_nv = Arc::new(i_diff_nv);
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let i_n = i_n.clone();
                let i_diff_nv = i_diff_nv.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let mut n: i32;
                    let mut sum: f64;
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        y1 = row - midpoint - 1;
                        if y1 < 0 {
                            y1 = 0;
                        }

                        y2 = row + midpoint;
                        if y2 >= rows {
                            y2 = rows - 1;
                        }
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input_data[(row, col)];
                            if z != nodata {
                                x1 = col - midpoint - 1;
                                if x1 < 0 {
                                    x1 = 0;
                                }

                                x2 = col + midpoint;
                                if x2 >= columns {
                                    x2 = columns - 1;
                                }
                                n = i_n[(y2, x2)] + i_n[(y1, x1)] - i_n[(y1, x2)] - i_n[(y2, x1)];
                                if n > 0 {
                                    sum = i_diff_nv[(y2, x2)] + i_diff_nv[(y1, x1)]
                                        - i_diff_nv[(y1, x2)]
                                        - i_diff_nv[(y2, x1)];
                                    data[col as usize] = sum / n as f64;
                                }
                            }
                        }

                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            for _ in 0..rows {
                let (row, data) = rx.recv().unwrap();
                for col in 0..columns {
                    if data[col as usize] > output_mag.get_value(row, col) {
                        output_mag.set_value(row, col, data[col as usize]);
                        output_scale.set_value(row, col, midpoint as f64);
                    }
                }
            }
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
