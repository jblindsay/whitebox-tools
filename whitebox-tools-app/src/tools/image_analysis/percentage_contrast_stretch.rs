/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 13/07/2017
Last Modified: 30/01/2020
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

/// This tool performs a percentage contrast stretch on a raster image. This operation maps each grid cell value
/// in the input raster image (z<sub>in</sub>) onto a new scale that ranges from a lower-tail clip value (`min_val`)
/// to the upper-tail clip value (`max_val`), with the user-specified number of tonal values (`num_tones`), such that:
///
/// > z<sub>out</sub> = ((z<sub>in</sub> – min_val)/(max_val – min_val)) x num_tones
///
/// where z<sub>out</sub> is the output value. The values of `min_val` and `max_val` are determined from the frequency
/// distribution and the user-specified tail clip value (`--clip`). For example, if a value of 1% is specified, the tool
/// will determine the values in the input image for which 1% of the grid cells have a lower value `min_val` and 1% of
/// the grid cells have a higher value `max_val`. The user must also specify which tails (upper, lower, or both) to clip
/// (`--tail`).
///
/// This is a type of linear contrast stretch with saturation at the tails of the frequency distribution. This is
/// the same kind of stretch that is used to display raster type data on the fly in many GIS software packages,
/// such that the lower and upper tail values are set using the minimum and maximum display values and the number
/// of tonal values is determined by the number of palette entries.
///
/// # See Also
/// `PiecewiseContrastStretch`, `GaussianContrastStretch`, `HistogramEqualization`, `MinMaxContrastStretch`, `SigmoidalContrastStretch`,
/// `StandardDeviationContrastStretch`
pub struct PercentageContrastStretch {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PercentageContrastStretch {
    pub fn new() -> PercentageContrastStretch {
        // public constructor
        let name = "PercentageContrastStretch".to_string();
        let toolbox = "Image Processing Tools/Image Enhancement".to_string();
        let description =
            "Performs a percentage linear contrast stretch on input images.".to_string();

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
            name: "Distribution Tail Clip Amount (%)".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Optional amount to clip the distribution tails by, in percent."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Tail".to_owned(), 
            flags: vec!["--tail".to_owned()], 
            description: "Specified which tails to clip; options include 'upper', 'lower', and 'both' (default is 'both').".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["upper".to_owned(), "lower".to_owned(), "both".to_owned()]),
            default_value: Some("both".to_owned()),
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --clip=2.0 --tail='both' --num_tones=1024", short_exe, name).replace("*", &sep);

        PercentageContrastStretch {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PercentageContrastStretch {
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
        let mut tail = String::from("both");
        let mut clip = f64::NEG_INFINITY;
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
            } else if flag_val == "-clip" {
                if keyval {
                    clip = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    clip = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-tail" || flag_val == "-tails" {
                if keyval {
                    tail = vec[1].to_string();
                } else {
                    tail = args[i + 1].to_string();
                }
                if tail.to_lowercase().contains("u") {
                    tail = String::from("upper");
                } else if tail.to_lowercase().contains("l") {
                    tail = String::from("lower");
                } else {
                    tail = String::from("both");
                }
            } else if flag_val == "-num_tones" {
                if keyval {
                    num_tones = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    num_tones = args[i + 1]
                        .to_string()
                        .parse::<f64>()
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

        if clip < 0f64
            || (tail == "both".to_string() && clip >= 50f64)
            || (tail != "both".to_string() && clip >= 100f64)
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Incorrect clip value (correct range is 0.0 to 50.0.",
            ));
        }

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

        if verbose {
            println!("Calculating clip values...")
        };

        let (min_val, max_val) = if !is_rgb_image {
            let (a, b) = input.calculate_clip_values(clip);
            let min_val: f64;
            let max_val: f64;
            if tail == "both".to_string() {
                // let (a, b) = input.calculate_clip_values(clip);
                min_val = a;
                max_val = b;
            } else if tail == "upper".to_string() {
                // let (_, b) = input.calculate_clip_values(clip);
                min_val = input.configs.display_min;
                max_val = b;
            } else {
                // tail == lower
                // let (a, _) = input.calculate_clip_values(clip);
                min_val = a;
                max_val = input.configs.display_max;
            }
            (min_val, max_val) // return
        } else {
            // make a histogram of the itensity values
            let mut histo = vec![0usize; 1000];
            let mut min_val = f64::INFINITY;
            let mut max_val = f64::NEG_INFINITY;
            let mut n = 0f64;
            let mut bin: usize;
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
                        n += 1f64;
                        bin = (x * 999f64).floor() as usize;
                        histo[bin] += 1usize;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Calculating clip values: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            let num_cells_in_tail = (n * clip / 100f64).round() as usize;
            let mut sum = 0usize;
            let mut a = 0f64;
            let mut b = 1f64;
            for j in 0..1000 {
                sum += histo[j];
                if sum >= num_cells_in_tail {
                    a = j as f64 / 999f64;
                    break;
                }
            }
            sum = 0usize;
            for j in (0..1000).rev() {
                sum += histo[j];
                if sum >= num_cells_in_tail {
                    b = j as f64 / 999f64;
                    break;
                }
            }

            if tail == "both".to_string() {
                min_val = min_val.max(a);
                max_val = max_val.min(b);
            } else if tail == "upper".to_string() {
                max_val = max_val.min(b);
            } else {
                // tail == lower
                min_val = min_val.max(a);
            }

            (min_val, max_val) // return
        };

        let value_range = max_val - min_val;
        if value_range < 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "The calculated clip values ({}, {}) are incorrect.",
                    min_val, max_val
                ),
            ));
        }

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
                    Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                } else {
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            let ret = hsi2value(h, s, value / num_tones);
                            return ret;
                        }
                        nodata
                    })
                };

                let mut z_in: f64;
                let mut z_out: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = input_fn(row, col);
                        if z_in != nodata {
                            z_out = ((z_in - min_val) / value_range * num_tones).floor();
                            if z_out < 0f64 {
                                z_out = 0f64;
                            }
                            if z_out >= num_tones {
                                z_out = num_tones - 1f64;
                            }
                            data[col as usize] = output_fn(row, col, z_out);
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
                    println!("Progress: {}%", progress);
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
        output.add_metadata_entry(format!("Percentage clip value: {}", clip));
        output.add_metadata_entry(format!("Clipped tails: {}", tail));
        output.add_metadata_entry(format!("Number of tones: {}", num_tones));
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
