/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 25, 2017
Last Modified: July 26, 2017
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
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

/// Tool struct containing the essential descriptors required to interact with the tool.
pub struct RgbToIhs {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RgbToIhs {

    /// Public constructor.
    pub fn new() -> RgbToIhs {
        let name = "RgbToIhs".to_string();
        
        let description = "Converts red, green, and blue (RGB) images into intensity, hue, and saturation (IHS) images.".to_string();
        
        let mut parameters = "--red          Input red band raster file.\n".to_owned();
        parameters.push_str("--green        Input green raster file.\n");
        parameters.push_str("--blue         Input blue raster file.\n");
        parameters.push_str("--composite    Optional input colour-composite image file.\n");
        parameters.push_str("--intensity    Output intensity raster file.\n");
        parameters.push_str("--hue          Output hue raster file.\n");
        parameters.push_str("--saturation   Output saturation file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --red=band3.dep --green=band2.dep --blue=band1.dep --intensity=intensity.dep --hue=hue.dep --saturation=saturation.dep
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --composite=image.dep --intensity=intensity.dep --hue=hue.dep --saturation=saturation.dep", short_exe, name).replace("*", &sep);
    
        RgbToIhs { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RgbToIhs {
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
        let mut red_file = String::new();
        let mut green_file = String::new();
        let mut blue_file = String::new();
        let mut intensity_file = String::new();
        let mut hue_file = String::new();
        let mut saturation_file = String::new();
        let mut composite_file = String::new();
        let mut use_composite = false;
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
            if vec[0].to_lowercase() == "-red" || vec[0].to_lowercase() == "--red" {
                if keyval {
                    red_file = vec[1].to_string();
                } else {
                    red_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-g" || vec[0].to_lowercase() == "-green" || vec[0].to_lowercase() == "--green" {
                if keyval {
                    green_file = vec[1].to_string();
                } else {
                    green_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-b" || vec[0].to_lowercase() == "-blue" || vec[0].to_lowercase() == "--blue" {
                if keyval {
                    blue_file = vec[1].to_string();
                } else {
                    blue_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-composite" || vec[0].to_lowercase() == "--composite" {
                if keyval {
                    composite_file = vec[1].to_string();
                } else {
                    composite_file = args[i+1].to_string();
                }
                use_composite = true;
            } else if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "-intensity" || vec[0].to_lowercase() == "--intensity" {
                if keyval {
                    intensity_file = vec[1].to_string();
                } else {
                    intensity_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "-hue" || vec[0].to_lowercase() == "--hue" {
                if keyval {
                    hue_file = vec[1].to_string();
                } else {
                    hue_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-s" || vec[0].to_lowercase() == "-saturation" || vec[0].to_lowercase() == "--saturation" {
                if keyval {
                    saturation_file = vec[1].to_string();
                } else {
                    saturation_file = args[i+1].to_string();
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

        if !red_file.contains(&sep) {
            red_file = format!("{}{}", working_directory, red_file);
        }
        if !green_file.contains(&sep) {
            green_file = format!("{}{}", working_directory, green_file);
        }
        if !blue_file.contains(&sep) {
            blue_file = format!("{}{}", working_directory, blue_file);
        }
        if !composite_file.contains(&sep) {
            composite_file = format!("{}{}", working_directory, composite_file);
        }
        if !intensity_file.contains(&sep) {
            intensity_file = format!("{}{}", working_directory, intensity_file);
        }
        if !hue_file.contains(&sep) {
            hue_file = format!("{}{}", working_directory, hue_file);
        }
        if !saturation_file.contains(&sep) {
            saturation_file = format!("{}{}", working_directory, saturation_file);
        }

        let num_procs = num_cpus::get() as isize;

        if !use_composite {
            if verbose { println!("Reading red band data...") };
            let input_r = Arc::new(Raster::new(&red_file, "r")?);
            if verbose { println!("Reading green band data...") };
            let input_g = Arc::new(Raster::new(&green_file, "r")?);
            if verbose { println!("Reading blue band data...") };
            let input_b = Arc::new(Raster::new(&blue_file, "r")?);

            let rows = input_r.configs.rows as isize;
            let columns = input_r.configs.columns as isize;
            let nodata_r = input_r.configs.nodata;
            let nodata_g = input_g.configs.nodata;
            let nodata_b = input_b.configs.nodata;
            let red_min = input_r.configs.display_min;
            let green_min = input_g.configs.display_min;
            let blue_min = input_b.configs.display_min;
            let red_max = input_r.configs.display_max;
            let green_max = input_g.configs.display_max;
            let blue_max = input_b.configs.display_max;
            let overall_min = red_min.min(green_min.min(blue_min));
            let overall_max = red_max.max(green_max.max(blue_max));
            let range = overall_max - overall_min;

            let start = time::now();

            // make sure the input files have the same size
            if input_r.configs.rows != input_g.configs.rows || input_r.configs.columns != input_g.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                                    "The input files must have the same number of rows and columns and spatial extent."));
            }
            if input_r.configs.rows != input_b.configs.rows || input_r.configs.columns != input_b.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                                    "The input files must have the same number of rows and columns and spatial extent."));
            }

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_r = input_r.clone();
                let input_g = input_g.clone();
                let input_b = input_b.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let (mut i, mut h, mut s, mut m): (f64, f64, f64, f64);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut intensity_data = vec![nodata_r; columns as usize];
                        let mut hue_data = vec![nodata_r; columns as usize];
                        let mut saturation_data = vec![nodata_r; columns as usize];
                        for col in 0..columns {
                            r = input_r[(row, col)];
                            g = input_g[(row, col)];
                            b = input_b[(row, col)];
                            if r != nodata_r && g != nodata_g && b != nodata_b {
                                r = (r - overall_min) / range;
                                if r < 0f64 { r = 0f64; }
                                if r > 1f64 { r = 1f64; }
                                
                                g = (g - overall_min) / range;
                                if g < 0f64 { g = 0f64; }
                                if g > 1f64 { g = 1f64; }
                                
                                b = (b - overall_min) / range;
                                if b < 0f64 { b = 0f64; }
                                if b > 1f64 { b = 1f64; }
                                
                                m = r.min(g.min(b));
                            
                                i = r + g + b;
                                
                                if i == 3f64 { 
                                    h = 0f64;
                                } else if m == b {
                                    h = (g - b) / (i - 3f64 * b);
                                } else if m == r {
                                    h = (b - r) / (i - 3f64 * r) + 1f64;
                                } else { // m == g
                                    h = (r - g) / (i - 3f64 * g) + 2f64;
                                }
                                
                                if h <= 1f64 {
                                    s = (i - 3f64 * b) / i;
                                } else if h <= 2f64 {
                                    s = (i - 3f64 * r) / i;
                                } else { // H <= 3
                                    s = (i - 3f64 * g) / i;
                                }

                                intensity_data[col as usize] = i;
                                hue_data[col as usize] = h;
                                saturation_data[col as usize] = s;
                            }
                        }
                        tx.send((row, intensity_data, hue_data, saturation_data)).unwrap();
                    }
                });
            }

            let mut output_i = Raster::initialize_using_file(&intensity_file, &input_r);
            output_i.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_i.configs.data_type = DataType::F32;
            
            let mut output_h = Raster::initialize_using_file(&hue_file, &input_r);
            output_h.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_h.configs.data_type = DataType::F32;
            
            let mut output_s = Raster::initialize_using_file(&saturation_file, &input_r);
            output_s.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_s.configs.data_type = DataType::F32;

            for row in 0..rows {
                let data = rx.recv().unwrap();
                output_i.set_row_data(data.0, data.1);
                output_h.set_row_data(data.0, data.2);
                output_s.set_row_data(data.0, data.3);
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
            
            output_i.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            // output_i.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_i.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving intensity data...") };
            let _ = match output_i.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_h.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            // output_h.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_h.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving hue data...") };
            let _ = match output_h.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_s.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            // output_s.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_s.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving saturation data...") };
            let _ = match output_s.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        } else {
            if verbose { println!("Reading image data...") };
            let input = Arc::new(Raster::new(&composite_file, "r")?);
            let rows = input.configs.rows as isize;
            let columns = input.configs.columns as isize;
            let nodata = input.configs.nodata;

            let start = time::now();

            // find the overall minimum and range
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut overall_min = f64::INFINITY;
                    let mut overall_max = f64::NEG_INFINITY;
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        for col in 0..columns {
                            z = input[(row, col)];
                            if z != nodata {
                                r = (z as u32 & 0xFF) as f64;
                                g = ((z as u32 >> 8) & 0xFF) as f64;
                                b = ((z as u32 >> 16) & 0xFF) as f64;

                                if r < overall_min { overall_min = r; }
                                if r > overall_max { overall_max = r; }

                                if g < overall_min { overall_min = g; }
                                if g > overall_max { overall_max = g; }

                                if b < overall_min { overall_min = b; }
                                if b > overall_max { overall_max = b; }
                            }
                        }
                    }
                    tx.send((overall_min, overall_max)).unwrap();
                });
            }

