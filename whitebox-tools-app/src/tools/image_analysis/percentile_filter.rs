/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 03/05/2019
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

/// This tool calculates the percentile of the center cell in a moving filter window applied to an input image (`--input).
/// This indicates the value below which a given percentage of the neighbouring values in within the filter fall. For example,
/// the 35th percentile is the value below which 35% of the neighbouring values in the filter window may be found. As such,
/// the percentile of a pixel value is indicative of the relative location of the site within the statistical distribution
/// of values contained within a filter window. When applied to input digital elevation models, percentile is a measure of
/// local topographic position, or elevation residual.
///
/// Neighbourhood size, or filter size, is specified in the x and y dimensions using the `--filterx` and `--filtery` flags.
/// These dimensions should be odd, positive integer values, e.g. 3, 5, 7, 9... If the kernel filter size is the same in
/// the x and y dimensions, the silent `--filter` flag may be used instead (command-line interface only).
///
/// This tool takes advantage of the redundancy between overlapping, neighbouring filters to enhance computationally
/// efficiency, using a method similar to Huang et al. (1979). This efficient method of calculating percentiles requires
/// rounding of floating-point inputs, and therefore the user must specify the number of significant digits (`--sig_digits`)
/// to be used during the processing. Like most of WhiteboxTools' filters, this tool is also parallelized for further efficiency.
///
/// # Reference
/// Huang, T., Yang, G.J.T.G.Y. and Tang, G., 1979. A fast two-dimensional median filtering algorithm. IEEE
/// Transactions on Acoustics, Speech, and Signal Processing, 27(1), pp.13-18.
///
/// # See Also
/// `MedianFilter`
pub struct PercentileFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PercentileFilter {
    /// Public constructor.
    pub fn new() -> PercentileFilter {
        let name = "PercentileFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a percentile filter on an input image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --filter=25",
            short_exe, name
        )
        .replace("*", &sep);

        PercentileFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PercentileFilter {
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
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-filter" {
                if keyval {
                    filter_size_x = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_x = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
                filter_size_y = filter_size_x;
            } else if flag_val == "-filterx" {
                if keyval {
                    filter_size_x = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_x = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
            } else if flag_val == "-filtery" {
                if keyval {
                    filter_size_y = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_y = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
            } else if flag_val == "-sig_digits" {
                if keyval {
                    num_sig_digits = vec[1]
                        .to_string()
                        .parse::<i32>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    num_sig_digits = args[i + 1]
                        .to_string()
                        .parse::<i32>()
                        .expect(&format!("Error parsing {}", flag_val));
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
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let multiplier = 10f64.powi(num_sig_digits);
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        let min_bin = (min_val * multiplier).floor() as i64;
        let num_bins = (max_val * multiplier).floor() as i64 - min_bin + 1;
        let bin_nodata = i64::MIN;
        let mut binned_data: Array2D<i64> = Array2D::new(rows, columns, bin_nodata, bin_nodata)?;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
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
                    tx.send((row, data)).unwrap();
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

        let bd = Arc::new(binned_data); // wrap binned_data in an Arc
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let binned_data = bd.clone();
            let tx = tx.clone();
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

                        if n > 0f64 {
                            data[col as usize] = n_less_than / n * 100.0;
                        } else {
                            data[col as usize] = nodata;
                        }

                        old_bin_val = bin_val;
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
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
