/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/07/2017
Last Modified: 12/04/2019
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to split a red-green-blue (RGB) colour-composite image into three separate bands of
/// multi-spectral imagery. The user must specify the input image (`--input`) and output red, green, blue images
/// (`--red`, `--green`, `--blue`).
///
/// # See Also
/// `CreateColourComposite`
pub struct SplitColourComposite {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SplitColourComposite {
    /// Public constructor.
    pub fn new() -> SplitColourComposite {
        let name = "SplitColourComposite".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description =
            "This tool splits an RGB colour composite image into separate multispectral images."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Colour Composite Image File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input colour composite image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Red Band File".to_owned(),
            flags: vec!["--red".to_owned()],
            description: "Output red band file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Green Band File".to_owned(),
            flags: vec!["--green".to_owned()],
            description: "Output green band file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Blue Band File".to_owned(),
            flags: vec!["--blue".to_owned()],
            description: "Output blue band file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif --red=red.tif --green=green.tif --blue=blue.tif",
            short_exe, name
        )
        .replace("*", &sep);

        SplitColourComposite {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SplitColourComposite {
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
        let mut input_file = String::new();
        let mut red_file = String::new();
        let mut green_file = String::new();
        let mut blue_file = String::new();
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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-red" {
                red_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-green" {
                green_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-blue" {
                blue_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !red_file.contains(&sep) && !red_file.contains("/") {
            red_file = format!("{}{}", working_directory, red_file);
        }
        if !green_file.contains(&sep) && !green_file.contains("/") {
            green_file = format!("{}{}", working_directory, green_file);
        }
        if !blue_file.contains(&sep) && !blue_file.contains("/") {
            blue_file = format!("{}{}", working_directory, blue_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let output_nodata = -32768f64;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut in_val: f64;
                let mut val: u32;
                let (mut red, mut green, mut blue): (u32, u32, u32);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data_r = vec![output_nodata; columns as usize];
                    let mut data_g = vec![output_nodata; columns as usize];
                    let mut data_b = vec![output_nodata; columns as usize];
                    for col in 0..columns {
                        in_val = input.get_value(row, col);
                        if in_val != nodata {
                            val = in_val as u32;
                            red = val & 0xFF;
                            green = (val >> 8) & 0xFF;
                            blue = (val >> 16) & 0xFF;
                            data_r[col as usize] = red as f64;
                            data_g[col as usize] = green as f64;
                            data_b[col as usize] = blue as f64;
                        }
                    }
                    tx.send((row, data_r, data_g, data_b)).unwrap();
                }
            });
        }

        let mut output_r = Raster::initialize_using_file(&red_file, &input);
        output_r.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output_r.configs.data_type = DataType::F32;
        output_r.configs.nodata = output_nodata;

        let mut output_g = Raster::initialize_using_file(&green_file, &input);
        output_g.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output_g.configs.data_type = DataType::F32;
        output_g.configs.nodata = output_nodata;

        let mut output_b = Raster::initialize_using_file(&blue_file, &input);
        output_b.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output_b.configs.data_type = DataType::F32;
        output_b.configs.nodata = output_nodata;

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
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

        let elapsed_time = get_formatted_elapsed_time(start);

        output_r.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_r.add_metadata_entry(format!("Input file: {}", input_file));
        output_r.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        if verbose {
            println!("Saving red image...")
        };
        let _ = match output_r.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        output_g.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_g.add_metadata_entry(format!("Input file: {}", input_file));
        output_g.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        if verbose {
            println!("Saving green image...")
        };
        let _ = match output_g.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        output_b.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_b.add_metadata_entry(format!("Input file: {}", input_file));
        output_b.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        if verbose {
            println!("Saving blue image...")
        };
        let _ = match output_b.write() {
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
