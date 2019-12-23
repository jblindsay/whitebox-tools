/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 02/04/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
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

/// Elevation percentile (EP) is a measure of local topographic position (LTP). It expresses the vertical 
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
/// and 100%. Quantile-based estimates (e.g., the median and interquartile
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
/// Neighbourhood size, or filter size, is specified in the x and y dimensions using the `--filterx` and `--filtery`flags. 
/// These dimensions should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.).
/// 
/// # References
/// Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic position 
/// for multiscale geomorphometric analysis. Geomorphology, 312, 40-50.
/// 
/// Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE 
/// Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18.
/// 
/// # See Also
/// `DevFromMeanElev`, `DiffFromMeanElev`
pub struct ElevPercentile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ElevPercentile {
    pub fn new() -> ElevPercentile {
        // public constructor
        let name = "ElevPercentile".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates the elevation percentile raster from a DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned(), "--dem".to_owned()],
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
            name: "Filter X-Dimension".to_owned(),
            flags: vec!["--filterx".to_owned()],
            description: "Size of the filter kernel in the x-direction.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Filter Y-Dimension".to_owned(),
            flags: vec!["--filtery".to_owned()],
            description: "Size of the filter kernel in the y-direction.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Number of Significant Digits".to_owned(),
            flags: vec!["--sig_digits".to_owned()],
            description: "Number of significant digits.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("2".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --filter=25",
            short_exe, name
        )
        .replace("*", &sep);

        ElevPercentile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ElevPercentile {
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
        let mut filter_size_x = 11usize;
        let mut filter_size_y = 11usize;
        let mut num_sig_digits = 2i32;
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
                filter_size_x = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
                filter_size_y = filter_size_x;
            } else if flag_val == "-filterx" {
                filter_size_x = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
            } else if flag_val == "-filtery" {
                filter_size_y = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
            } else if flag_val == "-sig_digits" {
                num_sig_digits = if keyval {
                    vec[1].to_string().parse::<i32>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<i32>().unwrap()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if filter_size_x < 3 {
            filter_size_x = 3;
        }
        if filter_size_y < 3 {
            filter_size_y = 3;
        }

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size_x as f64 / 2f64).floor() == (filter_size_x as f64 / 2f64) {
            filter_size_x += 1;
        }
        if (filter_size_y as f64 / 2f64).floor() == (filter_size_y as f64 / 2f64) {
            filter_size_y += 1;
        }

        // let (mut z, mut z_n): (f64, f64);
        let midpoint_x = (filter_size_x as f64 / 2f64).floor() as isize;
        let midpoint_y = (filter_size_y as f64 / 2f64).floor() as isize;
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
        let bin_nodata = i64::MIN;
        let mut binned_data: Array2D<i64> = Array2D::new(rows, columns, bin_nodata, bin_nodata)?;

        let num_procs = num_cpus::get() as isize;
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
            let data = rx.recv().unwrap();
            binned_data.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Binning data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input);

        let bd = Arc::new(binned_data); // wrap binned_data in an Arc
        let mut output = Raster::initialize_using_config(&output_file, &configs);
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
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
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
                                    bin_val_n = binned_data.get_value(row2, col - midpoint_x - 1);
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
                                    bin_val_n = binned_data.get_value(row2, col + midpoint_x);
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
                                start_col = col - midpoint_x;
                                end_col = col + midpoint_x;
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

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = 0.0;
        output.configs.display_max = 100.0;
        output.configs.palette = "blue_white_red.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
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
