/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 14, 2017
Last Modified: July 14, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;
extern crate rand;

use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::WhiteboxTool;
use self::rand::distributions::{Normal, IndependentSample, Range};

pub struct TurningBandsSimulation {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl TurningBandsSimulation {
    pub fn new() -> TurningBandsSimulation {
        // public constructor
        let name = "TurningBandsSimulation".to_string();

        let description = "Creates an image containing random values based on a turning-bands simulation."
            .to_string();

        let mut parameters = "--base          Input base raster file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--range         The field's range, in xy-units, related to the extent of spatial autocorrelation.\n");
        parameters.push_str("--iterations    The number of iterations; default is 1000.\n");
        
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
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --base=in.dep -o=out.dep --range=850.0 --iterations=2500", short_exe, name).replace("*", &sep);

        TurningBandsSimulation {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TurningBandsSimulation {
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
        let mut range = 1f64;
        let mut iterations = 1000;
        
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
            } else if vec[0].to_lowercase() == "-range" || vec[0].to_lowercase() == "--range" {
                if keyval {
                    range = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    range = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-iterations" || vec[0].to_lowercase() == "--iterations" {
                if keyval {
                    iterations = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    iterations = args[i+1].to_string().parse::<usize>().unwrap();
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

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        // let nodata = input.configs.nodata;

        let diagonal_size = (rows as f64 * rows as f64 + columns as f64 * columns as f64).sqrt() as usize;
        let filter_half_size = (range / (2f64 * input.configs.resolution_x as f64)) as usize;
        let filter_size = filter_half_size * 2 + 1;
        let mut cell_offsets = vec![0isize; filter_size];
        for i in 0..filter_size as isize {
            cell_offsets[i as usize] = i - filter_half_size as isize;
        }

        let w = (36f64 / (filter_half_size * (filter_half_size + 1) * filter_size) as f64).sqrt();
            

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(0.0);

        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0);
        let between = Range::new(0, 4);
        let between_rows = Range::new(0f64, rows as f64);
        let between_cols = Range::new(0f64, columns as f64);
        let mut z: f64;
        let (mut pnt1x, mut pnt1y, mut pnt2x, mut pnt2y): (f64, f64, f64, f64);
        
        // loop through the number of iterations
        for i in 0..iterations {

            // create the data line and fill it with random numbers.
            // notice that the initial dataline is 2 * filterHalfSize larger 
            // because of the edge effects of the filter.
            let mut t = vec![0f64; diagonal_size + 2 * filter_half_size];
            for j in 0..diagonal_size {
                t[j] = normal.ind_sample(&mut rng);
            }

            let mut y = vec![0f64; diagonal_size];

            // filter the line
            let mut m: isize;
            for j in 0..diagonal_size {
                z = 0f64;
                for k in 0..filter_size {
                    m = cell_offsets[k];
                    z += m as f64 * t[(j as isize + filter_half_size as isize + m) as usize];
                }
                y[j] = w * z;
            }

            // assign the spatially autocorrelated data line an equation of a transect of the grid
            // first, pick two points on different edges of the grid at random.
            // Edges are as follows 0 = left, 1 = top, 2 = right, and 3 = bottom
            let edge1 = between.ind_sample(&mut rng);
            let mut edge2 = edge1;
            while edge2 == edge1 {
                edge2 = between.ind_sample(&mut rng);
            }

            match edge1 {
                0 => {
                    pnt1x = 0f64;
                    pnt1y = between_rows.ind_sample(&mut rng);
                },
                1 => {
                    pnt1x = between_cols.ind_sample(&mut rng);
                    pnt1y = 0f64;
                },
                2 => {
                    pnt1x = (columns - 1) as f64;
                    pnt1y = between_rows.ind_sample(&mut rng);
                },
                _ => { // 3
                    pnt1x = between_cols.ind_sample(&mut rng);
                    pnt1y = (rows - 1) as f64;
                },
            }

            match edge2 {
                0 => {
                    pnt2x = 0f64;
                    pnt2y = between_rows.ind_sample(&mut rng);
                },
                1 => {
                    pnt2x = between_cols.ind_sample(&mut rng);
                    pnt2y = 0f64;
                },
                2 => {
                    pnt2x = (columns - 1) as f64;
                    pnt2y = between_rows.ind_sample(&mut rng);
                },
                _ => { // 3
                    pnt2x = between_cols.ind_sample(&mut rng);
                    pnt2y = (rows - 1) as f64;
                },
            }
            
            if pnt1x == pnt2x || pnt1y == pnt2y {
                while pnt1x == pnt2x || pnt1y == pnt2y {
                    match edge2 {
                        0 => {
                            pnt2x = 0f64;
                            pnt2y = between_rows.ind_sample(&mut rng);
                        },
                        1 => {
                            pnt2x = between_cols.ind_sample(&mut rng);
                            pnt2y = 0f64;
                        },
                        2 => {
                            pnt2x = (columns - 1) as f64;
                            pnt2y = between_rows.ind_sample(&mut rng);
                        },
                        _ => { // 3
                            pnt2x = between_cols.ind_sample(&mut rng);
                            pnt2y = (rows - 1) as f64;
                        },
                    }
                }
            }

            let line_slope = (pnt2y - pnt1y) / (pnt2x - pnt1x);
            let line_intercept = pnt1y - line_slope * pnt1x;
            let perpendicular_line_slope = -1f64 / line_slope;
            let slope_diff = line_slope - perpendicular_line_slope;
            let mut perpendicular_line_intercept: f64;
            let (mut row, mut col): (usize, usize);

            // for each of the four corners, figure out what the perpendicular line 
            // intersection coordinates would be.

            // point (0,0)
            perpendicular_line_intercept = 0f64;
            let corner1x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner1y = line_slope * corner1x - line_intercept;

            // point (0,cols)
            row = 0;
            col = columns as usize;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner2x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner2y = line_slope * corner2x - line_intercept;

            // point (rows,0)
            row = rows as usize;
            col = 0;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner3x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner3y = line_slope * corner3x - line_intercept;

            // point (rows,cols)
            row = rows as usize;
            col = columns as usize;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner4x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner4y = line_slope * corner4x - line_intercept;

            // find the point with the minimum Y value and set it as the line starting point
            let mut line_start_x = corner1x;
            let mut line_start_y = corner1y;
            if corner2y < line_start_y {
                line_start_x = corner2x;
                line_start_y = corner2y;
            }
            if corner3y < line_start_y {
                line_start_x = corner3x;
                line_start_y = corner3y;
            }
            if corner4y < line_start_y {
                line_start_x = corner4x;
                line_start_y = corner4y;
            }

            // scan through each grid cell and assign it the closest value on the line segment
            let num_procs = num_cpus::get() as isize;
            let (tx, rx) = mpsc::channel();
            let y = Arc::new(y);
            for tid in 0..num_procs {
                let y = y.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut perpendicular_line_intercept: f64;
                        let (mut intersecting_point_x, mut intersecting_point_y): (f64, f64);
                        let mut data = vec![0f64; columns as usize];
                        for col in 0..columns {
                            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
                            intersecting_point_x = (perpendicular_line_intercept - line_intercept) / slope_diff;
                            intersecting_point_y = line_slope * intersecting_point_x - line_intercept;
                            let mut p = (((intersecting_point_x - line_start_x) * (intersecting_point_x - line_start_x)
                                    + (intersecting_point_y - line_start_y) * (intersecting_point_y - line_start_y)).sqrt()) as isize;
                            if p < 0 {
                                p = 0;
                            }
                            if p > (diagonal_size - 1) as isize {
                                p = (diagonal_size - 1) as isize;
                            }
                            data[col as usize] = y[p as usize];
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for _ in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.increment_row_data(row, data);
            }

            if verbose {
                progress = (100.0_f64 * i as f64 / (iterations - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }

        }

        println!("Calculating the mean and standard deviation...");
        let (mean, stdev) = output.calculate_mean_and_stdev();

        for row in 0..rows {
            for col in 0..columns {
                output[(row, col)] = (output[(row, col)] - mean) / stdev; // / iterations as f64;
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input base raster file: {}", input_file));
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
