/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: June 27, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::cmp::Ordering::Less;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct KNearestMeanFilter {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl KNearestMeanFilter {
    pub fn new() -> KNearestMeanFilter { // public constructor
        let name = "KNearestMeanFilter".to_string();
        
        let description = "A k-nearest mean filter is a type of edge-preserving smoothing filter.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--filter      Size of the filter kernel (default is 3).\n");
        parameters.push_str("--filterx     Optional size of the filter kernel in the x-direction (default is 3; not used if --filter is specified).\n");
        parameters.push_str("--filtery     Optional size of the filter kernel in the y-direction (default is 3; not used if --filter is specified).\n");
        parameters.push_str("-k            k-value in pixels (default is 5); this is the number of nearest-valued neighbours to use.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --filter=9 -k=5
>>.*{0} -r={1} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --filtery=7 --filtery=9  -k=5", short_exe, name).replace("*", &sep);
    
        KNearestMeanFilter { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for KNearestMeanFilter {
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
        let mut filter_size_x = 3usize;
        let mut filter_size_y = 3usize;
        let mut k = 5usize;
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
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<usize>().unwrap();
                }
                filter_size_y = filter_size_x;
            } else if vec[0].to_lowercase() == "-filterx" || vec[0].to_lowercase() == "--filterx" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<usize>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-filtery" || vec[0].to_lowercase() == "--filtery" {
                if keyval {
                    filter_size_y = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size_y = args[i + 1].to_string().parse::<usize>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-k" || vec[0].to_lowercase() == "--k" {
                if keyval {
                    k = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    k = args[i + 1].to_string().parse::<usize>().unwrap();
                }
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
        
        k += 1; // Let's assume that we want k neighbours, or k + 1 including the centre cell itself.
        if k > filter_size_x * filter_size_y {
            println!("The value of k cannot be greater than the size of the filter (i.e. filterx * filtery). The value has been changed."); 
            k = filter_size_x * filter_size_y;
        }

        let midpoint_x = (filter_size_x as f64 / 2f64).floor() as isize;
        let midpoint_y = (filter_size_y as f64 / 2f64).floor() as isize;
        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        
        let (tx, rx) = mpsc::channel();
        let num_procs = num_cpus::get() as isize;
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut n: f64;
                let mut sum: f64;
                let mut z: f64;
                let mut zn: f64;
                let mut dx = vec![];
                let mut dy = vec![];
                for r in 0..filter_size_y {
                    for c in 0..filter_size_x {
                        dx.push(c as isize - midpoint_x);
                        dy.push(r as isize - midpoint_y);
                    }
                }
                let num_cells = dx.len();
                let mut neighbour_vals = vec![0f64; num_cells];
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            for i in 0..num_cells {
                                zn = input[(row + dy[i], col + dx[i])];
                                if zn != nodata {
                                    neighbour_vals[i] = zn;
                                } else {
                                    neighbour_vals[i] = f64::INFINITY;
                                }
                            }
                            // sort the list of neighbours by their squared difference with z
                            neighbour_vals.sort_by(|a, b| ((a - z)*(a - z)).partial_cmp(&((b - z)*(b - z))).unwrap_or(Less));

                            sum = 0f64;
                            n = 0f64;
                            for i in 0..k {
                                if neighbour_vals[i] != f64::INFINITY {
                                    sum += neighbour_vals[i];
                                    n += 1f64;
                                } 
                            }
                            if n > 0f64 {
                                data[col as usize] = sum / n;
                            }
                        }
                    }

                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.palette = input.configs.palette.clone();
        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
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
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
        output.add_metadata_entry(format!("k-value: {}", k));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => { if verbose { println!("Output file written"); } },
            Err(e) => return Err(e),
        };

        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}
