/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 5, 2017
Last Modified: 13/10/2018
License: MIT

NOTES: The input image should contain integer values but floating point data will be handled using a multiplier.
*/

use num_cpus;
use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use tools::*;

/// Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.
pub struct DiversityFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DiversityFilter {
    /// Public constructor.
    pub fn new() -> DiversityFilter {
        let name = "DiversityFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Assigns each cell in the output grid the number of different values in a moving window centred on each grid cell in the input raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            name: "Filter X-Dimension".to_owned(),
            flags: vec!["--filterx".to_owned()],
            description: "Size of the filter kernel in the x-direction.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Filter Y-Dimension".to_owned(),
            flags: vec!["--filtery".to_owned()],
            description: "Size of the filter kernel in the y-direction.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --filter=25",
            short_exe, name
        ).replace("*", &sep);

        DiversityFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DiversityFilter {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut output_file = String::new();
        let mut filter_size_x = 11usize;
        let mut filter_size_y = 11usize;
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
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
                filter_size_y = filter_size_x;
            } else if vec[0].to_lowercase() == "-filterx" || vec[0].to_lowercase() == "--filterx" {
                if keyval {
                    filter_size_x = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_x = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            } else if vec[0].to_lowercase() == "-filtery" || vec[0].to_lowercase() == "--filtery" {
                if keyval {
                    filter_size_y = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    filter_size_y = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if filter_size_x < 3 {
            filter_size_x = 3;
        }
        if filter_size_y < 3 {
            filter_size_y = 3;
        }

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size_x as f64 / 2f64).floor() == (filter_size_x as f64 / 2f64) {
            filter_size_x += 1;
        }
        if (filter_size_y as f64 / 2f64).floor() == (filter_size_y as f64 / 2f64) {
            filter_size_y += 1;
        }

        // let (mut z, mut z_n): (f64, f64);
        let midpoint_x = (filter_size_x as f64 / 2f64).floor() as isize;
        let midpoint_y = (filter_size_y as f64 / 2f64).floor() as isize;
        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);

        /* 
        Need to know if the image contains integer or floating point values.
        If it is floating point values, then a non-unit multiplier must be used.
        */
        let mut multiplier = 1.0;
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        if min_val.floor() != min_val || max_val.floor() != max_val {
            multiplier = 1000.0;
        }
        let min_val_mult = min_val * multiplier;
        let num_bins = (max_val * multiplier - min_val_mult).ceil() as usize + 1;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut bin_val: usize;
                let (mut start_col, mut end_col, mut start_row, mut end_row): (
                    isize,
                    isize,
                    isize,
                    isize,
                );
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
                    let mut data = vec![nodata; columns as usize];
                    let mut histo = vec![0; num_bins];
                    let mut diversity = 0;
                    let mut z: f64;
                    for col in 0..columns {
                        if col > 0 {
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;

                            // remove the trailing column from the histo
                            for row2 in start_row..=end_row {
                                z = input.get_value(row2, start_col - 1);
                                if z != nodata {
                                    bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                    histo[bin_val] -= 1;
                                    if histo[bin_val] == 0 {
                                        diversity -= 1;
                                    }
                                }
                            }

                            // add the leading column to the histo
                            for row2 in start_row..=end_row {
                                z = input.get_value(row2, end_col);
                                if z != nodata {
                                    bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                    if histo[bin_val] == 0 {
                                        diversity += 1;
                                    }
                                    histo[bin_val] += 1;
                                }
                            }
                        } else {
                            // initialize the filter histo
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            for col2 in start_col..=end_col {
                                for row2 in start_row..=end_row {
                                    z = input.get_value(row2, col2);
                                    if z != nodata {
                                        bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                        if histo[bin_val] == 0 {
                                            diversity += 1;
                                        }
                                        histo[bin_val] += 1;
                                    }
                                }
                            }
                        }
                        if input.get_value(row, col) != nodata {
                            data[col as usize] = diversity as f64;
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
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
