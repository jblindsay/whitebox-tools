/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/01/2019
Last Modified: 27/01/2019
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

/// This tool can be used to calculate the circular variance (i.e. one minus the mean resultant length) of aspect
/// for an input digital elevation model (DEM). This is a measure of how variable slope aspect is within a local
/// neighbourhood of a specified size (`--filter`). `CircularVarianceOfAspect` is therefore a measure of surface
/// shape complexity, or texture. It will take a value near 0.0 for smooth sites and 1.0 in areas of high surface 
/// roughness or complex topography. 
/// 
/// # See Also
/// `Aspect`, `MultiscaleRoughness`, `EdgeDensity`, `SurfaceAreaRatio`, `RuggednessIndex`
pub struct CircularVarianceOfAspect {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CircularVarianceOfAspect {
    pub fn new() -> CircularVarianceOfAspect {
        // public constructor
        let name = "CircularVarianceOfAspect".to_string();
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
            name: "Output Roughness Scale File".to_owned(),
            flags: vec!["--output".to_owned()],
            description: "Output raster roughness scale file.".to_owned(),
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

        CircularVarianceOfAspect {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CircularVarianceOfAspect {
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
                    vec[1].to_string().parse::<f64>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap() as usize
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
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

        // first calculate the aspect
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let eight_grid_res = input.configs.resolution_x * 8.0;
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
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            // let rows = rows.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let (mut fx, mut fy): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut xdata = vec![0f64; columns as usize];
                    let mut ydata = vec![0f64; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            for c in 0..8 {
                                n[c] = input[(row + dy[c], col + dx[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z * z_factor;
                                }
                            }
                            // calculate slope
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            if fx != 0f64 {
                                z = (fx * fx + fy * fy).sqrt();
                                xdata[col as usize] = fx / z;
                                ydata[col as usize] = fy / z;
                            } else {
                                xdata[col as usize] = 0f64;
                                ydata[col as usize] = 0f64;
                            }
                        }
                    }
                    tx.send((row, xdata, ydata)).unwrap();
                }
            });
        }

        let mut xc: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?;
        let mut yc: Array2D<f64> = Array2D::new(rows, columns, 0f64, -1f64)?;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            xc.set_row_data(data.0, data.1);
            yc.set_row_data(data.0, data.2);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating aspect data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // convert to integral images
        let mut i_n: Array2D<u32> = Array2D::new(rows, columns, 1, 0)?;
        let (mut sumx, mut sumy): (f64, f64);
        let mut sumn: u32;
        for row in 0..rows {
            if row > 0 {
                sumx = 0f64;
                sumy = 0f64;
                sumn = 0u32;
                for col in 0..columns {
                    sumx += xc.get_value(row, col);
                    sumy += yc.get_value(row, col);
                    if input.get_value(row, col) == nodata {
                        i_n.decrement(row, col, 1);
                    }
                    sumn += i_n.get_value(row, col);
                    xc.set_value(row, col, sumx + xc.get_value(row-1, col));
                    yc.set_value(row, col, sumy + yc.get_value(row-1, col));
                    i_n.set_value(row, col, sumn + i_n.get_value(row-1, col));
                }
            } else {
                if input.get_value(0, 0) == nodata {
                    i_n.set_value(0, 0, 0);
                }
                for col in 1..columns {
                    xc.increment(row, col, xc.get_value(row, col-1));
                    yc.increment(row, col, yc.get_value(row, col-1));
                    i_n.increment(row, col, i_n.get_value(row, col-1));
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

        let xc = Arc::new(xc);
        let yc = Arc::new(yc);
        let i_n = Arc::new(i_n);
        let (tx2, rx2) = mpsc::channel();
        for tid in 0..num_procs {
            let xc = xc.clone();
            let yc = yc.clone();
            let input = input.clone();
            let i_n = i_n.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                let mut n: f64;
                let (mut sumx, mut sumy): (f64, f64);
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
                        z = input.get_value(row, col);
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
                                mean = (sumx * sumx + sumy * sumy).sqrt() / n;
                                if mean > 1f64 { 
                                    mean = 1f64; 
                                }
                                data[col as usize] = 1f64 - mean;
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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        if output.configs.data_type != DataType::F32 && output.configs.data_type != DataType::F64 {
            output.configs.data_type = DataType::F32;
        }
        for row in 0..rows {
            match rx2.recv() {
                Ok(data) => {
                    output.set_row_data(data.0, data.1);
                },
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
