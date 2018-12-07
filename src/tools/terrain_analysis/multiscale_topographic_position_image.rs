/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 19, 2017
Last Modified: 12/10/2017
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Creates a multiscale topographic position image from three DEVmax rasters of differing spatial scale ranges.
pub struct MultiscaleTopographicPositionImage {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultiscaleTopographicPositionImage {
    /// Public constructor.
    pub fn new() -> MultiscaleTopographicPositionImage {
        let name = "MultiscaleTopographicPositionImage".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Creates a multiscale topographic position image from three DEVmax rasters of differing spatial scale ranges.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Local-Scale File".to_owned(),
            flags: vec!["--local".to_owned()],
            description: "Input local-scale topographic position (DEVmax) raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Meso-Scale File".to_owned(),
            flags: vec!["--meso".to_owned()],
            description: "Input meso-scale topographic position (DEVmax) raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Broad-Scale File".to_owned(),
            flags: vec!["--broad".to_owned()],
            description: "Input broad-scale topographic position (DEVmax) raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Image Lightness Value".to_owned(),
            flags: vec!["--lightness".to_owned()],
            description: "Image lightness value (default is 1.2).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.2".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --local=DEV_local.tif --meso=DEV_meso.tif --broad=DEV_broad.tif -o=output.tif --lightness=1.5", short_exe, name).replace("*", &sep);

        MultiscaleTopographicPositionImage {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultiscaleTopographicPositionImage {
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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input1_file = String::new();
        let mut input2_file = String::new();
        let mut input3_file = String::new();
        let mut output_file = String::new();
        let mut cutoff = 1.2f64;
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
            ));
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
            if vec[0].to_lowercase() == "-broad" || vec[0].to_lowercase() == "--broad" {
                if keyval {
                    input1_file = vec[1].to_string();
                } else {
                    input1_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-meso" || vec[0].to_lowercase() == "--meso" {
                if keyval {
                    input2_file = vec[1].to_string();
                } else {
                    input2_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-local" || vec[0].to_lowercase() == "--local" {
                if keyval {
                    input3_file = vec[1].to_string();
                } else {
                    input3_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-lightness"
                || vec[0].to_lowercase() == "--lightness"
            {
                if keyval {
                    cutoff = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    cutoff = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if !input1_file.contains(&sep) && input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) && input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }
        if !input3_file.contains(&sep) && input3_file.contains("/") {
            input3_file = format!("{}{}", working_directory, input3_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading broad-scale DEV data...")
        };
        let input_r = Arc::new(Raster::new(&input1_file, "r")?);
        if verbose {
            println!("Reading meso-scale DEV data...")
        };
        let input_g = Arc::new(Raster::new(&input2_file, "r")?);
        if verbose {
            println!("Reading local-scale DEV data...")
        };
        let input_b = Arc::new(Raster::new(&input3_file, "r")?);

        let rows = input_r.configs.rows as isize;
        let columns = input_r.configs.columns as isize;
        let nodata_r = input_r.configs.nodata;
        let nodata_g = input_g.configs.nodata;
        let nodata_b = input_b.configs.nodata;
        // let red_min = input_r.configs.display_min;
        // let green_min = input_g.configs.display_min;
        // let blue_min = input_b.configs.display_min;
        // let red_range = input_r.configs.display_max - red_min;
        // let green_range = input_g.configs.display_max - green_min;
        // let blue_range = input_b.configs.display_max - blue_min;

        let start = Instant::now();

        // make sure the input files have the same size
        if input_r.configs.rows != input_g.configs.rows
            || input_r.configs.columns != input_g.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }
        if input_r.configs.rows != input_b.configs.rows
            || input_r.configs.columns != input_b.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input_r = input_r.clone();
            let input_g = input_g.clone();
            let input_b = input_b.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut red_val: f64;
                let mut green_val: f64;
                let mut blue_val: f64;
                let (mut red, mut green, mut blue): (u32, u32, u32);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata_r; columns as usize];
                    for col in 0..columns {
                        red_val = input_r[(row, col)];
                        green_val = input_g[(row, col)];
                        blue_val = input_b[(row, col)];
                        if red_val != nodata_r && green_val != nodata_g && blue_val != nodata_b {
                            /* Replaced the linear interpolation with this logistic function.*/
                            red_val = (512f64 / (1f64 + (-cutoff * (red_val).abs()).exp())).floor()
                                - 256f64;
                            green_val = (512f64 / (1f64 + (-cutoff * (green_val).abs()).exp()))
                                .floor()
                                - 256f64;
                            blue_val = (512f64 / (1f64 + (-cutoff * (blue_val).abs()).exp()))
                                .floor()
                                - 256f64;

                            if red_val < 0f64 {
                                red_val = 0f64;
                            }
                            if red_val > 255f64 {
                                red_val = 255f64;
                            }
                            red = red_val as u32;

                            if green_val < 0f64 {
                                green_val = 0f64;
                            }
                            if green_val > 255f64 {
                                green_val = 255f64;
                            }
                            green = green_val as u32;

                            if blue_val < 0f64 {
                                blue_val = 0f64;
                            }
                            if blue_val > 255f64 {
                                blue_val = 255f64;
                            }
                            blue = blue_val as u32;

                            data[col as usize] =
                                ((255 << 24) | (blue << 16) | (green << 8) | red) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input_r);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::I32;
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input broad-scale image file: {}", input1_file));
        output.add_metadata_entry(format!("Input meso-scale image file: {}", input2_file));
        output.add_metadata_entry(format!("Input local-scale image file: {}", input3_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
