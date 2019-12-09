/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/06/2017
Last Modified: 22/10/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// The Laplacian-of-Gaussian (LoG) is a spatial filter used for edge enhancement and is closely related to the 
/// difference-of-Gaussians filter (`DiffOfGaussianFilter`). The formulation of the LoG filter algorithm is based 
/// on the equation provided in the Hypermedia Image Processing Reference (HIPR) 2. The LoG operator calculates 
/// the second spatial derivative of an image. In areas where image intensity is constant, the LoG response will 
/// be zero. Near areas of change in intensity the LoG will be positive on the darker side, and negative on the 
/// lighter side. This means that at a sharp edge, or boundary, between two regions of uniform but different 
/// intensities, the LoG response will be:
/// 
/// - zero at a long distance from the edge,
/// - positive just to one side of the edge,
/// - negative just to the other side of the edge,
/// - zero at some point in between, on the edge itself.
/// 
/// The user may optionally choose to reflecting the data along image edges. **NoData** values in the input image are 
/// similarly valued in the output. The output raster is of the float data type and continuous data scale.
/// 
/// # Reference
/// 
/// Fisher, R. 2004. *Hypertext Image Processing Resources 2 (HIPR2)*. Available online: 
/// http://homepages.inf.ed.ac.uk/rbf/HIPR2/roberts.htm
/// 
/// # See Also
/// `DiffOfGaussianFilter`
pub struct LaplacianOfGaussianFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LaplacianOfGaussianFilter {
    pub fn new() -> LaplacianOfGaussianFilter {
        // public constructor
        let name = "LaplacianOfGaussianFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a Laplacian-of-Gaussian (LoG) filter on an image.".to_string();

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
            name: "Standard Deviation (Pixels)".to_owned(),
            flags: vec!["--sigma".to_owned()],
            description: "Standard deviation in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.75".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --sigma=2.0",
            short_exe, name
        )
        .replace("*", &sep);

        LaplacianOfGaussianFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LaplacianOfGaussianFilter {
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
        let mut filter_size = 0usize;
        let mut sigma = 0.75;
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
            } else if vec[0].to_lowercase() == "-sigma" || vec[0].to_lowercase() == "--sigma" {
                if keyval {
                    sigma = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if sigma < 0.5 {
            sigma = 0.5;
        } else if sigma > 20.0 {
            sigma = 20.0;
        }

        let recip_root_2_pi_times_sigma = 1.0 / ((2.0 * PI).sqrt() * sigma);
        let two_sigma_sqr = 2.0 * sigma * sigma;

        // figure out the size of the filter
        let mut weight: f64;
        for i in 0..250 {
            weight = recip_root_2_pi_times_sigma * (-1.0 * ((i * i) as f64) / two_sigma_sqr).exp();
            if weight <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }

        // the filter dimensions must be odd numbers such that there is a middle pixel
        if filter_size % 2 == 0 {
            filter_size += 1;
        }

        if filter_size < 3 {
            filter_size = 3;
        }

        let num_pixels_in_filter = filter_size * filter_size;
        let mut d_x = vec![0isize; num_pixels_in_filter];
        let mut d_y = vec![0isize; num_pixels_in_filter];
        let mut weights = vec![0.0; num_pixels_in_filter];

        let cells_on_either_side = (filter_size as f64 / 2.0).floor() as isize;

        let term1 = -1.0 / (PI * sigma * sigma * sigma * sigma);
        let mut term2: f64;
        let mut term3: f64;

        // fill the filter d_x and d_y values and the distance-weights
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size {
            for col in 0..filter_size {
                x = col as isize - cells_on_either_side;
                y = row as isize - cells_on_either_side;
                term2 = 1.0 - ((x * x + y * y) as f64 / two_sigma_sqr);
                term3 = (-((x * x + y * y) as f64) / two_sigma_sqr).exp();
                weights[a] = term1 * term2 * term3;
                d_x[a] = x;
                d_y[a] = y;
                a += 1;
            }
        }

        // let midpoint = (filter_size as f64 / 2f64).floor() as isize;
        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let d_x = Arc::new(d_x);
        let d_y = Arc::new(d_y);
        let weights = Arc::new(weights);

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

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let d_x = d_x.clone();
            let d_y = d_y.clone();
            let weights = weights.clone();
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
                let (mut sum, mut z_final): (f64, f64);
                let mut z: f64;
                let mut zn: f64;
                let (mut x, mut y): (isize, isize);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
                        if z != nodata {
                            sum = 0.0;
                            z_final = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + d_x[a];
                                y = row + d_y[a];
                                zn = input_fn(y, x);
                                if zn != nodata {
                                    sum += weights[a];
                                    z_final += weights[a] * zn;
                                }
                            }
                            data[col as usize] = z_final / sum;
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
        output.configs.palette = "grey.plt".to_string();
        output.update_min_max();
        output.clip_display_min_max(1.0);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma: {}", sigma));
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
