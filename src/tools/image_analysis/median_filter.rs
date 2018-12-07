/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 15, 2017
Last Modified: 13/10/2018
License: MIT

NOTES: This tool uses the efficient running-median filtering algorithm of Huang, Yang, and Tang (1979).
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::i64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Performs an efficient median filter based on Huang, Yang, and Tang's (1979) method.
pub struct MedianFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MedianFilter {
    /// Public constructor.
    pub fn new() -> MedianFilter {
        let name = "MedianFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a median filter on an input image.".to_string();

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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --filter=25",
            short_exe, name
        ).replace("*", &sep);

        MedianFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MedianFilter {
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
                    filter_size_x = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
                filter_size_y = filter_size_x;
            } else if flag_val == "-filterx" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            } else if flag_val == "-filtery" {
                if keyval {
                    filter_size_y = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_y = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            } else if flag_val == "-sig_digits" {
                if keyval {
                    num_sig_digits = vec[1].to_string().parse::<i32>().unwrap();
                } else {
                    num_sig_digits = args[i + 1].to_string().parse::<i32>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        // a median filter of less than 3 x 3 doesn't make sense.
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
        // let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        if is_rgb_image && num_sig_digits < 4 {
            num_sig_digits = 4;
        }

        // first bin the data
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let display_min = if !is_rgb_image {
            input.configs.display_min
        } else {
            0f64
        };
        let display_max = if !is_rgb_image {
            input.configs.display_max
        } else {
            1f64
        };
        let multiplier = 10f64.powi(num_sig_digits);
        let min_val = if !is_rgb_image {
            input.configs.minimum
        } else {
            0f64
        };
        let max_val = if !is_rgb_image {
            input.configs.maximum
        } else {
            1f64
        };
        let min_bin = (min_val * multiplier).floor() as i64;
        let num_bins = (max_val * multiplier).floor() as i64 - min_bin + 1;
        let bin_nodata = i64::MIN;
        let mut binned_data: Array2D<i64> = Array2D::new(rows, columns, bin_nodata, bin_nodata)?;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<Fn(isize, isize) -> f64> = if !is_rgb_image {
                    Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                } else {
                    Box::new(|row: isize, col: isize| -> f64 {
                        let value = input.get_value(row, col);
                        if value != nodata {
                            return value2i(value);
                        }
                        nodata
                    })
                };

                let mut z: f64;
                let mut val: i64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![bin_nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
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

        let bd = Arc::new(binned_data); // wrap binned_data in an Arc
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let binned_data = bd.clone();
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let output_fn: Box<Fn(isize, isize, f64) -> f64> = if !is_rgb_image {
                    // simply return the value.
                    Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                } else {
                    // convert it back into an rgb value, using the modified intensity value.
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            return hsi2value(h, s, value);
                        }
                        nodata
                    })
                };
                let (mut bin_val, mut bin_val_n): (i64, i64);
                let (mut start_col, mut end_col, mut start_row, mut end_row): (
                    isize,
                    isize,
                    isize,
                    isize,
                );
                let mut median: i64;
                let mut old_median: i64;
                let (mut n, mut n_less_than): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
                    let mut histo: Vec<i64> = vec![];
                    old_median = bin_nodata;
                    median = bin_nodata;
                    n = 0.0;
                    n_less_than = 0.0;
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        bin_val = binned_data.get_value(row, col);
                        if bin_val != bin_nodata {
                            if old_median != bin_nodata {
                                // remove the trailing column from the histo
                                for row2 in start_row..end_row + 1 {
                                    bin_val_n = binned_data.get_value(row2, col - midpoint_x - 1);
                                    if bin_val_n != bin_nodata {
                                        histo[bin_val_n as usize] -= 1;
                                        n -= 1.0;
                                        if bin_val_n < old_median {
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
                                        if bin_val_n < old_median {
                                            n_less_than += 1.0;
                                        }
                                    }
                                }

                                // adjust the median
                                let target = (n / 2f64).floor();
                                if n_less_than < target {
                                    // add bins
                                    for v in old_median..num_bins {
                                        if n_less_than + (histo[v as usize] as f64) >= target {
                                            median = v as i64;
                                            break;
                                        } else {
                                            n_less_than += histo[v as usize] as f64;
                                        }
                                    }
                                } else {
                                    //if n_less_than >= target { // remove bins
                                    for v in (0..old_median).rev() {
                                        if n_less_than - (histo[v as usize] as f64) >= target {
                                            n_less_than -= histo[v as usize] as f64;
                                        } else {
                                            median = v + 1;
                                            break;
                                        }
                                    }
                                } // otherwise they are in the same bin and there is no need to update
                            } else {
                                // This is the first cell in a row or after a nodata cell; initialize the histogram.
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
                                        }
                                    }
                                }
                                // calcualate the median from the histogram
                                let mut sum = 0f64;
                                let target = (n / 2f64).floor();
                                for i in 0..num_bins as usize {
                                    sum += histo[i] as f64;
                                    if sum >= target {
                                        median = i as i64;
                                        break;
                                    } else {
                                        n_less_than = sum;
                                    }
                                }
                            }

                            if n > 0f64 {
                                data[col as usize] =
                                    output_fn(row, col, (median + min_bin) as f64 / multiplier);
                            } else {
                                data[col as usize] = nodata;
                            }

                            old_median = median;
                        } else {
                            old_median = bin_nodata;
                        }
                    }
                    tx.send((row, data)).unwrap();
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
        output.configs.display_min = display_min;
        output.configs.display_max = display_max;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
        output.add_metadata_entry(format!("Num. significant digits: {}", num_sig_digits));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

#[inline]
fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    (r + g + b) / 3f64
}

#[inline]
fn value2hsi(value: f64) -> (f64, f64, f64) {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    let i = (r + g + b) / 3f64;

    let rn = r / (r + g + b);
    let gn = g / (r + g + b);
    let bn = b / (r + g + b);

    let mut h = if rn != gn || rn != bn {
        ((0.5 * ((rn - gn) + (rn - bn))) / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt())
            .acos()
    } else {
        0f64
    };
    if b > g {
        h = 2f64 * PI - h;
    }

    let s = 1f64 - 3f64 * rn.min(gn).min(bn);

    (h, s, i)
}

#[inline]
fn hsi2value(h: f64, s: f64, i: f64) -> f64 {
    let mut r: u32;
    let mut g: u32;
    let mut b: u32;

    let x = i * (1f64 - s);

    if h < 2f64 * PI / 3f64 {
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (y * 255f64).round() as u32;
        g = (z * 255f64).round() as u32;
        b = (x * 255f64).round() as u32;
    } else if h < 4f64 * PI / 3f64 {
        let h = h - 2f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (x * 255f64).round() as u32;
        g = (y * 255f64).round() as u32;
        b = (z * 255f64).round() as u32;
    } else {
        let h = h - 4f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (z * 255f64).round() as u32;
        g = (x * 255f64).round() as u32;
        b = (y * 255f64).round() as u32;
    }

    if r > 255u32 {
        r = 255u32;
    }
    if g > 255u32 {
        g = 255u32;
    }
    if b > 255u32 {
        b = 255u32;
    }

    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}
