/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/12/2019
Last Modified: 22/12/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::i64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool calculates the most elevation percentile (EP) across a range of spatial scales.
/// EP is a measure of local topographic position (LTP) and expresses the vertical
/// position for a digital elevation model (DEM) grid cell (z<sub>0</sub>) as the percentile of the
/// elevation distribution within the filter window, such that:
///
/// > EP = count<sub>i&isin;C</sub>(z<sub>i</sub> > z<sub>0</sub>) x (100 / n<sub>C</sub>)
///
/// where z<sub>0</sub> is the elevation of the window's center grid cell, z<sub>i</sub> is the elevation
/// of cell *i* contained within the neighboring set C, and n<sub>C</sub> is the number
/// of grid cells contained within the window.
///
/// EP is unsigned and expressed as a percentage, bound between 0%
/// and 100%. This tool outputs two rasters, the multiscale EP magnitude (`--out_mag`) and
/// the scale at which the most extreme EP value occurs (`--out_scale`). **The magnitude raster is
/// the most extreme EP value (i.e. the furthest from 50%) for each grid cell encountered within
/// the tested scales of EP.**
///
/// Quantile-based estimates (e.g., the median and interquartile
/// range) are often used in nonparametric statistics to provide data
/// variability estimates without assuming the distribution is normal.
/// Thus, EP is largely unaffected by irregularly shaped elevation frequency
/// distributions or by outliers in the DEM, resulting in a highly robust metric
/// of LTP. In fact, elevation distributions within small to medium sized
/// neighborhoods often exhibit skewed, multimodal, and non-Gaussian
/// distributions, where the occurrence of elevation errors can often result
/// in distribution outliers. Thus, based on these statistical characteristics,
/// EP is considered one of the most robust representation of LTP.
///
/// The algorithm implemented by this tool uses the relatively efficient running-histogram filtering algorithm of Huang
/// et al. (1979). Because most DEMs contain floating point data, elevation values must be rounded to be binned. The
/// `--sig_digits` parameter is used to determine the level of precision preserved during this binning process. The
/// algorithm is parallelized to further aid with computational efficiency.
///
/// Experience with multiscale EP has shown that it is highly variable at
/// shorter scales and changes more gradually at broader scales. Therefore, a nonlinear scale sampling
/// interval is used by this tool to ensure that the scale sampling density is higher for short scale
/// ranges and coarser at longer tested scales, such that:
///
/// > *r<sub>i</sub>* = *r<sub>L</sub>* + [step &times; (i - *r<sub>L</sub>*)]<sup>*p*</sup>
///
/// Where *ri* is the filter radius for step *i* and *p* is the nonlinear scaling factor (`--step_nonlinearity`)
/// and a step size (`--step`) of *step*.
///
///
/// # References
/// Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position
/// for multiscale geomorphometric analysis. Geomorphology, 312, 40-50.
///
/// Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE
/// Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18.
///
/// # See Also
/// `ElevationPercentile`, `MaxElevationDeviation`, `MaxDifferenceFromMean`
pub struct MultiscaleElevationPercentile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultiscaleElevationPercentile {
    pub fn new() -> MultiscaleElevationPercentile {
        // public constructor
        let name = "MultiscaleElevationPercentile".to_string();
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
            name: "Number of Significant Digits".to_owned(),
            flags: vec!["--sig_digits".to_owned()],
            description: "Number of significant digits.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("3".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Minimum Search Neighbourhood Radius (grid cells)".to_owned(),
            flags: vec!["--min_scale".to_owned()],
            description: "Minimum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("4".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Base Step Size".to_owned(),
            flags: vec!["--step".to_owned()],
            description: "Step size as any positive non-zero integer.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Number of Steps".to_owned(),
            flags: vec!["--num_steps".to_owned()],
            description: "Number of steps".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("10".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Step Nonlinearity".to_owned(),
            flags: vec!["--step_nonlinearity".to_owned()],
            description: "Step nonlinearity factor (1.0-2.0 is typical)".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --out_mag=roughness_mag.tif --out_scale=roughness_scale.tif --min_scale=1 --step=5 --num_steps=100 --step_nonlinearity=1.5", short_exe, name).replace("*", &sep);

        MultiscaleElevationPercentile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultiscaleElevationPercentile {
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
        let mut num_sig_digits = 3i32;
        let mut min_scale = 4isize;
        let mut step = 1isize;
        let mut num_steps = 10isize;
        let mut step_nonlinearity = 1.0f32;
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
            } else if flag_val == "-step" {
                step = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            } else if flag_val == "-num_steps" {
                num_steps = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            } else if flag_val == "-step_nonlinearity" {
                step_nonlinearity = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-sig_digits" {
                num_sig_digits = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<i32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<i32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        if step < 1 {
            eprintln!("Warning: Step value must be at least 1.0. Value set to 1.0.");
            step = 1;
        }

        if step_nonlinearity < 1.0 {
            eprintln!("Warning: Step nonlinearity value must be great than 1.0. Value set to 1.0.");
            step_nonlinearity = 1.0;
        }

        if step_nonlinearity > 4.0 {
            eprintln!("Warning: Step nonlinearity is set too high. Value reset to 4.0.");
            step_nonlinearity = 4.0;
        }

        if num_steps < 1 {
            eprintln!("Warning: Number of steps must be at least 1.");
            num_steps = 1;
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
        let input = Raster::new(&input_file, "r")?; // Memory requirements: 2.0X, assuming data is stored as f32s
        let start = Instant::now();

        // first bin the data
        let configs = input.configs.clone();
        let rows = configs.rows as isize;
        let columns = configs.columns as isize;
        let nodata = configs.nodata;
        let multiplier = 10f64.powi(num_sig_digits);
        let min_val = configs.minimum;
        let max_val = configs.maximum;
        let min_bin = (min_val * multiplier).floor() as i64;
        let num_bins = (max_val * multiplier).floor() as i64 - min_bin + 1;
        let bin_nodata = std::i16::MIN as i64;
        let mut binned_data: Array2D<i64> = Array2D::new(rows, columns, bin_nodata, bin_nodata)?; // Memory requirements: 4.0X

        let mut num_procs = num_cpus::get() as isize;
        let configurations = whitebox_common::configs::get_configs()?;
        let max_procs = configurations.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut val: i64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![bin_nodata; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            val = (z * multiplier).floor() as i64 - min_bin;
                            data[col as usize] = val;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            binned_data.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Binning data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input); // Memory requirements: 2.0X

        if verbose {
            println!("Initializing grids...");
        }
        let mut output_mag = Raster::initialize_using_config(&output_mag_file, &configs); // Memory requirements: 4.0X
        let mut output_scale = Raster::initialize_using_config(&output_scale_file, &configs); // Memory requirements: 6.0X
        output_mag.configs.data_type = DataType::F32;

        output_scale.configs.data_type = DataType::I16;
        output_scale.configs.nodata = std::i16::MIN as f64;
        output_scale.reinitialize_values(std::i16::MIN as f64);

        let bd = Arc::new(binned_data); // wrap binned_data in an Arc

        ///////////////////////////////
        // Perform the main analysis //
        ///////////////////////////////

        for s in min_scale..(min_scale + num_steps) {
            let midpoint = min_scale
                + (((step * (s - min_scale)) as f32).powf(step_nonlinearity)).floor() as isize;
            let filter_size = midpoint * 2 + 1;
            println!(
                "Loop {} / {} ({}x{})",
                s - min_scale + 1,
                num_steps,
                filter_size,
                filter_size
            );

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let binned_data = bd.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut bin_val, mut bin_val_n, mut old_bin_val): (i64, i64, i64);
                    let (mut start_col, mut end_col, mut start_row, mut end_row): (
                        isize,
                        isize,
                        isize,
                        isize,
                    );
                    let mut m: i64;
                    let (mut n, mut n_less_than): (f64, f64);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        start_row = row - midpoint;
                        end_row = row + midpoint;
                        let mut histo: Vec<i64> = vec![];
                        old_bin_val = bin_nodata;
                        n = 0.0;
                        n_less_than = 0.0;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            bin_val = binned_data.get_value(row, col);
                            if bin_val != bin_nodata {
                                if old_bin_val != bin_nodata {
                                    // remove the trailing column from the histo
                                    for row2 in start_row..end_row + 1 {
                                        bin_val_n = binned_data.get_value(row2, col - midpoint - 1);
                                        if bin_val_n != bin_nodata {
                                            histo[bin_val_n as usize] -= 1;
                                            n -= 1.0;
                                            if bin_val_n < old_bin_val {
                                                n_less_than -= 1.0;
                                            }
                                        }
                                    }

                                    // add the leading column to the histo
                                    for row2 in start_row..end_row + 1 {
                                        bin_val_n = binned_data.get_value(row2, col + midpoint);
                                        if bin_val_n != bin_nodata {
                                            histo[bin_val_n as usize] += 1;
                                            n += 1.0;
                                            if bin_val_n < old_bin_val {
                                                n_less_than += 1.0;
                                            }
                                        }
                                    }

                                    // how many cells lie between the bins of binVal and oldBinVal?
                                    if old_bin_val < bin_val {
                                        m = 0;
                                        for v in old_bin_val..bin_val {
                                            m += histo[v as usize];
                                        }
                                        n_less_than += m as f64;
                                    } else if old_bin_val > bin_val {
                                        m = 0;
                                        for v in bin_val..old_bin_val {
                                            m += histo[v as usize];
                                        }
                                        n_less_than -= m as f64;
                                    } // otherwise they are in the same bin and there is no need to update
                                } else {
                                    // initialize the histogram
                                    histo = vec![0i64; num_bins as usize];
                                    n = 0.0;
                                    n_less_than = 0.0;
                                    start_col = col - midpoint;
                                    end_col = col + midpoint;
                                    for col2 in start_col..end_col + 1 {
                                        for row2 in start_row..end_row + 1 {
                                            bin_val_n = binned_data.get_value(row2, col2);
                                            if bin_val_n != bin_nodata {
                                                histo[bin_val_n as usize] += 1;
                                                n += 1f64;
                                                if bin_val_n < bin_val {
                                                    n_less_than += 1f64;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if n > 0f64 && bin_val != bin_nodata {
                                data[col as usize] = n_less_than / n * 100.0;
                            } else {
                                data[col as usize] = nodata;
                            }

                            old_bin_val = bin_val;
                        }
                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut z1: f64;
            let mut z2: f64;
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns {
                    if data[col as usize] != nodata {
                        z1 = output_mag.get_value(row, col);
                        if z1 == nodata {
                            output_mag.set_value(row, col, data[col as usize]);
                            output_scale.set_value(row, col, s as f64);
                        } else {
                            z1 = (z1 - 50f64).abs();
                            z2 = (data[col as usize] - 50f64).abs();
                            if z2 > z1 {
                                output_mag.set_value(row, col, data[col as usize]);
                                output_scale.set_value(row, col, midpoint as f64);
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing analysis: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            // Update progress
            if verbose {
                progress = (s as f32 / num_steps as f32 * 100f32) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(bd);

        let elapsed_time = get_formatted_elapsed_time(start);
        output_mag.configs.palette = "blue_white_red.plt".to_string();
        output_mag.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_mag.add_metadata_entry(format!("Input file: {}", input_file));
        output_mag.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_mag.add_metadata_entry(format!("Step size: {}", step));
        output_mag.add_metadata_entry(format!("Number of steps: {}", num_steps));
        output_mag.add_metadata_entry(format!("Step nonlinearity: {}", step_nonlinearity));
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

        drop(output_mag);

        output_scale.configs.palette = "relief.plt".to_string();
        output_scale.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_scale.add_metadata_entry(format!("Input file: {}", input_file));
        output_scale.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_scale.add_metadata_entry(format!("Step size: {}", step));
        output_scale.add_metadata_entry(format!("Number of steps: {}", num_steps));
        output_scale.add_metadata_entry(format!("Step nonlinearity: {}", step_nonlinearity));
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
