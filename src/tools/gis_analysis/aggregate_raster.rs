/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 13/12/2017
Last Modified: 20/01/2019
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

/// This tool can be used to reduce the grid resolution of a raster by a user specified amount. For example, using 
/// an aggregation factor (`--agg_factor`) of 2 would result in a raster with half the number of rows and columns. 
/// The grid cell values (`--type`) in the output image will consist of the mean, sum, maximum, minimum, or range 
/// of the overlapping grid cells in the input raster (four cells in the case of an aggregation factor of 2).
/// 
/// # See Also
/// `Resample`
pub struct AggregateRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AggregateRaster {
    pub fn new() -> AggregateRaster {
        // public constructor
        let name = "AggregateRaster".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Aggregates a raster to a lower resolution.".to_string();

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
            name: "Aggregation Factor (pixels)".to_owned(),
            flags: vec!["--agg_factor".to_owned()],
            description: "Aggregation factor, in pixels.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("2".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Aggregation Type".to_owned(),
            flags: vec!["--type".to_owned()],
            description: "Statistic used to fill output pixels.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "mean".to_owned(),
                "sum".to_owned(),
                "maximum".to_owned(),
                "minimum".to_owned(),
                "range".to_owned(),
            ]),
            default_value: Some("mean".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --output_text",
            short_exe, name
        )
        .replace("*", &sep);

        AggregateRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AggregateRaster {
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
        let mut agg_factor = 2isize;
        let mut agg_type = String::from("mean");

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
            } else if flag_val == "-agg_factor" {
                if keyval {
                    agg_factor = vec[1].to_string().parse::<isize>().unwrap();
                } else {
                    agg_factor = args[i + 1].to_string().parse::<isize>().unwrap();
                }
                if agg_factor < 2isize {
                    println!(
                        "WARNING: Aggregation factor cannot be less than 2. It has been modified."
                    );
                    agg_factor = 2isize;
                }
            } else if flag_val == "-type" {
                if keyval {
                    agg_type = vec[1].to_string();
                } else {
                    agg_type = args[i + 1].to_string();
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let nodata = input.configs.nodata;
        let rows_in = input.configs.rows as isize;
        let columns_in = input.configs.columns as isize;
        let rows_out = (rows_in as f64 / agg_factor as f64).round() as isize;
        let columns_out = (columns_in as f64 / agg_factor as f64).round() as isize;

        let north = input.configs.north;
        let south = north - (input.configs.resolution_y * agg_factor as f64 * rows_out as f64);
        let west = input.configs.west;
        let east = west + (input.configs.resolution_x * agg_factor as f64 * columns_out as f64);

        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows_out as usize;
        configs.columns = columns_out as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = input.configs.resolution_x * agg_factor as f64;
        configs.resolution_y = input.configs.resolution_y * agg_factor as f64;
        configs.nodata = nodata;
        configs.data_type = DataType::F64;
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        configs.palette = input.configs.palette.clone();

        let mut output = Raster::initialize_using_config(&output_file, &configs);
        if output.configs.data_type != DataType::F32 && output.configs.data_type != DataType::F64 {
            output.configs.data_type = DataType::F32;
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();

        match agg_type.to_lowercase().trim() {
            "mean" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut z: f64;
                        let mut row_in: isize;
                        let mut col_in: isize;
                        let mut stat: f64;
                        let mut count: f64;
                        for row in (0..rows_out).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns_out as usize];
                            for col in 0..columns_out {
                                row_in = row * agg_factor;
                                col_in = col * agg_factor;
                                stat = 0f64;
                                count = 0f64;
                                for r in row_in..row_in + agg_factor {
                                    for c in col_in..col_in + agg_factor {
                                        z = input.get_value(r, c);
                                        if z != nodata {
                                            stat += z;
                                            count += 1f64;
                                        }
                                    }
                                }
                                if count > 0f64 {
                                    stat = stat / count;
                                    data[col as usize] = stat;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for r in 0..rows_out {
                    let (row, data) = rx.recv().unwrap();
                    output.set_row_data(row, data);
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows_out - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "sum" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut z: f64;
                        let mut row_in: isize;
                        let mut col_in: isize;
                        let mut stat: f64;
                        let mut count: f64;
                        for row in (0..rows_out).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns_out as usize];
                            for col in 0..columns_out {
                                row_in = row * agg_factor;
                                col_in = col * agg_factor;
                                stat = 0f64;
                                count = 0f64;
                                for r in row_in..row_in + agg_factor {
                                    for c in col_in..col_in + agg_factor {
                                        z = input.get_value(r, c);
                                        if z != nodata {
                                            stat += z;
                                            count += 1f64;
                                        }
                                    }
                                }
                                if count > 0f64 {
                                    data[col as usize] = stat;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for r in 0..rows_out {
                    let (row, data) = rx.recv().unwrap();
                    output.set_row_data(row, data);
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows_out - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "maximum" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut z: f64;
                        let mut row_in: isize;
                        let mut col_in: isize;
                        let mut stat: f64;
                        let mut count: f64;
                        for row in (0..rows_out).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns_out as usize];
                            for col in 0..columns_out {
                                row_in = row * agg_factor;
                                col_in = col * agg_factor;
                                stat = f64::NEG_INFINITY;
                                count = 0f64;
                                for r in row_in..row_in + agg_factor {
                                    for c in col_in..col_in + agg_factor {
                                        z = input.get_value(r, c);
                                        if z != nodata {
                                            if z > stat {
                                                stat = z;
                                            }
                                            count += 1f64;
                                        }
                                    }
                                }
                                if count > 0f64 {
                                    data[col as usize] = stat;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for r in 0..rows_out {
                    let (row, data) = rx.recv().unwrap();
                    output.set_row_data(row, data);
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows_out - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "minimum" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut z: f64;
                        let mut row_in: isize;
                        let mut col_in: isize;
                        let mut stat: f64;
                        let mut count: f64;
                        for row in (0..rows_out).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns_out as usize];
                            for col in 0..columns_out {
                                row_in = row * agg_factor;
                                col_in = col * agg_factor;
                                stat = f64::INFINITY;
                                count = 0f64;
                                for r in row_in..row_in + agg_factor {
                                    for c in col_in..col_in + agg_factor {
                                        z = input.get_value(r, c);
                                        if z != nodata {
                                            if z < stat {
                                                stat = z;
                                            }
                                            count += 1f64;
                                        }
                                    }
                                }
                                if count > 0f64 {
                                    data[col as usize] = stat;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for r in 0..rows_out {
                    let (row, data) = rx.recv().unwrap();
                    output.set_row_data(row, data);
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows_out - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "range" => {
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut z: f64;
                        let mut row_in: isize;
                        let mut col_in: isize;
                        let mut max_val: f64;
                        let mut min_val: f64;
                        let mut count: f64;
                        for row in (0..rows_out).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns_out as usize];
                            for col in 0..columns_out {
                                row_in = row * agg_factor;
                                col_in = col * agg_factor;
                                max_val = f64::NEG_INFINITY;
                                min_val = f64::INFINITY;
                                count = 0f64;
                                for r in row_in..row_in + agg_factor {
                                    for c in col_in..col_in + agg_factor {
                                        z = input.get_value(r, c);
                                        if z != nodata {
                                            if z > max_val {
                                                max_val = z;
                                            }
                                            if z < min_val {
                                                min_val = z;
                                            }
                                            count += 1f64;
                                        }
                                    }
                                }
                                if count > 0f64 {
                                    data[col as usize] = max_val - min_val;
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for r in 0..rows_out {
                    let (row, data) = rx.recv().unwrap();
                    output.set_row_data(row, data);
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows_out - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            _ => {
                return Err(Error::new(ErrorKind::InvalidInput, "Unrecognized aggregation type input; should be mean, sum, maximum, minimum, or range."));
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Aggregation factor: {}", agg_factor));
        output.add_metadata_entry(format!("Aggregation type: {}", agg_type));
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