            let mut overall_min = f64::INFINITY;
            let mut overall_max = f64::NEG_INFINITY;
            for tid in 0..num_procs {
                let data = rx.recv().unwrap();
                if data.0 < overall_min { overall_min = data.0; }
                if data.1 > overall_max { overall_max = data.1; }
                if verbose {
                    progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let range = overall_max - overall_min;
            
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let (mut r, mut g, mut b): (f64, f64, f64);
                    let (mut i, mut h, mut s, mut m): (f64, f64, f64, f64);
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut intensity_data = vec![nodata; columns as usize];
                        let mut hue_data = vec![nodata; columns as usize];
                        let mut saturation_data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input[(row, col)];
                            if z != nodata {
                                r = (z as u32 & 0xFF) as f64;
                                g = ((z as u32 >> 8) & 0xFF) as f64;
                                b = ((z as u32 >> 16) & 0xFF) as f64;

                                r = (r - overall_min) / range;
                                if r < 0f64 { r = 0f64; }
                                if r > 1f64 { r = 1f64; }
                                
                                g = (g - overall_min) / range;
                                if g < 0f64 { g = 0f64; }
                                if g > 1f64 { g = 1f64; }
                                
                                b = (b - overall_min) / range;
                                if b < 0f64 { b = 0f64; }
                                if b > 1f64 { b = 1f64; }
                                
                                m = r.min(g.min(b));
                            
                                i = r + g + b;
                                
                                if i == 3f64 { 
                                    h = 0f64;
                                } else if m == b {
                                    h = (g - b) / (i - 3f64 * b);
                                } else if m == r {
                                    h = (b - r) / (i - 3f64 * r) + 1f64;
                                } else { // m == g
                                    h = (r - g) / (i - 3f64 * g) + 2f64;
                                }
                                
                                if h <= 1f64 {
                                    s = (i - 3f64 * b) / i;
                                } else if h <= 2f64 {
                                    s = (i - 3f64 * r) / i;
                                } else { // H <= 3
                                    s = (i - 3f64 * g) / i;
                                }

                                intensity_data[col as usize] = i;
                                hue_data[col as usize] = h;
                                saturation_data[col as usize] = s;
                            }
                        }
                        tx.send((row, intensity_data, hue_data, saturation_data)).unwrap();
                    }
                });
            }

            let mut output_i = Raster::initialize_using_file(&intensity_file, &input);
            output_i.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_i.configs.data_type = DataType::F32;
            
            let mut output_h = Raster::initialize_using_file(&hue_file, &input);
            output_h.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_h.configs.data_type = DataType::F32;
            
            let mut output_s = Raster::initialize_using_file(&saturation_file, &input);
            output_s.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_s.configs.data_type = DataType::F32;

            for row in 0..rows {
                let data = rx.recv().unwrap();
                output_i.set_row_data(data.0, data.1);
                output_h.set_row_data(data.0, data.2);
                output_s.set_row_data(data.0, data.3);
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
            
            output_i.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_i.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_i.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving intensity data...") };
            let _ = match output_i.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_h.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_h.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_h.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving hue data...") };
            let _ = match output_h.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_s.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_s.add_metadata_entry(format!("Input colour composite file: {}", composite_file));
            output_s.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving saturation data...") };
            let _ = match output_s.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}