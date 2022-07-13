/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::collections::HashSet;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool performs a majority (or modal) filter on a raster image. A mode filter assigns each
/// cell in the output grid the most commonly occurring value, i.e. mode, in a moving window centred
/// on each grid cell. Mode filters should only be applied to input images of a categorical data
/// scale. The input image should contain integer values but floating point data will be handled using a multiplier.
/// Because it requires binning the values in the window, a relatively computationally intensive
/// task, `MajorityFilter` is considerably less efficient than other smoothing filters. This may pose a problem
/// for large images or large neighbourhoods. Like all WhiteboxTools' filters, however, this tool is
/// parallelized, benefitting from multi-core processors, and the tool also takes advantage of the redundancy of
/// the overlapping areas of filter windows along a row of data.
///
/// Neighbourhood size, or filter size, is determined by the user-defined x and y dimensions. These dimensions
/// should be odd, positive integer values (e.g. 3, 5, 7, 9, etc.).
///
/// NoData values in the input image are ignored during filtering. When the neighbourhood around a grid cell extends
/// beyond the edge of the grid, NoData values are assigned to these sites. In the event of multiple modes, i.e.
/// neighbourhoods for which there is more than one class with tied and maximal frequency within the neighbourhood,
/// the tool will report the first-discovered class value in the output raster. This is unlikely to be an issue
/// for larger filter windows, but may be more problematic at smaller window sizes.
///
/// # See Also
/// `MedianFilter`
pub struct MajorityFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MajorityFilter {
    /// Public constructor.
    pub fn new() -> MajorityFilter {
        let name = "MajorityFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Assigns each cell in the output grid the most frequently occurring value (mode) in a moving window centred on each grid cell in the input raster.".to_string();

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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --filter=25",
            short_exe, name
        )
        .replace("*", &sep);

        MajorityFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MajorityFilter {
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
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-filter" {
                if keyval {
                    filter_size_x = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_x = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
                filter_size_y = filter_size_x;
            } else if flag_val == "-filterx" {
                if keyval {
                    filter_size_x = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_x = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
            } else if flag_val == "-filtery" {
                if keyval {
                    filter_size_y = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size_y = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
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
            multiplier = 100.0;
        }
        let min_val_mult = min_val * multiplier;
        let num_bins = (max_val * multiplier - min_val_mult).ceil() as usize + 1;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
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
                    let mut set = HashSet::new();
                    // I realize that the above two lines could be combined
                    // to use a HashMap instead of a Vec and a HashSet. Trouble is
                    // Rust's HashMap is painful to use.
                    let mut mode_bin = 0usize;
                    let mut mode_freq = 0usize;
                    let mut z: f64;
                    for col in 0..columns {
                        if col > 0 {
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            // remove the trailing column from the histo
                            for row2 in start_row..end_row + 1 {
                                z = input.get_value(row2, start_col - 1);
                                if z != nodata {
                                    bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                    histo[bin_val] -= 1;
                                    if histo[bin_val] == 0 {
                                        set.remove(&bin_val);
                                    }
                                }
                            }

                            // add the leading column to the histo
                            for row2 in start_row..end_row + 1 {
                                z = input.get_value(row2, end_col);
                                if z != nodata {
                                    bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                    histo[bin_val] += 1;
                                    if histo[bin_val] > histo[mode_bin] {
                                        mode_freq = histo[bin_val];
                                        mode_bin = bin_val;
                                    }
                                    if histo[bin_val] == 1 {
                                        set.insert(bin_val);
                                    }
                                }
                            }

                            if histo[mode_bin] < mode_freq {
                                mode_freq = histo[mode_bin];
                                for x in &set {
                                    if histo[*x] > mode_freq {
                                        mode_freq = histo[*x];
                                        mode_bin = *x;
                                    }
                                }
                            }
                        } else {
                            // initialize the filter histo
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            for col2 in start_col..end_col + 1 {
                                for row2 in start_row..end_row + 1 {
                                    z = input.get_value(row2, col2);
                                    if z != nodata {
                                        bin_val = (z * multiplier - min_val_mult).floor() as usize;
                                        histo[bin_val] += 1;
                                        if histo[bin_val] > mode_freq {
                                            mode_freq = histo[bin_val];
                                            mode_bin = bin_val;
                                        }
                                        if histo[bin_val] == 1 {
                                            set.insert(bin_val);
                                        }
                                    }
                                }
                            }
                        }
                        if input.get_value(row, col) != nodata {
                            data[col as usize] = (mode_bin as f64 + min_val_mult) / multiplier;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

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
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size x: {}", filter_size_x));
        output.add_metadata_entry(format!("Filter size y: {}", filter_size_y));
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
