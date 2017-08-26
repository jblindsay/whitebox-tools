/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 13, 2017
Last Modified: August 26, 2017
License: MIT

NOTES: 1. The tool should be updated to take multiple file inputs.
       2. Unlike the original Whitebox GAT tool that this is based on, 
          this tool will operate on RGB images in addition to greyscale images.
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::f64::consts::PI;
use raster::*;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use tools::WhiteboxTool;

pub struct PercentageContrastStretch {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl PercentageContrastStretch {
    pub fn new() -> PercentageContrastStretch {
        // public constructor
        let name = "PercentageContrastStretch".to_string();

        let description = "Performs a percentage linear contrast stretch on input images."
            .to_string();

        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--clip        Clip size in percentage (default is 1.0).\n");
        parameters.push_str("--tail        Specified which tails to clip; options include 'upper', 'lower', and 'both' (default is 'both').\n");
        parameters.push_str("--num_tones   Number of tones in the output image (default is 256).\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.dep -o=output.dep --clip=2.0 --tail='both' --num_tones=1024", short_exe, name).replace("*", &sep);

        PercentageContrastStretch {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PercentageContrastStretch {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut tail = String::from("both");
        let mut clip = f64::NEG_INFINITY;
        let mut num_tones = 256f64;

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-clip" || vec[0].to_lowercase() == "--clip" {
                if keyval {
                    clip = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    clip = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-tail" || vec[0].to_lowercase() == "--tail"
                      || vec[0].to_lowercase() == "--tails" {
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
            } else if vec[0].to_lowercase() == "-num_tones" ||
                      vec[0].to_lowercase() == "--num_tones" {
                if keyval {
                    num_tones = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    num_tones = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if num_tones < 16f64 {
            println!("Warning: The output number of greytones must be at least 16. The value has been modified.");
            num_tones = 16f64;
        }

        if clip < 0f64 || (tail == "both".to_string() && clip >= 50f64) ||
           (tail != "both".to_string() && clip >= 100f64) {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Incorrect clip value (correct range is 0.0 to 50.0."));
        }

        if verbose {
            println!("Reading input data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let is_rgb_image = 
            if input.configs.data_type == DataType::RGB24 ||
                input.configs.data_type == DataType::RGBA32 ||
                input.configs.photometric_interp == PhotometricInterpretation::RGB {
                
                true
            } else {
                false
            };

        if input.configs.data_type == DataType::RGB48 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool cannot be applied to 48-bit RGB colour-composite images."));
        }

        let start = time::now();

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
                        if x < min_val { min_val = x; }
                        if x > max_val { max_val = x; }
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
            return Err(Error::new(ErrorKind::InvalidInput,
                                  format!("The calculated clip values ({}, {}) are incorrect.", min_val, max_val)));
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<Fn(isize, isize) -> f64> = 
                    if !is_rgb_image {
                        Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                    } else {
                        Box::new(
                        |row: isize, col: isize| -> f64 {
                            let value = input.get_value(row, col);
                            if value != nodata {
                                let v = value2i(value);
                                return v;
                            }
                            nodata
                        }
                        )
                    };
                
                let output_fn: Box<Fn(isize, isize, f64) -> f64> = 
                    if !is_rgb_image {
                        Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                    } else {
                        Box::new(
                        |row: isize, col: isize, value: f64| -> f64 {
                            if value != nodata {
                                let (h, s, _) = value2hsi(input.get_value(row, col));
                                let ret = hsi2value(h, s, value / num_tones);
                                return ret;
                            }
                            nodata
                        }
                        )
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
            let (row, data) = rx.recv().unwrap();
            output.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Percentage clip value: {}", clip));
        output.add_metadata_entry(format!("Clipped tails: {}", tail));
        output.add_metadata_entry(format!("Number of tones: {}", num_tones));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

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

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

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
	    ((0.5 * ((rn - gn) + (rn - bn))) / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt()).acos()
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
    
    if r > 255u32 { r = 255u32; }
	if g > 255u32 { g = 255u32; }
	if b > 255u32 { b = 255u32; }

    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}