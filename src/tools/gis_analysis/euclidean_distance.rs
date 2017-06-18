extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;
use structures::Array2D;

pub struct EuclideanDistance {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl EuclideanDistance {
    pub fn new() -> EuclideanDistance { // public constructor
        let name = "EuclideanDistance".to_string();
        
        let description = "Calculates the Shih and Wu (2004) Euclidean distance transform.".to_string();
        
        let mut parameters = "-i, --input   Input raster DEM file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        EuclideanDistance { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for EuclideanDistance {
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

        if verbose { println!("Reading data...") };

        let input = Raster::new(&input_file, "r")?;

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let mut r_x: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut r_y: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        let start = time::now();
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        //output.reinitialize_values(0.0);

        let mut h: f64;
        let mut which_cell: usize;
        let inf_val = f64::INFINITY;
        let d_x = [ -1, -1, 0, 1, 1, 1, 0, -1 ];
        let d_y = [ 0, -1, -1, -1, 0, 1, 1, 1 ];
        let g_x = [ 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0 ];
        let g_y = [ 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0 ];
        let (mut x, mut y): (isize, isize);
        let (mut z, mut z2, mut z_min): (f64, f64, f64);
        
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != 0.0 {
                    output[(row, col)] = 0.0;
                } else {
                    output[(row, col)] = inf_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing Rasters: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = output[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 0..4 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = output[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                0 => 2.0 * r_x[(y, x)] + 1.0,
                                1 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                2 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 3
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        for row in (0..rows).rev() {
            for col in (0..columns).rev() {
                z = output[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 4..8 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = output[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                5 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                4 => 2.0 * r_x[(y, x)] + 1.0,
                                6 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 7
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    output[(row, col)] = output[(row, col)].sqrt() * cell_size;
                } else {
                    output[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "spectrum.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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