/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 20, 2017
Last Modified: 12/10/2018
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

/// This tool can be used to calculate the maximum deviation from mean elevation, *DEVmax* (Lindsay et al. 2015) for each
/// grid cell in a digital elevation model (DEM) across a range specified spatial scales. *DEV*
/// is an elevation residual index and is essentially equivalent to a local elevation z-score.
/// This attribute measures the *relative topographic position* as a fraction of
/// local relief, and so is normalized to the local surface roughness. The multi-scaled calculation
/// of *DEVmax* utilizes an integral image approach (Crow, 1984) to ensure highly efficient
/// filtering that is invariant with filter size, which is the algorithm characteristic that allows for
/// this densely sampled multi-scale analysis. In this way, `MaxElevationDeviation` allows users
/// to estimate the locally optimal scale with which to estimate *DEV* on a pixel-by-pixel basis.
/// This multi-scaled version of local topographic position can reveal significant terrain
/// characteristics and can aid with soil, vegetation, landform, and other mapping applications
/// that depend on geomorphometric characterization.
///
/// The user must specify the name (`--dem`) of the input digital elevation model (DEM). The range of scales that are
/// evaluated in calculating *DEVmax* are determined by the user-specified `--min_scale`, `--max_scale`, and `--step`
/// parameters. All filter radii between the minimum and maximum scales, increasing by `step`, will be evaluated.
/// The scale parameters are in units of grid cells and specify kernel size "radii" (*r*), such that:
///
/// > *d* = 2*r* + 1
///
/// That is, a radii of 1, 2, 3... yields a square filters of dimension (*d*) 3 x 3, 5 x 5, 7 x 7...
///
/// *DEV* is estimated at each tested filter size and every grid cell is assigned the maximum *DEV* value
/// across the evaluated scales.
///
/// The user must specify the file names of two output rasters, including the magnitude (*DEVmax*) and a
/// second raster the assigns each pixel the scale at which *DEVmax* is encountered (*DEVscale*). The
/// *DEVscale* raster can be very useful for revealing multi-scale landscape structure.
///
/// # Reference
/// Lindsay J, Cockburn J, Russell H. 2015. An integral image approach to performing multi-scale
/// topographic position analysis. Geomorphology, 245: 51-61.
///
/// # See Also
/// `DevFromMeanElev`, `MaxDifferenceFromMean`, `MultiscaleElevationPercentile`
pub struct MaxElevationDeviation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MaxElevationDeviation {
    pub fn new() -> MaxElevationDeviation {
        // public constructor
        let name = "MaxElevationDeviation".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the maximum elevation deviation over a range of spatial scales."
                .to_string();

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
            name: "Output DEVmax Magnitude File".to_owned(),
            flags: vec!["--out_mag".to_owned()],
            description: "Output raster DEVmax magnitude file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output DEVmax Scale File".to_owned(),
            flags: vec!["--out_scale".to_owned()],
            description: "Output raster DEVmax scale file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Search Neighbourhood Radius (grid cells)".to_owned(),
            flags: vec!["--min_scale".to_owned()],
            description: "Minimum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false,
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
            optional: false,
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --out_mag=DEVmax_mag.tif --out_scale=DEVmax_scale.tif --min_scale=1 --max_scale=1000 --step=5", short_exe, name).replace("*", &sep);

        MaxElevationDeviation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MaxElevationDeviation {
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
            if vec[0].to_lowercase() == "-i"
                || vec[0].to_lowercase() == "--input"
                || vec[0].to_lowercase() == "--dem"
            {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_mag" || vec[0].to_lowercase() == "--out_mag" {
                if keyval {
                    output_mag_file = vec[1].to_string();
                } else {
                    output_mag_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_scale"
                || vec[0].to_lowercase() == "--out_scale"
            {
                if keyval {
                    output_scale_file = vec[1].to_string();
                } else {
                    output_scale_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-min_scale"
                || vec[0].to_lowercase() == "--min_scale"
            {
                if keyval {
                    min_scale = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    min_scale = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if min_scale < 1 {
                    min_scale = 1;
                }
            } else if vec[0].to_lowercase() == "-max_scale"
                || vec[0].to_lowercase() == "--max_scale"
            {
                if keyval {
                    max_scale = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    max_scale = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if max_scale < 5 {
                    max_scale = 5;
                }
            } else if vec[0].to_lowercase() == "-step" || vec[0].to_lowercase() == "--step" {
                if keyval {
                    step = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    step = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if step < 1 {
                    step = 1;
                }
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

        // create the integral images
        let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral2: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1)?;

        let mut val: f64;
        let mut sum: f64;
        let mut sum_sqr: f64;
        let mut sum_n: i32;
        let (mut i_prev, mut i2_prev): (f64, f64);
        let mut n_prev: i32;
        for row in 0..rows {
            sum = 0f64;
            sum_sqr = 0f64;
            sum_n = 0;
            for col in 0..columns {
                val = input[(row, col)];
                if val == nodata {
                    val = 0f64;
                } else {
                    sum_n += 1;
                }
                sum += val;
                sum_sqr += val * val;
                if row > 0 {
                    i_prev = integral[(row - 1, col)];
                    i2_prev = integral2[(row - 1, col)];
                    n_prev = integral_n[(row - 1, col)];
                    integral[(row, col)] = sum + i_prev;
                    integral2[(row, col)] = sum_sqr + i2_prev;
                    integral_n[(row, col)] = sum_n + n_prev;
                } else {
                    integral[(row, col)] = sum;
                    integral2[(row, col)] = sum_sqr;
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
        let i2 = Arc::new(integral2); // wrap integral2 in an Arc
        let i_n = Arc::new(integral_n); // wrap integral_n in an Arc

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        let mut output_mag = Raster::initialize_using_file(&output_mag_file, &input);
        output_mag.configs.data_type = DataType::F32;
        let mut output_scale = Raster::initialize_using_file(&output_scale_file, &input);
        output_scale.configs.data_type = DataType::I16;

        let num_loops = (max_scale - min_scale) / step;
        let mut loop_num = 0;
        for midpoint in (min_scale..max_scale).step_by(step as usize) {
            loop_num += 1;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let i = i.clone();
                let i2 = i2.clone();
                let i_n = i_n.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let mut n: i32;
                    let (mut mean, mut sum, mut sum_sqr): (f64, f64, f64);
                    let (mut v, mut s): (f64, f64);
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
                                    sum_sqr =
                                        i2[(y2, x2)] + i2[(y1, x1)] - i2[(y1, x2)] - i2[(y2, x1)];
                                    v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                    if v > 0f64 {
                                        s = v.sqrt();
                                        mean = sum / n as f64;
                                        data[col as usize] = (z - mean) / s;
                                    } else {
                                        data[col as usize] = 0f64;
                                    }
                                } else {
                                    data[col as usize] = 0f64;
                                }
                            }
                        }

                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let (mut z1, mut z2): (f64, f64);
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns {
                    z2 = data[col as usize];
                    if z2 != nodata {
                        z1 = output_mag[(row, col)];
                        if z1 != nodata {
                            if z2 * z2 > z1 * z1 {
                                output_mag[(row, col)] = z2;
                                output_scale[(row, col)] = midpoint as f64;
                            }
                        } else {
                            output_mag[(row, col)] = z2;
                            output_scale[(row, col)] = midpoint as f64;
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Progress (Loop {} of {}): {}%",
                            loop_num, num_loops, progress
                        );
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output_mag.configs.display_min = -3.0;
        output_mag.configs.display_max = 3.0;
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
