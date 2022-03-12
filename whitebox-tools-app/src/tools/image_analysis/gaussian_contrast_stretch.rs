/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/05/2018
Last Modified: 22/10/2019
License: MIT

NOTES: 1. The tool should be updated to take multiple file inputs.
       2. Unlike the original Whitebox GAT tool that this is based on,
          this tool will operate on RGB images in addition to greyscale images.
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool performs a Gaussian stretch on a raster image. The observed histogram of the input image is fitted
/// to a Gaussian histogram, i.e. normal distribution. A histogram matching technique is used to map the values from
/// the input image onto the output Gaussian distribution. The user must the number of tones (`--num_tones`) used in the
/// output image.
///
/// This tool is related to the more general `HistogramMatching` tool, which can be used to fit any frequency distribution
/// to an input image, and other contrast enhancement tools such as `HistogramEqualization`, `MinMaxContrastStretch`,
/// `PercentageContrastStretch`, `SigmoidalContrastStretch`, and `StandardDeviationContrastStretch`.
///
/// # See Also
/// `PiecewiseContrastStretch`, `HistogramEqualization`, `MinMaxContrastStretch`, `PercentageContrastStretch`, `SigmoidalContrastStretch`,
/// `StandardDeviationContrastStretch`, `HistogramMatching`
pub struct GaussianContrastStretch {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl GaussianContrastStretch {
    pub fn new() -> GaussianContrastStretch {
        // public constructor
        let name = "GaussianContrastStretch".to_string();
        let toolbox = "Image Processing Tools/Image Enhancement".to_string();
        let description = "Performs a Gaussian contrast stretch on input images.".to_string();

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
            name: "Number of Tones".to_owned(),
            flags: vec!["--num_tones".to_owned()],
            description: "Number of tones in the output image.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("256".to_owned()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --num_tones=1024",
            short_exe, name
        )
        .replace("*", &sep);

