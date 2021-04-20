/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/06/2017
Last Modified: 30/01/2020
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

/// This tool can be used to perform a Laplacian filter on a raster image. A Laplacian filter can be used
/// to emphasize the edges in an image. As such, this filter type is commonly used in edge-detection
/// applications. The algorithm operates by convolving a kernel of weights with each grid cell and its
/// neighbours in an image. Four 3x3 sized filters and one 5x5 filter are available for selection. The
/// weights of the kernels are as follows:
///
/// 3x3(1)
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// | 0  | -1  | 0  |
/// | -1 |  4  | -1 |
/// | 0  | -1  | 0  |
///
/// 3x3(2)
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// | 0  | -1  | 0  |
/// | -1 |  5  | -1 |
/// | 0  | -1  | 0  |
///
/// 3x3(3)
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// | -1 | -1  | -1 |
/// | -1 |  8  | -1 |
/// | -1 | -1  | -1 |
///
/// 3x3(4)
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// |  1 | -2  |  1 |
/// | -2 |  4  | -2 |
/// |  1 | -2  |  1 |
///
/// 5x5(1)
///
/// | .  |  . |  . | .  |  . |
/// |:--:|:--:|:--:|:--:|:--:|
/// |  0 |  0 | -1 |  0 |  0 |
/// |  0 | -1 | -2 | -1 |  0 |
/// | -1 | -2 | 17 | -2 | -1 |
/// |  0 | -1 | -2 | -1 |  0 |
/// |  0 |  0 | -1 |  0 |  0 |
///
/// 5x5(2)
///
/// | .  |  . |  . | .  |  . |
/// |:--:|:--:|:--:|:--:|:--:|
/// |  0 |  0 | -1 |  0 |  0 |
/// |  0 | -1 | -2 | -1 |  0 |
/// | -1 | -2 | 16 | -2 | -1 |
/// |  0 | -1 | -2 | -1 |  0 |
/// |  0 |  0 | -1 |  0 |  0 |
///
/// The user must specify the `--variant`, including '3x3(1)', '3x3(2)', '3x3(3)', '3x3(4)', '5x5(1)', and '5x5(2)'.
/// The user may also optionally clip the output image distribution tails by a specified amount (e.g. 1%).
///
/// # See Also
/// `PrewittFilter`, `SobelFilter`
pub struct LaplacianFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LaplacianFilter {
    pub fn new() -> LaplacianFilter {
        // public constructor
        let name = "LaplacianFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a Laplacian filter on an image.".to_string();

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

        parameters.push(ToolParameter{
            name: "Variant".to_owned(), 
            flags: vec!["--variant".to_owned()], 
            description: "Optional variant value. Options include 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), and 5x5(2) (default is 3x3(1)).".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["3x3(1)".to_owned(), "3x3(2)".to_owned(), "3x3(3)".to_owned(), "3x3(4)".to_owned(), "5x5(1)".to_owned(), "5x5(2)".to_owned()]),
            default_value: Some("3x3(1)".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Distribution Tail Clip Amount (%)".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Optional amount to clip the distribution tails by, in percent."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --variant='3x3(1)' --clip=1.0", short_exe, name).replace("*", &sep);

        LaplacianFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LaplacianFilter {
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
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }

        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut variant = "3x3(1)".to_string();
        let mut clip_amount = 0.0;
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
            } else if flag_val == "-variant" {
                if keyval {
                    variant = vec[1].to_string();
                } else {
                    variant = args[i + 1].to_string();
                }
            } else if flag_val == "-clip" {
                if keyval {
                    clip_amount = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    clip_amount = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
                if clip_amount < 0.0 {
                    clip_amount = 0.0;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let variant = variant.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let input_fn: Box<dyn Fn(isize, isize) -> f64> = if !is_rgb_image {
                    Box::new(|row: isize, col: isize| -> f64 { input.get_value(row, col) })
                } else {
                    Box::new(|row: isize, col: isize| -> f64 {
                        let value = input.get_value(row, col);
                        if value != nodata {
                            return value2i(value);
                        }
                        nodata
                    })
                };
                let mut z: f64;
                let mut zn: f64;
                let weights: Vec<f64>;
                let dx: Vec<isize>;
                let dy: Vec<isize>;

                if variant.contains("3x3(1)") {
                    weights = vec![0.0, -1.0, 0.0, -1.0, 4.0, -1.0, 0.0, -1.0, 0.0];
                    dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                    dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                } else if variant.contains("3x3(2)") {
                    weights = vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0];
                    dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                    dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                } else if variant.contains("3x3(3)") {
                    weights = vec![-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0];
                    dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                    dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                } else if variant.contains("3x3(4)") {
                    weights = vec![1.0, -2.0, 1.0, -2.0, 4.0, -2.0, 1.0, -2.0, 1.0];
                    dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                    dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                } else if variant.contains("5x5(1)") {
                    weights = vec![
                        0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 17.0,
                        -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0,
                    ];
                    dx = vec![
                        -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1,
                        0, 1, 2,
                    ];
                    dy = vec![
                        -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2,
                        2, 2, 2,
                    ];
                } else {
                    // 5 x 5 (2)
                    weights = vec![
                        0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 16.0,
                        -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0,
                    ];
                    dx = vec![
                        -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1,
                        0, 1, 2,
                    ];
                    dy = vec![
                        -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2,
                        2, 2, 2,
                    ];
                }

                let num_pixels_in_filter = dx.len();
                let mut sum: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
                        if z != nodata {
                            sum = 0.0;
                            for i in 0..num_pixels_in_filter {
                                zn = input_fn(row + dy[i], col + dx[i]);
                                if zn == nodata {
                                    zn = z; // replace it with z
                                }
                                sum += zn * weights[i];
                            }
                            data[col as usize] = sum;
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

        if clip_amount > 0.0 {
            println!("Clipping output...");
            output.clip_min_and_max_by_percent(clip_amount);
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Variant: {}", variant));
        output.add_metadata_entry(format!("Clip amount: {}", clip_amount));
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

fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    (r + g + b) / 3f64
}
