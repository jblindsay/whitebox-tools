/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 13, 2017
Last Modified: August 26, 2017
License: MIT

NOTES: 1. The tool should be updated to take multiple file inputs.
       2. Unlike the original Whitebox GAT tool that this is based on, 
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

pub struct GammaCorrection {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl GammaCorrection {
    pub fn new() -> GammaCorrection { // public constructor
        let name = "GammaCorrection".to_string();
        
        let description = "Performs a sigmoidal contrast stretch on input images.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--gamma       Gamma value (default is 0.5).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=input.dep -o=output.dep --gamma=0.5", short_exe, name).replace("*", &sep);
    
        GammaCorrection { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for GammaCorrection {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut gamma = 0.5;
        
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
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-gamma" || vec[0].to_lowercase() == "--gamma" {
                if keyval {
                    gamma = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    gamma = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if verbose { println!("Reading input data...") };
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

        if gamma < 0.0 { gamma = 0f64; }
        if gamma > 4.0 { gamma = 4f64; }
        
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
                                let ret = hsi2value(h, s, value / 1f64.powf(gamma));
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
                            z_out = z_in.powf(gamma);
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
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Gamma value: {}", gamma));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

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