        GaussianContrastStretch {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for GaussianContrastStretch {
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
        let mut num_tones = 256f64;

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
            } else if flag_val == "-num_tones" {
                num_tones = if keyval {
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
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if num_tones < 16f64 {
            println!("Warning: The output number of greytones must be at least 16. The value has been modified.");
            num_tones = 16f64;
        }

        let num_tones_int = num_tones.ceil() as usize;
        let num_tones_less_one = num_tones - 1f64;

        if verbose {
            println!("Reading input data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        if input.configs.data_type == DataType::RGB48 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "This tool cannot be applied to 48-bit RGB colour-composite images.",
            ));
        }

        let start = Instant::now();

        // Get the min and max values
        if verbose {
            println!("Calculating min and max values...")
        };

        let (min_val, max_val) = if !is_rgb_image {
            (input.configs.minimum, input.configs.maximum) // return
        } else {
            let mut min_val = f64::INFINITY;
            let mut max_val = f64::NEG_INFINITY;
            let mut value: f64;
            let mut x: f64;
            for row in 0..rows {
                for col in 0..columns {
                    value = input.get_value(row, col);
                    if value != nodata {
                        x = value2i(value); // gets the intensity
                        if x < min_val {
                            min_val = x;
                        }
                        if x > max_val {
                            max_val = x;
                        }
                    }
                }
            }
            (min_val, max_val) // return
        };

        // Get the input file distribution
        let num_bins = ((max_val - min_val).max(2048f64)).ceil() as usize;
        let bin_size = (max_val - min_val) / num_bins as f64;
        let mut histogram = vec![0f64; num_bins];
        let num_bins_less_one = num_bins - 1;
        let mut z: f64;
        let mut numcells: f64 = 0f64;
        let mut bin_num;
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata {
                    if is_rgb_image {
                        z = value2i(z); // gets the intensity
                    }
                    numcells += 1f64;
                    bin_num = ((z - min_val) / bin_size) as usize;
                    if bin_num > num_bins_less_one {
                        bin_num = num_bins_less_one;
                    }
                    histogram[bin_num] += 1f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 1 of 2: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut cdf = vec![0f64; histogram.len()];
        cdf[0] = histogram[0];
        for i in 1..cdf.len() {
            cdf[i] = cdf[i - 1] + histogram[i];
        }
        for i in 0..cdf.len() {
            cdf[i] = cdf[i] / numcells;
        }

        // Create the reference distribution
        let mut reference_cdf: Vec<Vec<f64>> = vec![];
        let p_step = 6f64 / (num_tones - 1f64);
        for a in 0..num_tones_int {
            let x = -3.0 + a as f64 * p_step;
            // Use the standard form (μ = 0.0, σ = 1.0) of:
            // (1 / sqrt(2σ^2 * π)) * e^(-(x - μ)^2 / 2σ^2)
            let p = (1f64 / (2f64 * PI).sqrt()) * (-x.powi(2) / 2f64).exp();
            reference_cdf.push(vec![x, p]);
        }

        // convert the reference histogram to a cdf.
        for i in 1..num_tones_int {
            reference_cdf[i][1] += reference_cdf[i - 1][1];
        }
        let total_frequency = reference_cdf[num_tones_int - 1][1];
        for i in 0..num_tones_int {
            reference_cdf[i][1] = reference_cdf[i][1] / total_frequency;
        }

        let mut starting_vals = [0usize; 11];
        let mut p_val: f64;
        for i in 0..num_tones_int {
            p_val = reference_cdf[i][1];
            if p_val < 0.1 {
                starting_vals[1] = i;
            }
            if p_val < 0.2 {
                starting_vals[2] = i;
            }
            if p_val < 0.3 {
                starting_vals[3] = i;
            }
            if p_val < 0.4 {
                starting_vals[4] = i;
            }
            if p_val < 0.5 {
                starting_vals[5] = i;
            }
            if p_val < 0.6 {
                starting_vals[6] = i;
            }
            if p_val < 0.7 {
                starting_vals[7] = i;
            }
            if p_val < 0.8 {
                starting_vals[8] = i;
            }
            if p_val < 0.9 {
                starting_vals[9] = i;
            }
            if p_val <= 1f64 {
                starting_vals[10] = i;
            }
        }

        // Perform the contrast stretch
        let starting_vals = Arc::new(starting_vals);
        let reference_cdf = Arc::new(reference_cdf);
        let cdf = Arc::new(cdf);

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let starting_vals = starting_vals.clone();
            let reference_cdf = reference_cdf.clone();
            let cdf = cdf.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<dyn Fn(isize, isize) -> f64> = if !is_rgb_image {
                    Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                } else {
                    Box::new(|row: isize, col: isize| -> f64 {
                        let value = input.get_value(row, col);
                        if value != nodata {
                            let v = value2i(value);
                            return v;
                        }
                        nodata
                    })
                };

                let output_fn: Box<dyn Fn(isize, isize, f64) -> f64> = if !is_rgb_image {
                    Box::new(|_: isize, _: isize, value: f64| -> f64 {
                        ((value + 3f64) / 6f64 * num_tones_less_one).round()
                    })
                } else {
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            let ret = hsi2value(h, s, (value + 3f64) / 6f64);
                            return ret;
                        }
                        nodata
                    })
                };

                let mut z: f64;
                let mut bin_num: usize;
                let mut j: usize;
                let mut x_val = 0f64;
                let mut p_val: f64;
                let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
                        if z != nodata {
                            bin_num = ((z - min_val) / bin_size) as usize;
                            if bin_num > num_bins_less_one {
                                bin_num = num_bins_less_one;
                            }
                            p_val = cdf[bin_num];
                            j = ((p_val * 10f64).floor()) as usize;
                            for i in starting_vals[j]..num_tones_int {
                                if reference_cdf[i][1] > p_val {
                                    if i > 0 {
                                        x1 = reference_cdf[i - 1][0];
                                        x2 = reference_cdf[i][0];
                                        p1 = reference_cdf[i - 1][1];
                                        p2 = reference_cdf[i][1];
                                        if p1 != p2 {
                                            x_val = x1 + ((x2 - x1) * ((p_val - p1) / (p2 - p1)));
                                        } else {
                                            x_val = x1;
                                        }
                                    } else {
                                        x_val = reference_cdf[i][0];
                                    }
                                    break;
                                }
                            }
                            data[col as usize] = output_fn(row, col, x_val);
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 2 of 2: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Number of tones: {}", num_tones_int));
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

fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    (r + g + b) / 3f64
}

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
