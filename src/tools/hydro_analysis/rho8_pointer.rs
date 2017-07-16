extern crate time;
extern crate num_cpus;
extern crate rand;

use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;
use self::rand::distributions::{IndependentSample, Range};

pub struct Rho8Pointer {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl Rho8Pointer {
    pub fn new() -> Rho8Pointer { // public constructor
        let name = "Rho8Pointer".to_string();
        
        let description = "Calculates a stochastic Rho8 flow pointer raster from an input DEM.".to_string();
        
        let mut parameters = "--dem         Input raster DEM file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        parameters.push_str("--esri_style  Uses the ESRI style D8 pointer output (default is false).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        Rho8Pointer { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Rho8Pointer {
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
        let mut esri_style = false;
        
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
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
            } else if vec[0].to_lowercase() == "-esri_style" || vec[0].to_lowercase() == "--esri_style" {
                esri_style = true;
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let out_vals = match esri_style {
                    true => [ 128f64, 1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64 ],
                    false => [ 1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64, 128f64 ],
                };
                let (mut z, mut z_n, mut slope): (f64, f64, f64);
                let between = Range::new(0f64, 1f64);
                let mut rng = rand::thread_rng();
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            let mut dir = 0;
							let mut max_slope = f64::MIN;
							for i in 0..8 {
                                z_n = input[(row + d_y[i], col + d_x[i])];
                                if z_n != nodata {
                                    slope = match i {
                                        1 | 3 | 5 | 7 => (z - z_n),
                                        _ => (z - z_n) / (2f64 - between.ind_sample(&mut rng)),
                                    };
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i;
                                    }
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = out_vals[dir];
                            } else {
                                data[col as usize] = 0f64;
                            }
                        } else {
                            data[col as usize] = nodata;
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
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        if esri_style {
            output.add_metadata_entry("ESRI-style output: true".to_string());
        } else {
            output.add_metadata_entry("ESRI-style output: false".to_string());
        }
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