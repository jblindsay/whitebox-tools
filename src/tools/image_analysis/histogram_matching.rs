/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 14, 2017
Last Modified: September 14, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use tools::WhiteboxTool;

pub struct HistogramMatching {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl HistogramMatching {
    pub fn new() -> HistogramMatching {
        // public constructor
        let name = "HistogramMatching".to_string();

        let description = "Alters the statistical distribution of a raster image matching it to a specified cdf."
            .to_string();

        let mut parameters = "-i, --input    Input raster file to modify.\n".to_owned();
        parameters.push_str("--histo_file   Input reference pdf text file.\n");
        parameters.push_str("-o, --output   Output raster file.\n");

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input1.dep --histo_file=histo.txt -o=output.dep", short_exe, name).replace("*", &sep);

        HistogramMatching {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HistogramMatching {
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
        let mut histo_file = String::new();
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-histo_file" || vec[0].to_lowercase() == "--histo_file" {
                if keyval {
                    histo_file = vec[1].to_string();
                } else {
                    histo_file = args[i + 1].to_string();
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

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !histo_file.contains(&sep) {
            histo_file = format!("{}{}", working_directory, histo_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading input data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        if input.configs.data_type == DataType::RGB24 ||
            input.configs.data_type == DataType::RGB48 ||
            input.configs.data_type == DataType::RGBA32 ||
            input.configs.photometric_interp == PhotometricInterpretation::RGB {

            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool is for single-band greyscale images and cannot be applied to RGB colour-composite images."));
        }
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let min_value = input.configs.minimum;
        let max_value = input.configs.maximum;
        let num_bins = ((max_value - min_value).max(1024f64)).ceil() as usize; 
        let bin_size = (max_value - min_value) / num_bins as f64;
        let mut histogram = vec![0f64; num_bins];
        let num_bins_less_one = num_bins - 1;
        let mut z: f64;
        let mut numcells: f64 = 0f64;
        let mut bin_num;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    numcells += 1f64;
                    bin_num = ((z - min_value) / bin_size) as usize;
                    if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                    histogram[bin_num] += 1f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 1 of 3: {}%", progress);
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

        // Get the reference distribution from the input file.
        let mut reference_cdf: Vec<Vec<f64>> = vec![];
        let f = File::open(histo_file.clone())?;
        let f = BufReader::new(f);
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            let mut v: Vec<&str> = line_unwrapped.split(";").collect();
            if v.len() < 2 { // delimiter can be a semicolon, comma, space, or tab.
                v = line_unwrapped.split(",").collect();
                if v.len() < 2 {
                    v = line_unwrapped.split(" ").collect();
                    if v.len() < 2 {
                        v = line_unwrapped.split("\t").collect();
                    }
                }
            }
            if v.len() == 2 {
                let x = v[0].parse().unwrap();
                let f = v[1].parse().unwrap();
                reference_cdf.push(vec![x, f]);
            } else if v.len() > 2 && v.len() % 2 == 0 {
                // it's probably a matter of
            }
        }

        // convert the reference histogram to a cdf.
        let num_lines = reference_cdf.len();
        for i in 1..num_lines {
            reference_cdf[i][1] += reference_cdf[i - 1][1];
        }
        let total_frequency = reference_cdf[num_lines - 1][1];
        for i in 0..num_lines {
            reference_cdf[i][1] = reference_cdf[i][1] / total_frequency;
        }

        println!("Hello world");
        
        let mut starting_vals = [0usize; 11];
        let mut p_val: f64;
        for i in 0..num_lines {
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

        let starting_vals = Arc::new(starting_vals);
        let reference_cdf = Arc::new(reference_cdf);
        let cdf = Arc::new(cdf);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let starting_vals = starting_vals.clone();
            let reference_cdf = reference_cdf.clone();
            let cdf = cdf.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut bin_num: usize;
                let mut j: usize;
                let mut x_val = 0f64;
                let mut p_val: f64;
                let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            bin_num = ((z - min_value) / bin_size) as usize;
                            if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                            p_val = cdf[bin_num];
                            j = ((p_val * 10f64).floor()) as usize;
                            for i in starting_vals[j]..num_lines {
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
                            data[col as usize] = x_val;
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
                    println!("Loop 3 of 3: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file to modify: {}", input_file));
        output.add_metadata_entry(format!("Input reference file: {}", histo_file));
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
