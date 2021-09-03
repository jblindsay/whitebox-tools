/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 18/07/2017
Last Modified: 13/10/2018
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Jensen (2015) describes write function memory (WFM) insertion as a simple yet effective method of visualizing
/// land-cover change between two or three dates. WFM insertion may be used to qualitatively inspect change in any
/// type of registered, multi-date imagery. The technique operates by creating a red-green-blue (RGB) colour composite
/// image based on co-registered imagery from two or three dates. If two dates are input, the first date image will be
/// put into the red channel, while the second date image will be put into both the green and blue channels. The result
/// is an image where the areas of change are displayed as red (date 1 is brighter than date 2) and cyan (date 1 is
/// darker than date 2), and areas of little change are represented in grey-tones. The larger the change in pixel
/// brightness between dates, the more intense the resulting colour will be.
///
/// If images from three dates are input, the resulting composite can contain many distinct colours. Again, more
/// intense the colours are indicative of areas of greater land-cover change among the dates, while areas of little
/// change are represented in grey-tones. Interpreting the direction of change is more difficult when three dates are
/// used. Note that for multi-spectral imagery, only one band from each date can be used for creating a WFM insertion
/// image.
///
/// # Reference
/// Jensen, J. R. (2015). Introductory Digital Image Processing: A Remote Sensing Perspective.
///
/// # See Also
/// `CreateColourComposite`, `ChangeVectorAnalysis`
pub struct WriteFunctionMemoryInsertion {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl WriteFunctionMemoryInsertion {
    /// Public constructor.
    pub fn new() -> WriteFunctionMemoryInsertion {
        let name = "WriteFunctionMemoryInsertion".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Performs a write function memory insertion for single-band multi-date change detection.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "First Date Input File".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input raster file associated with the first date.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Second Date Input File".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input raster file associated with the second date.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Third Date Input File (Optional)".to_owned(),
            flags: vec!["--i3".to_owned(), "--input3".to_owned()],
            description: "Optional input raster file associated with the third date.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i1=input1.tif -i2=input2.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        WriteFunctionMemoryInsertion {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for WriteFunctionMemoryInsertion {
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
        let mut input3_used = false;
        let mut output_file = String::new();
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
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
            if vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input1_file = vec[1].to_string();
                } else {
                    input1_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input2_file = vec[1].to_string();
                } else {
                    input2_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "--i3" || vec[0].to_lowercase() == "--input3" {
                if keyval {
                    input3_file = vec[1].to_string();
                } else {
                    input3_file = args[i + 1].to_string();
                }
                input3_used = true;
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input1_file.contains(&sep) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }
        if input3_used {
            if !input3_file.contains(&sep) && !input3_file.contains("/") {
                input3_file = format!("{}{}", working_directory, input3_file);
            }
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input_r = Arc::new(Raster::new(&input1_file, "r")?);
        let input_g = Arc::new(Raster::new(&input2_file, "r")?);
        let input_b = match input3_used {
            true => Arc::new(Raster::new(&input3_file, "r")?),
            false => Arc::new(Raster::new(&input2_file, "r")?),
        };

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

        let rows = input_r.configs.rows as isize;
        let columns = input_r.configs.columns as isize;
        let nodata_r = input_r.configs.nodata;
        let nodata_g = input_g.configs.nodata;
        let nodata_b = input_b.configs.nodata;
        let red_min = input_r.configs.display_min;
        let green_min = input_g.configs.display_min;
        let blue_min = input_b.configs.display_min;
        let red_range = input_r.configs.display_max - red_min;
        let green_range = input_g.configs.display_max - green_min;
        let blue_range = input_b.configs.display_max - blue_min;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
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
                let (mut r, mut g, mut b): (u32, u32, u32);
                let alpha_mask = (255 << 24) as u32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata_r; columns as usize];
                    for col in 0..columns {
                        red_val = input_r[(row, col)];
                        green_val = input_g[(row, col)];
                        blue_val = input_b[(row, col)];
                        if red_val != nodata_r && green_val != nodata_g && blue_val != nodata_b {
                            red_val = (red_val - red_min) / red_range * 255f64;
                            if red_val < 0f64 {
                                red_val = 0f64;
                            }
                            if red_val > 255f64 {
                                red_val = 255f64;
                            }
                            r = red_val as u32;

                            green_val = (green_val - green_min) / green_range * 255f64;
                            if green_val < 0f64 {
                                green_val = 0f64;
                            }
                            if green_val > 255f64 {
                                green_val = 255f64;
                            }
                            g = green_val as u32;

                            blue_val = (blue_val - blue_min) / blue_range * 255f64;
                            if blue_val < 0f64 {
                                blue_val = 0f64;
                            }
                            if blue_val > 255f64 {
                                blue_val = 255f64;
                            }
                            b = blue_val as u32;
                            data[col as usize] = (alpha_mask | (b << 16) | (g << 8) | r) as f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input_r);
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGBA32;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
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
        output.add_metadata_entry(format!("Input file Date 1: {}", input1_file));
        output.add_metadata_entry(format!("Input file Date 2: {}", input2_file));
        if input3_used {
            output.add_metadata_entry(format!("Input file Date 3: {}", input3_file));
        }
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
