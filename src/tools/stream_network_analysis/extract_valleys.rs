/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 12, 2017
Last Modified: July 12, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::cmp::Ordering::Equal;
use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::WhiteboxTool;

pub struct ExtractValleys {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl ExtractValleys {
    pub fn new() -> ExtractValleys {
        // public constructor
        let name = "ExtractValleys".to_string();

        let description = "Identifies potential valley bottom grid cells based on local topolography alone."
            .to_string();

        let mut parameters = "--dem           Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--variant       Options include 'lq' (lower quartile), 'JandR' (Johnston and Rosenfeld), and 'PandD' (Peucker and Douglas); default is 'lq'.\n");
        parameters.push_str("--line_thin     Optional flag indicating whether post-processing line-thinning should be performed.\n");
        parameters.push_str("--filter        Optional argument (only used when variant='lq') providing the filter size, in grid cells, used for lq-filtering (default is 5).\n");

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
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --dem=pointer.dep -o=out.dep --variant='JandR' --line_thin
>>.*{0} -r={1} --wd=\"*path*to*data*\" --dem=pointer.dep -o=out.dep --variant='lq' --filter=7 --line_thin", short_exe, name).replace("*", &sep);

        ExtractValleys {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ExtractValleys {
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
        let mut variant = String::from("lq");
        let mut line_thin = false;
        let mut filter_size = 5;
        
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
            if vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-variant" || vec[0].to_lowercase() == "--variant" {
                if keyval {
                    variant = vec[1].to_string();
                } else {
                    variant = args[i+1].to_string();
                }
                if variant.to_lowercase().contains("q") {
                    variant = String::from("lq");
                } else if variant.to_lowercase().contains("j") {
                    variant = String::from("JandR");
                } else { //if variant.to_lowercase().contains("p") {
                    variant = String::from("PandD");
                }
            } else if vec[0].to_lowercase() == "-line_thin" || vec[0].to_lowercase() == "--line_thin" {
                line_thin = true;
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size = args[i + 1].to_string().parse::<usize>().unwrap();
                }

                //the filter dimensions must be odd numbers such that there is a middle pixel
                if (filter_size as f64 / 2f64).floor() == filter_size as f64 / 2f64 {
                    println!("WARNING: Filter dimensions must be odd numbers. The specified filter dimension has been modified.");
                    filter_size += 1;
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        
        match &variant as &str {
            "lq" => {
                output.reinitialize_values(0f64);
                
                // This one can be performed conccurently.
                let num_procs = num_cpus::get() as isize;
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let num_cells_in_filter = filter_size * filter_size;
                        let mut dx = vec![0isize; num_cells_in_filter];
                        let mut dy = vec![0isize; num_cells_in_filter];
                        let midpoint = (filter_size as f64 / 2f64).floor() as isize;
                        let mut z: f64;
                        let mut zn: f64;
                        let large_value = f64::INFINITY;
                        let mut n: f64;
                        let mut lower_quartile: usize;
                        
                        // let mut filter_shape = vec![1f64; num_cells_in_filter];
                        // //see which pixels in the filter lie within the largest ellipse 
                        // //that fits in the filter box 
                        // let mut asqr = midpoint * midpoint;
                        let mut i = 0;
                        for row in 0..filter_size as isize {
                            for col in 0..filter_size as isize {
                                dx[i] = col - midpoint;
                                dy[i] = row - midpoint;
                                // z = (dx[i] * dx[i]) / asqr + (dy[i] * dy[i]) / asqr;
                                // if z > 1f64 {
                                //     filter_shape[i] = 0f64;
                                // }
                                i += 1;
                            }
                        }
                        
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                z = input[(row, col)];
                                if z != nodata  {
                                    let mut cell_data = vec![1f64; num_cells_in_filter];
                                    n = 0f64;
                                    for i in 0..num_cells_in_filter {
                                        zn = input[(row + dy[i], col + dx[i])];
                                        if zn != nodata {
                                            cell_data[i] = zn;
                                            n += 1f64;
                                        } else {
                                            cell_data[i] = large_value;
                                        }
                                    }
                                    if n > 0f64 {
                                        // sort the array
                                        cell_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
                                        lower_quartile = (n / 4f64).floor() as usize;
                                        if z <= cell_data[lower_quartile] {
                                            data[col as usize] = 1f64;
                                        }
                                    }
                                } else {
                                    data[col as usize] = nodata;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for row in 0..rows {
                    let data = rx.recv().unwrap();
                    output.set_row_data(data.0, data.1);
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

            },
            "JandR" => {
                // This one can be performed conccurently.
                // output.reinitialize_values(0f64);
                let num_procs = num_cpus::get() as isize;
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let (mut z, mut zn1, mut zn2): (f64, f64, f64);
                        let dx = [ 0, 0, -1, 1 ];
                        let dy = [ -1, 1, 0, 0 ];
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                z = input[(row, col)];
                                if z != nodata  {
                                    zn1 = input[(row + dy[0], col + dx[0])];
                                    zn2 = input[(row + dy[1], col + dx[1])];
                                    if zn1 != nodata && zn2 != nodata && zn1 > z && zn2 > z {
                                        data[col as usize] = 1f64;
                                    } else {
                                        zn1 = input[(row + dy[2], col + dx[2])];
                                        zn2 = input[(row + dy[3], col + dx[3])];
                                        if zn1 != nodata && zn2 != nodata && zn1 > z && zn2 > z {
                                            data[col as usize] = 1f64;
                                        } else {
                                            data[col as usize] = 0f64;
                                        }
                                    }
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for row in 0..rows {
                    let data = rx.recv().unwrap();
                    output.set_row_data(data.0, data.1);
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            },
            _ => { // "PandD"
                // This one can't easily be performed conccurently because a cell can be 
                // modified while a row other than the row containing the cell is being scanned.
                output.reinitialize_values(1f64);
                let mut z: f64;
                let mut maxz: f64;
                let mut which_cell: usize;
                let dx = [ -1, 0, -1, 0 ];
                let dy = [ -1, -1, 0, 0 ];
                let num_scan_cells = dx.len();
                for row in 0..rows {
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata  {
                            maxz = z;
                            which_cell = 3;
                            for n in 0..num_scan_cells {
                                z = input[(row + dy[n], col + dx[n])];
                                if z != nodata {
                                    if z > maxz {
                                        maxz = z;
                                        which_cell = n;
                                    }
                                }
                            }
                            output.set_value(row + dy[which_cell], col + dx[which_cell], 0f64);
                        } else {
                            output[(row, col)] = nodata;
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }
        

        if line_thin {
            println!("Line thinning operation...");
            let mut did_something = true;
            let mut loop_num = 0;
            let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
            let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
            let elements = vec![ vec![ 6, 7, 0, 4, 3, 2 ], vec![ 7, 0, 1, 3, 5 ], 
                vec![ 0, 1, 2, 4, 5, 6 ], vec![ 1, 2, 3, 5, 7 ], 
                vec![ 2, 3, 4, 6, 7, 0 ], vec![ 3, 4, 5, 7, 1 ], 
                vec![ 4, 5, 6, 0, 1, 2 ], vec![ 5, 6, 7, 1, 3 ] ];

            let vals = vec![ vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ], 
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ],
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ],
                vec![ 0f64, 0f64, 0f64, 1f64, 1f64, 1f64 ], vec![ 0f64, 0f64, 0f64, 1f64, 1f64 ] ];
            
            let mut neighbours = [0.0; 8];
            let mut pattern_match: bool;
            let mut z: f64;
            while did_something {
                loop_num += 1;
                did_something = false;
                for a in 0..8 {
                    for row in 0..rows {
                        for col in 0..columns {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }
                                
                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != vals[a][i] {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as i32;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("Variant: {}", variant));
        if variant == String::from("lq") {
            output.add_metadata_entry(format!("Filter size: {}", filter_size));
        }
        output.add_metadata_entry(format!("Line thinning: {}", line_thin));
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
