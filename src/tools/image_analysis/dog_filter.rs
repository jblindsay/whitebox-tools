/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: June 26, 2017
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

pub struct DiffOfGaussianFilter {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl DiffOfGaussianFilter {
    pub fn new() -> DiffOfGaussianFilter { // public constructor
        let name = "DiffOfGaussianFilter".to_string();
        
        let description = "Performs a Difference of Gaussian (DoG) filter on an image.".to_string();
        
        let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--sigma1      Standard deviation distance in pixels.\n");
        parameters.push_str("--sigma2      Standard deviation distance in pixels.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=image.dep -o=output.dep --sigma1=2.0 --sigma2=4.0", short_exe, name).replace("*", &sep);
    
        DiffOfGaussianFilter { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for DiffOfGaussianFilter {
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
        let mut filter_size1 = 0usize;
        let mut filter_size2 = 0usize;
        let mut sigma1 = 2.0;
        let mut sigma2 = 4.0;
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
            } else if vec[0].to_lowercase() == "-sigma1" || vec[0].to_lowercase() == "--sigma1" {
                if keyval {
                    sigma1 = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma1 = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-sigma2" || vec[0].to_lowercase() == "--sigma2" {
                if keyval {
                    sigma2 = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma2 = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if sigma1 < 0.5 {
            sigma1 = 0.5;
        } else if sigma1 > 20.0 {
            sigma1 = 20.0;
        }

        if sigma2 < 0.5 {
            sigma2 = 0.5;
        } else if sigma2 > 20.0 {
            sigma2 = 20.0;
        }

        if sigma1 == sigma2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The two input sigma values should not be equal."));
        }

        let recip_root_2_pi_times_sigma1 = 1.0 / ((2.0 * PI).sqrt() * sigma1);
        let two_sigma_sqr1 = 2.0 * sigma1 * sigma1;

        let recip_root_2_pi_times_sigma2 = 1.0 / ((2.0 * PI).sqrt() * sigma2);
        let two_sigma_sqr2 = 2.0 * sigma2 * sigma2;

        // figure out the size of the filter
        let mut weight: f64;
        for i in 0..250 {
            weight = recip_root_2_pi_times_sigma1 * (-1.0 * ((i * i) as f64) / two_sigma_sqr1).exp();
            if weight <= 0.001 {
                filter_size1 = i * 2 + 1;
                break;
            }
        }
        
        // the filter dimensions must be odd numbers such that there is a middle pixel
        if filter_size1 % 2 == 0 {
            filter_size1 += 1;
        }

        if filter_size1 < 3 { filter_size1 = 3; }

        let num_pixels_in_filter1 = filter_size1 * filter_size1;
        let mut d_x1 = vec![0isize; num_pixels_in_filter1];
        let mut d_y1 = vec![0isize; num_pixels_in_filter1];
        let mut weights1 = vec![0.0; num_pixels_in_filter1];
        
        // fill the filter d_x and d_y values and the distance-weights
        let midpoint1: isize = (filter_size1 as f64 / 2f64).floor() as isize + 1;
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size1 {
            for col in 0..filter_size1 {
                x = col as isize - midpoint1;
                y = row as isize - midpoint1;
                d_x1[a] = x;
                d_y1[a] = y;
                weight = recip_root_2_pi_times_sigma1 * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr1).exp();
                weights1[a] = weight;
                a += 1;
            }
        }


        // figure out the size of the filter
        for i in 0..250 {
            weight = recip_root_2_pi_times_sigma2 * (-1.0 * ((i * i) as f64) / two_sigma_sqr2).exp();
            if weight <= 0.001 {
                filter_size2 = i * 2 + 1;
                break;
            }
        }
        
        // the filter dimensions must be odd numbers such that there is a middle pixel
        if filter_size2 % 2 == 0 {
            filter_size2 += 1;
        }

        if filter_size2 < 3 { filter_size2 = 3; }

        let num_pixels_in_filter2 = filter_size2 * filter_size2;
        let mut d_x2 = vec![0isize; num_pixels_in_filter2];
        let mut d_y2 = vec![0isize; num_pixels_in_filter2];
        let mut weights2 = vec![0.0; num_pixels_in_filter2];
        
        // fill the filter d_x and d_y values and the distance-weights
        let midpoint2: isize = (filter_size2 as f64 / 2f64).floor() as isize + 1;
        a = 0;
        for row in 0..filter_size2 {
            for col in 0..filter_size2 {
                x = col as isize - midpoint2;
                y = row as isize - midpoint2;
                d_x2[a] = x;
                d_y2[a] = y;
                weight = recip_root_2_pi_times_sigma2 * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr2).exp();
                weights2[a] = weight;
                a += 1;
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let d_x1 = Arc::new(d_x1);
        let d_y1 = Arc::new(d_y1);
        let weights1 = Arc::new(weights1);
        let d_x2 = Arc::new(d_x2);
        let d_y2 = Arc::new(d_y2);
        let weights2 = Arc::new(weights2);

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
            let d_x1 = d_x1.clone();
            let d_y1 = d_y1.clone();
            let weights1 = weights1.clone();
            let d_x2 = d_x2.clone();
            let d_y2 = d_y2.clone();
            let weights2 = weights2.clone();
            starting_row = id * row_block_size;
            ending_row = starting_row + row_block_size;
            if ending_row > rows {
                ending_row = rows;
            }
            id += 1;
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut sum, mut z_final1, mut z_final2): (f64, f64, f64);
                let mut z: f64;
                let mut zn: f64;
                let (mut x, mut y): (isize, isize);
                for row in starting_row..ending_row {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_data[(row, col)];
                        if z != nodata {
                            sum = 0.0;
                            z_final1 = 0.0;
                            for a in 0..num_pixels_in_filter1 {
                                x = col + d_x1[a];
                                y = row + d_y1[a];
                                zn = input_data[(y, x)];
                                if zn != nodata {
                                    sum += weights1[a];
                                    z_final1 += weights1[a] * zn;
                                }
                            }
                            z_final1 = z_final1 / sum;

                            sum = 0.0;
                            z_final2 = 0.0;
                            for a in 0..num_pixels_in_filter2 {
                                x = col + d_x2[a];
                                y = row + d_y2[a];
                                zn = input_data[(y, x)];
                                if zn != nodata {
                                    sum += weights2[a];
                                    z_final2 += weights2[a] * zn;
                                }
                            }
                            z_final2 = z_final2 / sum;

                            data[col as usize] = z_final1 - z_final2;
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
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma1: {}", sigma1));
        output.add_metadata_entry(format!("Sigma2: {}", sigma2));
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