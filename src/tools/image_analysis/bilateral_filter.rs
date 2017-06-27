/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: June 27, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::f64::consts::PI;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct BilateralFilter {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl BilateralFilter {
    pub fn new() -> BilateralFilter { // public constructor
        let name = "BilateralFilter".to_string();
        
        let description = "A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--sigma_dist  Standard deviation in distance in pixels (default is 0.75).\n");
        parameters.push_str("--sigma_int   Standard deviation in intensity in pixels (default is 1.0).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --sigma_dist=2.5 --sigma_int=4.0", short_exe, name).replace("*", &sep);
    
        BilateralFilter { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for BilateralFilter {
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
        let mut filter_size = 0usize;
        let mut sigma_dist = 0.75;
        let mut sigma_int = 1.0;
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
            } else if vec[0].to_lowercase() == "-sigma_dist" || vec[0].to_lowercase() == "--sigma_dist" {
                if keyval {
                    sigma_dist = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma_dist = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-sigma_int" || vec[0].to_lowercase() == "--sigma_int" {
                if keyval {
                    sigma_int = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma_int = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if sigma_dist < 0.5 {
            sigma_dist = 0.5;
        } else if sigma_dist > 20.0 {
            sigma_dist = 20.0;
        }

        if sigma_int < 0.001 {
            sigma_int = 0.001;
        }

        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma_dist);
        let two_sigma_sqr_d = 2.0 * sigma_dist * sigma_dist;

        let recip_root_2_pi_times_sigma_i = 1.0 / ((2.0 * PI).sqrt() * sigma_int);
        let two_sigma_sqr_i = 2.0 * sigma_int * sigma_int;

        // figure out the size of the filter
        let mut weight: f64;
        for i in 0..250 {
            weight = recip_root_2_pi_times_sigma_d * (-1.0 * ((i * i) as f64) / two_sigma_sqr_d).exp();
            if weight <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }
        
        // the filter dimensions must be odd numbers such that there is a middle pixel
        if filter_size % 2 == 0 {
            filter_size += 1;
        }

        if filter_size < 3 { filter_size = 3; }

        let num_pixels_in_filter = filter_size * filter_size;
        let mut dx = vec![0isize; num_pixels_in_filter];
        let mut dy = vec![0isize; num_pixels_in_filter];
        let mut weights_d = vec![0.0; num_pixels_in_filter];
        
        // fill the filter d_x and d_y values and the distance-weights
        let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize + 1;
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size {
            for col in 0..filter_size {
                x = col as isize - midpoint;
                y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                weight = recip_root_2_pi_times_sigma_d * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                weights_d[a] = weight;
                a += 1;
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights_d = Arc::new(weights_d);
        
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
    
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let num_procs = num_cpus::get() as isize;
        let row_block_size = rows / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut starting_row;
        let mut ending_row = 0;
        let mut id = 0;
        while ending_row < rows {
            let input_data = input.clone();
            let dx = dx.clone();
            let dy = dy.clone();
            let weights_d = weights_d.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut sum, mut z_final): (f64, f64);
                let mut z: f64;
                let mut zn: f64;
                let (mut x, mut y): (isize, isize);
                let mut weight: f64;
                let mut weights_i = vec![0.0; num_pixels_in_filter];

                for row in starting_row..ending_row {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_data[(row, col)];
                        if z != nodata {
                            //fill weights_i with the appropriate intensity weights
                            sum = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_data[(y, x)];
                                if zn != nodata {
                                    weight = recip_root_2_pi_times_sigma_i * (-1.0 * ((zn - z) * (zn - z)) / two_sigma_sqr_i).exp();
                                    weight *= weights_d[a];
                                    weights_i[a] = weight;
                                    sum += weight;
                                }
                            }

                            z_final = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_data[(y, x)];
                                if zn != nodata {
                                    z_final += weights_i[a] * zn / sum;
                                }
                            }

                            data[col as usize] = z_final;
                        }
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
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma distance: {}", sigma_dist));
        output.add_metadata_entry(format!("Sigma intensity: {}", sigma_int));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

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
            println!("{}",
                    &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}