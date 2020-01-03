/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/06/2017
Last Modified: 12/06/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Relative topographic position (RTP) is an index of local topographic position (i.e. how
/// elevated or low-lying a site is relative to its surroundings) and is a modification of percent
/// elevation range (PER; `PercentElevRange`) and accounts for the elevation distribution. Rather than
/// positioning the central cell's elevation solely between the filter extrema, RTP is a piece-wise
/// function that positions the central elevation relative to the minimum (z<sub>min</sub>), mean (&mu;),
/// and maximum values (z<sub>max</sub>), within a local neighbourhood of a user-specified size (`--filterx`,
/// `--filtery`), such that:
///
/// > RTP = (z<sub>0</sub> − &mu;) / (&mu; − z<sub>min</sub>), if z<sub>0</sub> < &mu;
/// >
/// > OR
/// >
/// > RTP = (z<sub>0</sub> − &mu;) / (z<sub>max</sub> - &mu;), if z<sub>0</sub> >= &mu;
/// 
///
/// The resulting index is bound by the interval [−1, 1], where the sign indicates if the cell is above or below
/// than the filter mean. Although RTP uses the mean to define two linear functions, the reliance on the filter
/// extrema is expected to result in sensitivity to outliers. Furthermore, the use of the mean implies assumptions
/// of unimodal and symmetrical elevation distribution.
///
/// In many cases, Elevation Percentile (`ElevPercentile`) and deviation from mean elevation (`DevFromMeanElev`)
/// provide more suitable and robust measures of relative topographic position.
///
/// # Reference
/// Newman, D. R., Lindsay, J. B., and Cockburn, J. M. H. (2018). Evaluating metrics of local topographic
/// position for multiscale geomorphometric analysis. Geomorphology, 312, 40-50.
///
/// # See Also
/// `DevFromMeanElev`, `DiffFromMeanElev`, `ElevPercentile`, `PercentElevRange`
pub struct RelativeTopographicPosition {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RelativeTopographicPosition {
    pub fn new() -> RelativeTopographicPosition {
        // public constructor
        let name = "RelativeTopographicPosition".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the relative topographic position index from a DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --filter=25",
            short_exe, name
        )
        .replace("*", &sep);

        RelativeTopographicPosition {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RelativeTopographicPosition {
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
            if vec[0].to_lowercase() == "-i"
                || vec[0].to_lowercase() == "--input"
                || vec[0].to_lowercase() == "--dem"
            {
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

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        let rows = input.configs.rows as isize;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let (mut z_n, mut z): (f64, f64);
                let (mut n, mut total, mut mean): (f64, f64, f64);
                let (mut min_val, mut max_val): (f64, f64);
                let (mut start_col, mut end_col, mut start_row, mut end_row): (
                    isize,
                    isize,
                    isize,
                    isize,
                );
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut filter_min_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    let mut filter_max_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    let mut valid_cells: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    let mut mean_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size_x);
                    start_row = row - midpoint_y;
                    end_row = row + midpoint_y;
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if col > 0 {
                            filter_min_vals.pop_front();
                            filter_max_vals.pop_front();
                            valid_cells.pop_front();
                            mean_vals.pop_front();
                            min_val = f64::INFINITY;
                            max_val = f64::NEG_INFINITY;
                            n = 0f64;
                            total = 0f64;
                            for row2 in start_row..end_row + 1 {
                                z_n = input.get_value(row2, col + midpoint_x);
                                if z_n != nodata {
                                    if z_n < min_val {
                                        min_val = z_n;
                                    }
                                    if z_n > max_val {
                                        max_val = z_n;
                                    }
                                    n += 1.0;
                                    total += z_n;
                                }
                            }
                            filter_min_vals.push_back(min_val);
                            filter_max_vals.push_back(max_val);
                            valid_cells.push_back(n);
                            mean_vals.push_back(total);
                        } else {
                            // initialize the filter_vals
                            start_col = col - midpoint_x;
                            end_col = col + midpoint_x;
                            for col2 in start_col..end_col + 1 {
                                min_val = f64::INFINITY;
                                max_val = f64::NEG_INFINITY;
                                n = 0f64;
                                total = 0f64;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col2);
                                    if z_n != nodata {
                                        if z_n < min_val {
                                            min_val = z_n;
                                        }
                                        if z_n > max_val {
                                            max_val = z_n;
                                        }
                                        n += 1.0;
                                        total += z_n;
                                    }
                                }
                                filter_min_vals.push_back(min_val);
                                filter_max_vals.push_back(max_val);
                                valid_cells.push_back(n);
                                mean_vals.push_back(total);
                            }
                        }
                        z = input.get_value(row, col);
                        if z != nodata {
                            min_val = f64::INFINITY;
                            max_val = f64::NEG_INFINITY;
                            n = 0f64;
                            total = 0f64;
                            for i in 0..filter_size_x {
                                if filter_min_vals[i] < min_val {
                                    min_val = filter_min_vals[i];
                                }
                                if filter_max_vals[i] > max_val {
                                    max_val = filter_max_vals[i];
                                }
                                n += valid_cells[i];
                                total += mean_vals[i];
                            }
                            if min_val < f64::INFINITY {
                                mean = total / n;
                                if z < mean {
                                    data[col as usize] = (z - mean) / (mean - min_val);
                                } else if max_val > mean {
                                    data[col as usize] = (z - mean) / (max_val - mean);
                                } else {
                                    // this will occur when max_val = mean, because there is no variation within the window
                                    data[col as usize] = 0.0;
                                }
                            }
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
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = -1.0;
        output.configs.display_max = 1.0;
        output.configs.palette = "blue_white_red.plt".to_string();
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
