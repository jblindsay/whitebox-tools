/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/01/2019
Last Modified: 03/09/2020
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

/// This tool calculates the density of edges, or breaks-in-slope within an input digital elevation model (DEM).
/// A break-in-slope occurs between two neighbouring grid cells if the angular difference between their normal
/// vectors is greater than a user-specified threshold value (`--norm_diff`). `EdgeDensity` calculates the proportion
/// of edge cells within the neighbouring window, of square filter dimension `--filter`, surrounding each grid cell.
/// Therefore, `EdgeDensity `is a measure of how complex the topographic surface is within a local neighbourhood.
/// It is therefore a measure of topographic texture. It will take a value near 0.0 for smooth sites and 1.0 in areas
/// of high surface roughness or complex topography.
///
/// The distribution of `EdgeDensity` is highly dependent upon the value of the `norm_diff` used in the calculation. This
/// threshold may require experimentation to find an appropriate value and is likely dependent upon the topography and
/// source data. Nonetheless, experience has shown that `EdgeDensity` provides one of the best measures of surface
/// texture of any of the available roughness tools.
///
/// # See Also
/// `CircularVarianceOfAspect`, `MultiscaleRoughness`, `SurfaceAreaRatio`, `RuggednessIndex`
pub struct EdgeDensity {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EdgeDensity {
    pub fn new() -> EdgeDensity {
        // public constructor
        let name = "EdgeDensity".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the density of edges, or breaks-in-slope within DEMs.".to_string();

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
            default_value: Some("5.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Z Conversion Factor".to_owned(),
            flags: vec!["--zfactor".to_owned()],
            description:
                "Optional multiplier for when the vertical and horizontal units are not the same."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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

        EdgeDensity {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EdgeDensity {
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
        let mut max_norm_diff = 5f64;
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
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-norm_diff" {
                max_norm_diff = if keyval {
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

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if filter_size < 3 {
            filter_size = 3;
        }

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size as f64 / 2f64).floor() == (filter_size as f64 / 2f64) {
            filter_size += 1;
        }

        let midpoint = (filter_size as f64 / 2f64).floor() as isize;

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

        let start = Instant::now();

        if input.is_in_geographic_coordinates() && z_factor < 0.0 {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (111320.0 * mid_lat.cos());
                println!("It appears that the DEM is in geographic coordinates. The z-factor has been updated: {}.", z_factor);
            }
        } else if z_factor < 0.0 {
            z_factor = 1.0;
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
                let eight_grid_res = input.configs.resolution_x as f32 * 8f32;
                let mut z: f64;
                let mut zn: f64;
                let (mut a, mut b): (f32, f32);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![
                        Normal {
                            a: 0f32,
                            b: 0f32,
                            c: 0f32
                        };
                        columns as usize
                    ];
                    let mut values = [0f32; 9];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            for i in 0..8 {
                                zn = input.get_value(row + dy[i], col + dx[i]);
                                if zn != nodata {
                                    values[i] = (zn * z_factor) as f32;
                                } else {
                                    values[i] = (z * z_factor) as f32;
                                }
                            }
                            a = -(values[2] - values[4]
                                + 2f32 * (values[1] - values[5])
                                + values[0]
                                - values[6]);
                            b = -(values[6] - values[4]
                                + 2f32 * (values[7] - values[3])
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
            a: 0f32,
            b: 0f32,
            c: 0f32,
        };
        let mut nv: Array2D<Normal> = Array2D::new(rows, columns, zero_vector, zero_vector)?;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            nv.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating normal vectors: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        ///////////////////////////
        // Find breaks in slope. //
        ///////////////////////////
        let nv = Arc::new(nv);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let nv = nv.clone();
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut z: f64;
                let mut zn: f64;
                let (mut centre_norm, mut neighbour_norm): (Normal, Normal);
                let mut edge_found: bool;
                let threshold32 = threshold as f32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![0f64; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            centre_norm = nv.get_value(row, col);
                            edge_found = false;
                            for c in 0..8 {
                                zn = input.get_value(row + dy[c], col + dx[c]);
                                if zn != nodata {
                                    neighbour_norm = nv.get_value(row + dy[c], col + dx[c]);
                                    if centre_norm.angle_between(neighbour_norm) <= threshold32 {
                                        edge_found = true;
                                        break;
                                    }
                                }
                            }

                            if edge_found {
                                data[col as usize] = 1f64;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut edges: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            edges.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding edges: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Convert edges into an integral image and create an 'n' integral too.
        let mut i_n: Array2D<u32> = Array2D::new(rows, columns, 1, u32::max_value())?;
        let mut sum: f64;
        let mut sumn: u32;
        for row in 0..rows {
            if row > 0 {
                sum = 0f64;
                sumn = 0u32;
                for col in 0..columns {
                    sum += edges.get_value(row, col);
                    edges.set_value(row, col, sum + edges.get_value(row - 1, col));
                    if input.get_value(row, col) == nodata {
                        i_n.decrement(row, col, 1);
                    }
                    sumn += i_n.get_value(row, col);
                    i_n.set_value(row, col, sumn + i_n.get_value(row - 1, col));
                }
            } else {
                if input.get_value(0, 0) == nodata {
                    i_n.set_value(0, 0, 0);
                }
                for col in 1..columns {
                    edges.increment(row, col, edges.get_value(row, col - 1));
                    i_n.increment(row, col, i_n.get_value(row, col - 1));
                    if input.get_value(row, col) == nodata {
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

        let edges = Arc::new(edges);
        let i_n = Arc::new(i_n);
        let (tx2, rx2) = mpsc::channel();
        for tid in 0..num_procs {
            let edges = edges.clone();
            let i_n = i_n.clone();
            let input = input.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                let mut n: f64;
                let mut sum: f64;
                let mut mean: f64;
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
                        z = input.get_value(row, col);
                        if z != nodata {
                            x1 = col - midpoint - 1;
                            if x1 < 0 {
                                x1 = 0;
                            }

                            x2 = col + midpoint;
                            if x2 >= columns {
                                x2 = columns - 1;
                            }
                            n = (i_n.get_value(y2, x2) + i_n.get_value(y1, x1)
                                - i_n.get_value(y1, x2)
                                - i_n.get_value(y2, x1)) as f64;
                            if n > 0f64 {
                                sum = edges.get_value(y2, x2) + edges.get_value(y1, x1)
                                    - edges.get_value(y1, x2)
                                    - edges.get_value(y2, x1);
                                mean = sum / n;
                                data[col as usize] = mean;
                            }
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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        if output.configs.data_type != DataType::F32 && output.configs.data_type != DataType::F64 {
            output.configs.data_type = DataType::F32;
        }
        for row in 0..rows {
            match rx2.recv() {
                Ok(data) => {
                    output.set_row_data(data.0, data.1);
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
        output.configs.palette = "blue_white_red.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size: {}", filter_size));
        output.add_metadata_entry(format!("Normal difference threshold: {}", max_norm_diff));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
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

#[derive(Clone, Copy, Debug)]
struct Normal {
    a: f32,
    b: f32,
    c: f32,
}

impl Normal {
    fn angle_between(self, other: Normal) -> f32 {
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
