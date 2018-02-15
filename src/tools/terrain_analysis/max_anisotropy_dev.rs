/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dan Newman and John Lindsay
Created: January 26, 2018
Last Modified: Jan. 26, 2018
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use structures::Array2D;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct MaxAnisotropyDev {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MaxAnisotropyDev {
    pub fn new() -> MaxAnisotropyDev { // public constructor
        let name = "MaxAnisotropyDev".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates the maximum anisotropy (directionality) in elevation deviation over a range of spatial scales.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output DEVmax Magnitude File".to_owned(), 
            flags: vec!["--out_mag".to_owned()], 
            description: "Output raster DEVmax magnitude file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output DEVmax Scale File".to_owned(), 
            flags: vec!["--out_scale".to_owned()], 
            description: "Output raster DEVmax scale file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Minimum Search Neighbourhood Radius (grid cells)".to_owned(), 
            flags: vec!["--min_scale".to_owned()], 
            description: "Minimum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some(String::from("3")),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Maximum Search Neighbourhood Radius (grid cells)".to_owned(), 
            flags: vec!["--max_scale".to_owned()], 
            description: "Maximum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Step Size".to_owned(), 
            flags: vec!["--step".to_owned()], 
            description: "Step size as any positive non-zero integer.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some(String::from("2")),
            optional: false
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.dep -out_mag=DEVmax_mag.dep --out_scale=DEVmax_scale.dep --min_scale=1 --max_scale=1000 --step=5", short_exe, name).replace("*", &sep);
    
        MaxAnisotropyDev { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for MaxAnisotropyDev {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }
    
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_mag_file = String::new();
        let mut output_scale_file = String::new();
        let mut min_scale = 3isize;
        let mut max_scale = 100isize;
        let mut step = 2isize;
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_mag" || vec[0].to_lowercase() == "--out_mag" {
                if keyval {
                    output_mag_file = vec[1].to_string();
                } else {
                    output_mag_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_scale" || vec[0].to_lowercase() == "--out_scale" {
                if keyval {
                    output_scale_file = vec[1].to_string();
                } else {
                    output_scale_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-min_scale" || vec[0].to_lowercase() == "--min_scale" {
                if keyval {
                    min_scale = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    min_scale = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if min_scale < 3 { min_scale = 3; }
            } else if vec[0].to_lowercase() == "-max_scale" || vec[0].to_lowercase() == "--max_scale" {
                if keyval {
                    max_scale = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    max_scale = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if max_scale < 5 { max_scale = 5; }
            } else if vec[0].to_lowercase() == "-step" || vec[0].to_lowercase() == "--step" {
                if keyval {
                    step = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    step = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if step < 1 { step = 1; }
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_mag_file.contains(&sep) && !output_mag_file.contains("/") {
            output_mag_file = format!("{}{}", working_directory, output_mag_file);
        }
        if !output_scale_file.contains(&sep) && !output_scale_file.contains("/") {
            output_scale_file = format!("{}{}", working_directory, output_scale_file);
        }

        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
    
        // create the integral images
        let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, 0f64)?;
        let mut integral2: Array2D<f64> = Array2D::new(rows, columns, 0f64, 0f64)?;
        let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, 0)?;

        let mut val: f64;
        let mut sum: f64;
        let mut sum_sqr: f64;
        let mut sum_n: i32;
        let (mut i_prev, mut i2_prev): (f64, f64);
        let mut n_prev: i32;
        for row in 0..rows {
            sum = 0f64;
            sum_sqr = 0f64;
            sum_n = 0;
            for col in 0..columns {
                val = input[(row, col)];
                if val == nodata {
                    val = 0f64;
                } else {
                    sum_n += 1;
                }
                sum += val;
                sum_sqr += val * val;
                if row > 0 {
                    i_prev = integral[(row - 1, col)];
                    i2_prev = integral2[(row - 1, col)];
                    n_prev = integral_n[(row - 1, col)];
                    integral[(row, col)] = sum + i_prev;
                    integral2[(row, col)] = sum_sqr + i2_prev;
                    integral_n[(row, col)] = sum_n + n_prev;
                } else {
                    integral[(row, col)] = sum;
                    integral2[(row, col)] = sum_sqr;
                    integral_n[(row, col)] = sum_n;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating integral images: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let i = Arc::new(integral); // wrap integral in an Arc
        let i2 = Arc::new(integral2); // wrap integral2 in an Arc
        let i_n = Arc::new(integral_n); // wrap integral_n in an Arc
        
        let num_procs = num_cpus::get() as isize;

        let mut output_mag = Raster::initialize_using_file(&output_mag_file, &input);
        let mut output_scale = Raster::initialize_using_file(&output_scale_file, &input);
        
        // num_loops can be determined exactly ahead of time.
        let num_loops = (min_scale..max_scale+1).fold(0, |acc, x| if (x - min_scale) % step == 0 { acc + 1 } else { acc });
        let mut middle_pane_radius: isize;
        let mut loop_num = 0;
        for midpoint in (min_scale..max_scale+1).filter(|x| (x - min_scale) % step == 0) {
            loop_num += 1;
            middle_pane_radius = (midpoint * 2 + 1) / 6;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let i = i.clone();
                let i2 = i2.clone();
                let i_n = i_n.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let (mut x3, mut x4, mut y3, mut y4): (isize, isize, isize, isize);
                    let mut n: i32;
                    let (mut mean, mut sum, mut sum_sqr): (f64, f64, f64);
                    let (mut v, mut s): (f64, f64);
                    let mut z: f64;
                    let mut values = [0f64; 5];
                    let mut num_panes_valid: f64;
                    // let mut min_dev: f64;
                    // let mut max_dev: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        // top to bottom:
                        // -midpoint  -middle_pane_radius  +middle_pane_radius  +midpoint  
                        //     y1             y2                  y3               y4
                        y1 = row - midpoint - 1;
                        y4 = row + midpoint;
                        y2 = row - middle_pane_radius - 1;
                        y3 = row + middle_pane_radius;
                        
                        let mut data = vec![nodata; columns as usize];

                        if y1 >= 0 && y4 < rows  { // restricts edge effects

                            for col in 0..columns {
                                z = input_data[(row, col)];
                                if z != nodata {
                                    // left to right:
                                    // -midpoint  -middle_pane_radius  +middle_pane_radius  +midpoint  
                                    //     x1             x2                  x3               x4
                                    x1 = col - midpoint - 1;
                                    x4 = col + midpoint;
                                    x2 = col - middle_pane_radius - 1;
                                    x3 = col + middle_pane_radius;
                                    
                                    if x1 >= 0 && x4 < columns { // restricts edge effects

                                        // min_dev = f64::INFINITY;
                                        // max_dev = f64::NEG_INFINITY;
                                    
                                        // Order is always lower-right + upper-left - upper-right - lower-left
                                        n = i_n[(y4, x4)] + i_n[(y1, x1)] - i_n[(y1, x4)] - i_n[(y4, x1)];
                                        if n > 3 {
                                            sum = i[(y4, x4)] + i[(y1, x1)] - i[(y1, x4)] - i[(y4, x1)];
                                            sum_sqr = i2[(y4, x4)] + i2[(y1, x1)] - i2[(y1, x4)] - i2[(y4, x1)];
                                            v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                            if v > 0f64 {
                                                s = v.sqrt();
                                                mean = sum / n as f64;
                                                values[0] = (z - mean) / s; // overall DEV

                                                num_panes_valid = 4f64;

                                                // North-south panel
                                                // - X -
                                                // - X -
                                                // - X -
                                                n = i_n[(y4, x3)] + i_n[(y1, x2)] - i_n[(y1, x3)] - i_n[(y4, x2)];
                                                if n > 3 { 
                                                    sum = i[(y4, x3)] + i[(y1, x2)] - i[(y1, x3)] - i[(y4, x2)];
                                                    sum_sqr = i2[(y4, x3)] + i2[(y1, x2)] - i2[(y1, x3)] - i2[(y4, x2)];
                                                    v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                                    if v > 0f64 {
                                                        s = v.sqrt();
                                                        mean = sum / n as f64;
                                                        values[1] = (z - mean) / s; // - values[0]; // N-S DEV
                                                        // if values[1] < min_dev { min_dev = values[1]; }
                                                        // if values[1] > max_dev { max_dev = values[1]; }
                                                        values[1] -= values[0];
                                                    } else {
                                                        values[1] = 0f64;
                                                        num_panes_valid -= 1f64;
                                                    }
                                                } else {
                                                    values[1] = 0f64;
                                                    num_panes_valid -= 1f64;
                                                }

                                                // East-west panel
                                                // - - - 
                                                // X X X
                                                // - - -
                                                n = i_n[(y3, x4)] + i_n[(y2, x1)] - i_n[(y2, x4)] - i_n[(y3, x1)];
                                                if n > 3 {
                                                    sum = i[(y3, x4)] + i[(y2, x1)] - i[(y2, x4)] - i[(y3, x1)];
                                                    sum_sqr = i2[(y3, x4)] + i2[(y2, x1)] - i2[(y2, x4)] - i2[(y3, x1)];
                                                    v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                                    if v > 0f64 {
                                                        s = v.sqrt();
                                                        mean = sum / n as f64;
                                                        values[2] = (z - mean) / s; // - values[0]; // E-W DEV
                                                        // if values[2] < min_dev { min_dev = values[2]; }
                                                        // if values[2] > max_dev { max_dev = values[2]; }
                                                        values[2] -= values[0];
                                                    } else {
                                                        values[2] = 0f64;
                                                        num_panes_valid -= 1f64;
                                                    }
                                                } else {
                                                    values[2] = 0f64;
                                                    num_panes_valid -= 1f64;
                                                }

                                                // Northeast-southwest panel
                                                // - - X
                                                // - X -
                                                // X - - 
                                                n = (i_n[(y2, x4)] + i_n[(y1, x3)] - i_n[(y1, x4)] - i_n[(y2, x3)]) +
                                                    (i_n[(y3, x3)] + i_n[(y2, x2)] - i_n[(y2, x3)] - i_n[(y3, x2)]) +
                                                    (i_n[(y4, x2)] + i_n[(y3, x1)] - i_n[(y3, x2)] - i_n[(y4, x1)]);
                                                if n > 3 {
                                                    sum = (i[(y2, x4)] + i[(y1, x3)] - i[(y1, x4)] - i[(y2, x3)]) +
                                                        (i[(y3, x3)] + i[(y2, x2)] - i[(y2, x3)] - i[(y3, x2)]) +
                                                        (i[(y4, x2)] + i[(y3, x1)] - i[(y3, x2)] - i[(y4, x1)]);
                                                    sum_sqr = (i2[(y2, x4)] + i2[(y1, x3)] - i2[(y1, x4)] - i2[(y2, x3)]) +
                                                            (i2[(y3, x3)] + i2[(y2, x2)] - i2[(y2, x3)] - i2[(y3, x2)]) +
                                                            (i2[(y4, x2)] + i2[(y3, x1)] - i2[(y3, x2)] - i2[(y4, x1)]);
                                                    v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                                    if v > 0f64 {
                                                        s = v.sqrt();
                                                        mean = sum / n as f64;
                                                        values[3] = (z - mean) / s; // - values[0]; // NE-SW DEV
                                                        // if values[3] < min_dev { min_dev = values[3]; }
                                                        // if values[3] > max_dev { max_dev = values[3]; }
                                                        values[3] -= values[0];
                                                    } else {
                                                        values[3] = 0f64;
                                                        num_panes_valid -= 1f64;
                                                    }
                                                } else {
                                                    values[3] = 0f64;
                                                    num_panes_valid -= 1f64;
                                                }

                                                // Northwest-southeast panel
                                                // X - -
                                                // - X -
                                                // - - X 
                                                n = (i_n[(y2, x2)] + i_n[(y1, x1)] - i_n[(y1, x2)] - i_n[(y2, x1)]) +
                                                    (i_n[(y3, x3)] + i_n[(y2, x2)] - i_n[(y2, x3)] - i_n[(y3, x2)]) +
                                                    (i_n[(y4, x4)] + i_n[(y3, x3)] - i_n[(y3, x4)] - i_n[(y4, x3)]);
                                                if n > 3 {
                                                    sum = (i[(y2, x2)] + i[(y1, x1)] - i[(y1, x2)] - i[(y2, x1)]) +
                                                        (i[(y3, x3)] + i[(y2, x2)] - i[(y2, x3)] - i[(y3, x2)]) +
                                                        (i[(y4, x4)] + i[(y3, x3)] - i[(y3, x4)] - i[(y4, x3)]);
                                                    sum_sqr = (i2[(y2, x2)] + i2[(y1, x1)] - i2[(y1, x2)] - i2[(y2, x1)]) +
                                                            (i2[(y3, x3)] + i2[(y2, x2)] - i2[(y2, x3)] - i2[(y3, x2)]) +
                                                            (i2[(y4, x4)] + i2[(y3, x3)] - i2[(y3, x4)] - i2[(y4, x3)]);
                                                    v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                                                    if v > 0f64 {
                                                        s = v.sqrt();
                                                        mean = sum / n as f64;
                                                        values[4] = (z - mean) / s; // - values[0]; // NW-SE DEV
                                                        // if values[4] < min_dev { min_dev = values[4]; }
                                                        // if values[4] > max_dev { max_dev = values[4]; }
                                                        values[4] -= values[0];
                                                    } else {
                                                        values[4] = 0f64;
                                                        num_panes_valid -= 1f64;
                                                    }
                                                } else {
                                                    values[4] = 0f64;
                                                    num_panes_valid -= 1f64;
                                                }
                                        
                                                if num_panes_valid > 0f64 {
                                                    // data[col as usize] = max_dev - min_dev;
                                                    data[col as usize] = ((values[1]*values[1] + values[2]*values[2] +
                                                                          values[3]*values[3] + values[4]*values[4]) /
                                                                          num_panes_valid).sqrt();
                                                }
                                            }
                                        }

                                    }

                                }
                            }

                        }

                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let (mut z1, mut z2): (f64, f64);
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                for col in 0..columns {
                    z2 = data[col as usize];
                    if z2 != nodata {
                        z1 = output_mag[(row, col)];
                        if z1 != nodata {
                            if z2 * z2 > z1 * z1 {
                                output_mag[(row, col)] = z2;
                                output_scale[(row, col)] = midpoint as f64;
                            }
                        } else {
                            output_mag[(row, col)] = z2;
                            output_scale[(row, col)] = midpoint as f64;
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop {} of {}): {}%", loop_num, num_loops, progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output_mag.configs.palette = "blue_white_red.pal".to_string();
        output_mag.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output_mag.add_metadata_entry(format!("Input file: {}", input_file));
        output_mag.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_mag.add_metadata_entry(format!("Maximum neighbourhood radius: {}", max_scale));
        output_mag.add_metadata_entry(format!("Step size y: {}", step));
        output_mag.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving magnitude data...") };
        let _ = match output_mag.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        output_scale.configs.palette = "spectrum.plt".to_string();
        output_scale.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output_scale.add_metadata_entry(format!("Input file: {}", input_file));
        output_scale.add_metadata_entry(format!("Minimum neighbourhood radius: {}", min_scale));
        output_scale.add_metadata_entry(format!("Maximum neighbourhood radius: {}", max_scale));
        output_scale.add_metadata_entry(format!("Step size: {}", step));
        output_scale.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving scale data...") };
        let _ = match output_scale.write() {
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