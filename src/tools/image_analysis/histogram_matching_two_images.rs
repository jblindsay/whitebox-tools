/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: August 31, 2017
Last Modified: August 31, 2017
License: MIT
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

pub struct HistogramMatchingTwoImages {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl HistogramMatchingTwoImages {
    pub fn new() -> HistogramMatchingTwoImages {
        // public constructor
        let name = "HistogramMatchingTwoImages".to_string();

        let description = "This tool alters the cumululative distribution function of a raster image to that of another image."
            .to_string();

        let mut parameters = "--i1, --input1   Input raster file to modify.\n".to_owned();
        parameters.push_str("--i2, --input2   Input reference raster file.\n");
        parameters.push_str("-o, --output    Output raster file.\n");

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=input1.dep --i2=input2.dep -o=output.dep", short_exe, name).replace("*", &sep);

        HistogramMatchingTwoImages {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HistogramMatchingTwoImages {
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
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
        
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
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        if !input_file1.contains(&sep) {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input data...")
        };
        let input1 = Arc::new(Raster::new(&input_file1, "r")?);
        let rows1 = input1.configs.rows as isize;
        let columns1 = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;

        let min_value1 = input1.configs.minimum;
        let max_value1 = input1.configs.maximum;
        let num_bins1 = (2f64 * (int)(max_value1 - min_value1 + 1f64).ceil(), 
                ((rows1 * cols1).powf(1f64 / 3f64)).ceil()).max() as usize;
        let bin_size = (maxValue1 - minValue1) / num_bins1 as f64;
        let histogram = vec![0f64; num_bins1];
        let mut bin_num;
        let num_bins_less_one1 = num_bins1 - 1;
        
        updateProgress("Loop 1 of 3: ", 0);
        for (row = 0; row < rows1; row++) {
            data = inputFile1.getRowValues(row);
            for (col = 0; col < cols1; col++) {
                z = data[col];
                if (z != noData1) {
                    numCells1++;
                    binNum = (int)((z - minValue1) / binSize);
                    if (binNum > numBinsLessOne1) { binNum = numBinsLessOne1; }
                    histogram[binNum]++;
                }

            }
            if (cancelOp) { cancelOperation(); return; }
            progress = (float) (100f * row / (rows1 - 1));    
            updateProgress("Loop 1 of 3: ", (int)progress);
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let cdf = cdf.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let num_cells_less_one = n - min_nonempty_bin; //n - 1f64;
                let mut z_in: f64;
                let mut z_out: f64;
                let mut bin: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = input[(row, col)];
                        if z_in != nodata {
                            bin = input_fn(row, col);
                            z_out = ((cdf[bin] - min_nonempty_bin) / num_cells_less_one * num_tones_less_one).round();
                            data[col as usize] = output_fn(row, col, z_out);
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }





        let input2 = Arc::new(Raster::new(&input_file2, "r")?);
        let rows2 = input2.configs.rows as isize;
        let columns2 = input2.configs.columns as isize;
        let nodata2 = input2.configs.nodata;

        if input.configs.data_type == DataType::RGB24 ||
            input.configs.data_type == DataType::RGB48 ||
            input.configs.data_type == DataType::RGBA32 ||
            input.configs.photometric_interp == PhotometricInterpretation::RGB {

            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool is for single-band greyscale images and cannot be applied to RGB colour-composite images."));
        }

        let start = time::now();

        if verbose {
            println!("Calculating clip values...")
        };
        
        // create the histogram
        let mut num_bins = 1024usize;
        let mut histo = vec![0f64; num_bins];
        let mut n = 0f64;
        let min_value: f64;
        let range: f64;
        let bin_size: f64;
        if !is_rgb_image {    
            min_value = input.configs.minimum;
            range = input.configs.maximum - min_value;
            if range.round() > num_bins { 
                num_bins = range.round(); 
                histo = vec![0f64; num_bins];
            }
            bin_size = range / (num_bins - 1) as f64;
            let mut bin: usize;
            let mut value: f64;
            for row in 0..rows {
                for col in 0..columns {
                    value = input.get_value(row, col);
                    if value != nodata {
                        n += 1f64;
                        bin = ((value - min_value) / bin_size).floor() as usize;
                        histo[bin] += 1f64;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Calculating histogram: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            min_value = 0f64;
            range = 1f64;
            bin_size = range / (num_bins - 1) as f64;
            let mut value: f64;
            let mut v: f64;
            let mut bin: usize;
            for row in 0..rows {
                for col in 0..columns {
                    value = input.get_value(row, col);
                    if value != nodata {
                        v = value2i(value);
                        n += 1f64;
                        bin = (v * (num_bins - 1) as f64).floor() as usize;
                        histo[bin] += 1f64;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Calculating histogram: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let mut cdf = vec![0f64; histo.len()];
        let min_nonempty_bin = histo[0];
        cdf[0] = histo[0];
        for j in 1..histo.len() {
            cdf[j] = cdf[j - 1] + histo[j];
        }

        let cdf = Arc::new(cdf); // wrap the cdf in an arc

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let cdf = cdf.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<Fn(isize, isize) -> usize> = 
                    if !is_rgb_image {
                        Box::new(|row: isize, col: isize| -> usize { 
                            let x = input.get_value(row, col);
                            ((x - min_value) / bin_size).floor() as usize
                        })
                    } else {
                        Box::new(
                        |row: isize, col: isize| -> usize {
                            let value = input.get_value(row, col);
                            let x = value2i(value);
                            ((x - min_value) / bin_size).floor() as usize
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
                                // convert the value into an rgb value based on modified hsi values.
                                let (h, s, _) = value2hsi(input.get_value(row, col));
                                let ret = hsi2value(h, s, value / num_tones_less_one);
                                return ret;
                            }
                            nodata
                        }
                        )
                    };

                let num_cells_less_one = n - min_nonempty_bin; //n - 1f64;
                let mut z_in: f64;
                let mut z_out: f64;
                let mut bin: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = input[(row, col)];
                        if z_in != nodata {
                            bin = input_fn(row, col);
                            z_out = ((cdf[bin] - min_nonempty_bin) / num_cells_less_one * num_tones_less_one).round();
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