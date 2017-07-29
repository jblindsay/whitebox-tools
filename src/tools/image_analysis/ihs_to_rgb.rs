/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 25, 2017
Last Modified: July 25, 2017
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
pub struct IhsToRgb {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl IhsToRgb {

    /// Public constructor.
    pub fn new() -> IhsToRgb {
        let name = "IhsToRgb".to_string();
        
        let description = "Converts intensity, hue, and saturation (IHS) images into red, green, and blue (RGB) images.".to_string();
        
        let mut parameters = "--intensity   Input intensity raster file.\n".to_owned();
        parameters.push_str("--hue          Input hue raster file.\n");
        parameters.push_str("--saturation   Input saturation file.\n");
        parameters.push_str("--red          Output red band raster file.\n");
        parameters.push_str("--green        Output green raster file.\n");
        parameters.push_str("--blue         Output blue raster file.\n");
        parameters.push_str("--composite    Optional output colour-composite image file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --intensity=intensity.dep --hue=hue.dep --saturation=saturation.dep --red=band3.dep --green=band2.dep --blue=band1.dep
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --intensity=intensity.dep --hue=hue.dep --saturation=saturation.dep --composite=image.dep", short_exe, name).replace("*", &sep);
    
        IhsToRgb { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for IhsToRgb {
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
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "-composite" || vec[0].to_lowercase() == "--composite" {
                if keyval {
                    composite_file = vec[1].to_string();
                } else {
                    composite_file = args[i+1].to_string();
                }
                use_composite = true;
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
        if !intensity_file.contains(&sep) {
            intensity_file = format!("{}{}", working_directory, intensity_file);
        }
        if !hue_file.contains(&sep) {
            hue_file = format!("{}{}", working_directory, hue_file);
        }
        if !saturation_file.contains(&sep) {
            saturation_file = format!("{}{}", working_directory, saturation_file);
        }

        if verbose { println!("Reading intensity band data...") };
        let input_i = Arc::new(Raster::new(&intensity_file, "r")?);
        if verbose { println!("Reading hue band data...") };
        let input_h = Arc::new(Raster::new(&hue_file, "r")?);
        if verbose { println!("Reading saturation band data...") };
        let input_s = Arc::new(Raster::new(&saturation_file, "r")?);

        let rows = input_i.configs.rows as isize;
        let columns = input_i.configs.columns as isize;
        let nodata_i = input_i.configs.nodata;
        let nodata_h = input_h.configs.nodata;
        let nodata_s = input_s.configs.nodata;
        
        let start = time::now();

        // make sure the input files have the same size
        if input_i.configs.rows != input_h.configs.rows || input_i.configs.columns != input_h.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }
        if input_i.configs.rows != input_s.configs.rows || input_i.configs.columns != input_s.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input_i = input_i.clone();
            let input_h = input_h.clone();
            let input_s = input_s.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut r, mut g, mut b): (f64, f64, f64);
                let (mut i, mut h, mut s): (f64, f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut red_data = vec![nodata_i; columns as usize];
                    let mut green_data = vec![nodata_i; columns as usize];
                    let mut blue_data = vec![nodata_i; columns as usize];
                    for col in 0..columns {
                        i = input_i[(row, col)];
                        h = input_h[(row, col)];
                        s = input_s[(row, col)];
                        if i != nodata_i && h != nodata_h && s != nodata_s {
                            if h <= 1f64 {
                                r = i * (1f64 + 2f64 * s - 3f64 * s * h) / 3f64;
                                g = i * (1f64 - s + 3f64 * s * h) / 3f64;
                                b = i * (1f64 - s) / 3f64;
                            } else if h <= 2f64 {
                                r = i * (1f64 - s) / 3f64;
                                g = i * (1f64 + 2f64 * s - 3f64 * s * (h - 1f64)) / 3f64;
                                b = i * (1f64 - s + 3f64 * s * (h - 1f64)) / 3f64;
                            } else { // h <= 3
                                r = i * (1f64 - s + 3f64 * s * (h - 2f64)) / 3f64;
                                g = i * (1f64 - s) / 3f64;
                                b = i * (1f64 + 2f64 * s - 3f64 * s * (h - 2f64)) / 3f64;
                            }

                            red_data[col as usize] = r;
                            green_data[col as usize] = g;
                            blue_data[col as usize] = b;
                        }
                    }
                    tx.send((row, red_data, green_data, blue_data)).unwrap();
                }
            });
        }

        if !use_composite {
            let mut output_r = Raster::initialize_using_file(&red_file, &input_i);
            output_r.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_r.configs.data_type = DataType::F32;
            
            let mut output_g = Raster::initialize_using_file(&green_file, &input_i);
            output_g.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_g.configs.data_type = DataType::F32;
            
            let mut output_b = Raster::initialize_using_file(&blue_file, &input_i);
            output_b.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_b.configs.data_type = DataType::F32;
            
            for row in 0..rows {
                let data = rx.recv().unwrap();
                output_r.set_row_data(data.0, data.1);
                output_g.set_row_data(data.0, data.2);
                output_b.set_row_data(data.0, data.3);
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
            
            output_r.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_r.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_r.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_r.add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_r.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving red data...") };
            let _ = match output_r.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_g.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_g.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_g.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_g.add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_g.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving green data...") };
            let _ = match output_g.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            output_b.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output_b.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_b.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_b.add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_b.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving blue data...") };
            let _ = match output_b.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        } else {
            let mut output = Raster::initialize_using_file(&composite_file, &input_i);
            output.configs.photometric_interp = PhotometricInterpretation::RGB;
            output.configs.data_type = DataType::I32;
            let out_nodata = 0f64;
            let (mut r, mut g, mut b): (u32, u32, u32);
            for row in 0..rows {
                let data = rx.recv().unwrap();
                let mut out_data = vec![out_nodata; columns as usize];
                for col in 0..columns {
                    r = data.1[col as usize] as u32;
                    g = data.2[col as usize] as u32;
                    b = data.3[col as usize] as u32;
                    out_data[col as usize] = ((255 << 24) | (b << 16) | (g << 8) | r) as f64;
                }
                output.set_row_data(data.0, out_data);
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
            output.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output.add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving red data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };

            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}