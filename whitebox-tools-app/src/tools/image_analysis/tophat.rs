/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 28/06/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
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

/// This tool performs either a white or black [top-hat transform](https://en.wikipedia.org/wiki/Top-hat_transform)
/// on an input image. A top-hat transform is a common digital image processing operation used for various tasks, such
/// as feature extraction, background equalization, and image enhancement. The size of the rectangular *structuring element*
/// used in the filtering can be specified using the `--filterx` and `--filtery` flags.
///
/// There are two distinct types of top-hat transform including *white* and *black* top-hat transforms. The white top-hat
/// transform is defined as the difference between the input image and its [opening](https://en.wikipedia.org/wiki/Opening_(morphology))
/// by some structuring element. An opening operation is the [dilation](https://en.wikipedia.org/wiki/Dilation_(morphology))
/// (maximum filter) of an [erosion](https://en.wikipedia.org/wiki/Erosion_(morphology)) (minimum filter) image.
/// The black top-hat transform, by comparison, is defined as the difference between the
/// [closing](https://en.wikipedia.org/wiki/Closing_(morphology)) and the input image. The user specifies which of the two
/// flavours of top-hat transform the tool should perform by specifying either 'white' or 'black' with the `--variant` flag.
///
/// # See Also:
/// `Closing`, `Opening`, `MaximumFilter`, `MinimumFilter`
pub struct TophatTransform {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TophatTransform {
    pub fn new() -> TophatTransform {
        // public constructor
        let name = "TophatTransform".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description =
            "Performs either a white or black top-hat transform on an input image.".to_string();

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

        parameters.push(ToolParameter {
            name: "Variant".to_owned(),
            flags: vec!["--variant".to_owned()],
            description: "Optional variant value. Options include 'white' and 'black'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["white".to_owned(), "black".to_owned()]),
            default_value: Some("white".to_owned()),
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

        TophatTransform {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TophatTransform {
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
        let mut variant = "white".to_string();
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
            } else if flag_val == "-variant" {
                if keyval {
                    variant = vec[1].to_string();
                } else {
                    variant = args[i + 1].to_string();
                }
                if variant.to_lowercase().contains("b") {
                    variant = "black".to_string();
                } else {
                    variant = "white".to_string();
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

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        if variant == "white".to_string() {
            // first perform the erosion
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut z_n, mut z): (f64, f64);
                    let mut min_val: f64;
                    let (mut start_col, mut end_col, mut start_row, mut end_row): (
                        isize,
                        isize,
                        isize,
                        isize,
                    );
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut filter_min_vals: VecDeque<f64> =
                            VecDeque::with_capacity(filter_size_x);
                        start_row = row - midpoint_y;
                        end_row = row + midpoint_y;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            if col > 0 {
                                filter_min_vals.pop_front();
                                min_val = f64::INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col + midpoint_x);
                                    if z_n != nodata {
                                        if z_n < min_val {
                                            min_val = z_n;
                                        }
                                    }
                                }
                                filter_min_vals.push_back(min_val);
                            } else {
                                // initialize the filter_vals
                                start_col = col - midpoint_x;
                                end_col = col + midpoint_x;
                                for col2 in start_col..end_col + 1 {
                                    min_val = f64::INFINITY;
                                    for row2 in start_row..end_row + 1 {
                                        z_n = input.get_value(row2, col2);
                                        if z_n != nodata {
                                            if z_n < min_val {
                                                min_val = z_n;
                                            }
                                        }
                                    }
                                    filter_min_vals.push_back(min_val);
                                }
                            }
                            z = input.get_value(row, col);
                            if z != nodata {
                                min_val = f64::INFINITY;
                                for i in 0..filter_size_x {
                                    if filter_min_vals[i] < min_val {
                                        min_val = filter_min_vals[i];
                                    }
                                }
                                if min_val < f64::INFINITY {
                                    data[col as usize] = min_val;
                                }
                            }
                        }
                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut erosion: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                erosion.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 1 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // now perform the dilation
            let erosion = Arc::new(erosion); // wrap the erosion result in an Arc
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let input = erosion.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut z_n, mut z): (f64, f64);
                    let mut max_val: f64;
                    let (mut start_col, mut end_col, mut start_row, mut end_row): (
                        isize,
                        isize,
                        isize,
                        isize,
                    );
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut filter_max_vals: VecDeque<f64> =
                            VecDeque::with_capacity(filter_size_x);
                        start_row = row - midpoint_y;
                        end_row = row + midpoint_y;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            if col > 0 {
                                filter_max_vals.pop_front();
                                max_val = f64::NEG_INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col + midpoint_x);
                                    if z_n != nodata {
                                        if z_n > max_val {
                                            max_val = z_n;
                                        }
                                    }
                                }
                                filter_max_vals.push_back(max_val);
                            } else {
                                // initialize the filter_vals
                                start_col = col - midpoint_x;
                                end_col = col + midpoint_x;
                                for col2 in start_col..end_col + 1 {
                                    max_val = f64::NEG_INFINITY;
                                    for row2 in start_row..end_row + 1 {
                                        z_n = input.get_value(row2, col2);
                                        if z_n != nodata {
                                            if z_n > max_val {
                                                max_val = z_n;
                                            }
                                        }
                                    }
                                    filter_max_vals.push_back(max_val);
                                }
                            }
                            z = input.get_value(row, col);
                            if z != nodata {
                                max_val = f64::NEG_INFINITY;
                                for i in 0..filter_size_x {
                                    if filter_max_vals[i] > max_val {
                                        max_val = filter_max_vals[i];
                                    }
                                }
                                if max_val > f64::NEG_INFINITY {
                                    data[col as usize] = input_data[(row, col)] - max_val;
                                }
                            }
                        }
                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&output_file, &input);
            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 2 of 2): {}%", progress);
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
            output.add_metadata_entry(format!("Variant: {}", variant));
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
        } else {
            // black top-hat transform
            // first perform the dilation
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut z_n, mut z): (f64, f64);
                    let mut max_val: f64;
                    let (mut start_col, mut end_col, mut start_row, mut end_row): (
                        isize,
                        isize,
                        isize,
                        isize,
                    );
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut filter_max_vals: VecDeque<f64> =
                            VecDeque::with_capacity(filter_size_x);
                        start_row = row - midpoint_y;
                        end_row = row + midpoint_y;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            if col > 0 {
                                filter_max_vals.pop_front();
                                max_val = f64::NEG_INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col + midpoint_x);
                                    if z_n != nodata {
                                        if z_n > max_val {
                                            max_val = z_n;
                                        }
                                    }
                                }
                                filter_max_vals.push_back(max_val);
                            } else {
                                // initialize the filter_vals
                                start_col = col - midpoint_x;
                                end_col = col + midpoint_x;
                                for col2 in start_col..end_col + 1 {
                                    max_val = f64::NEG_INFINITY;
                                    for row2 in start_row..end_row + 1 {
                                        z_n = input.get_value(row2, col2);
                                        if z_n != nodata {
                                            if z_n > max_val {
                                                max_val = z_n;
                                            }
                                        }
                                    }
                                    filter_max_vals.push_back(max_val);
                                }
                            }
                            z = input.get_value(row, col);
                            if z != nodata {
                                max_val = f64::NEG_INFINITY;
                                for i in 0..filter_size_x {
                                    if filter_max_vals[i] > max_val {
                                        max_val = filter_max_vals[i];
                                    }
                                }
                                if max_val > f64::NEG_INFINITY {
                                    data[col as usize] = max_val;
                                }
                            }
                        }
                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut dilation: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                dilation.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 1 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // now perform the erosion
            let dilation = Arc::new(dilation); // wrap the erosion result in an Arc
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_data = input.clone();
                let input = dilation.clone();
                let tx1 = tx.clone();
                thread::spawn(move || {
                    let (mut z_n, mut z): (f64, f64);
                    let mut min_val: f64;
                    let (mut start_col, mut end_col, mut start_row, mut end_row): (
                        isize,
                        isize,
                        isize,
                        isize,
                    );
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut filter_min_vals: VecDeque<f64> =
                            VecDeque::with_capacity(filter_size_x);
                        start_row = row - midpoint_y;
                        end_row = row + midpoint_y;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            if col > 0 {
                                filter_min_vals.pop_front();
                                min_val = f64::INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col + midpoint_x);
                                    if z_n != nodata {
                                        if z_n < min_val {
                                            min_val = z_n;
                                        }
                                    }
                                }
                                filter_min_vals.push_back(min_val);
                            } else {
                                // initialize the filter_vals
                                start_col = col - midpoint_x;
                                end_col = col + midpoint_x;
                                for col2 in start_col..end_col + 1 {
                                    min_val = f64::INFINITY;
                                    for row2 in start_row..end_row + 1 {
                                        z_n = input.get_value(row2, col2);
                                        if z_n != nodata {
                                            if z_n < min_val {
                                                min_val = z_n;
                                            }
                                        }
                                    }
                                    filter_min_vals.push_back(min_val);
                                }
                            }
                            z = input.get_value(row, col);
                            if z != nodata {
                                min_val = f64::INFINITY;
                                for i in 0..filter_size_x {
                                    if filter_min_vals[i] < min_val {
                                        min_val = filter_min_vals[i];
                                    }
                                }
                                if min_val < f64::INFINITY {
                                    data[col as usize] = min_val - input_data[(row, col)];
                                }
                            }
                        }
                        tx1.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&output_file, &input);
            for row in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(data.0, data.1);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 2 of 2): {}%", progress);
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
            output.add_metadata_entry(format!("Variant: {}", variant));
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
        }

        Ok(())
    }
}
