/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/06/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
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

/// This tool can be used to perform a Gaussian filter on a raster image. A Gaussian filter
/// can be used to emphasize the longer-range variability in an image, effectively acting to
/// smooth the image. This can be useful for reducing the noise in an image. The algorithm
/// operates by convolving a kernel of weights with each grid cell and its neighbours in an
/// image. The weights of the convolution kernel are determined by the 2-dimensional Gaussian
/// (i.e. normal) curve, which gives stronger weighting to cells nearer the kernel centre. It
/// is this characteristic that makes the Gaussian filter an attractive alternative for image
/// smoothing and noise reduction than the `MeanFilter`. The size of the filter is determined
/// by setting the standard deviation parameter (`--sigma`), which is in units of grid cells;
/// the larger the standard deviation the larger the resulting filter kernel. The standard
/// deviation can be any number in the range 0.5-20.
///
/// `GaussianFilter` works with both greyscale and red-green-blue (RGB) colour images. RGB images are
/// decomposed into intensity-hue-saturation (IHS) and the filter is applied to the intensity
/// channel. NoData values in the input image are ignored during processing.
///
/// Like many low-pass filters, Gaussian filtering can significantly blur well-defined edges in
/// the input image. The `EdgePreservingMeanFilter` and `BilateralFilter` offer more robust
/// feature preservation during image smoothing. `GaussianFilter` is relatively slow compared to
/// the `FastAlmostGaussianFilter` tool, which offers a fast-running approximatation to a
/// Gaussian filter for larger kernel sizes.
///
/// # See Also
/// `FastAlmostGaussianFilter`, `MeanFilter`, `MedianFilter`, `RgbToIhs`
pub struct GaussianFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl GaussianFilter {
    pub fn new() -> GaussianFilter {
        // public constructor
        let name = "GaussianFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a Gaussian filter on an image.".to_string();

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
            name: "Standard Deviation (pixels)".to_owned(),
            flags: vec!["--sigma".to_owned()],
            description: "Standard deviation distance in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.75".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --sigma=2.0",
            short_exe, name
        )
        .replace("*", &sep);

        GaussianFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for GaussianFilter {
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
        let mut sigma_d = 0.75;
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
            } else if flag_val == "-sigma" {
                if keyval {
                    sigma_d = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    sigma_d = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if sigma_d < 0.5 {
            sigma_d = 0.5;
        } else if sigma_d > 20.0 {
            sigma_d = 20.0;
        }

        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma_d);
        let two_sigma_sqr_d = 2.0 * sigma_d * sigma_d;

        // figure out the size of the filter
        let mut weight: f64;
        for i in 0..250 {
            weight =
                recip_root_2_pi_times_sigma_d * (-1.0 * ((i * i) as f64) / two_sigma_sqr_d).exp();
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
        let mut sum = 0f64;

        // fill the filter d_x and d_y values and the distance-weights
        let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size {
            for col in 0..filter_size {
                x = col as isize - midpoint;
                y = row as isize - midpoint;
                d_x[a] = x;
                d_y[a] = y;
                weight = recip_root_2_pi_times_sigma_d
                    * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                weights[a] = weight;
                sum += weight;
                a += 1;
            }
        }

        for a in 0..num_pixels_in_filter {
            weights[a] /= sum;
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

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut output = Raster::initialize_using_file(&output_file, &input);

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
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

                let output_fn: Box<dyn Fn(isize, isize, f64) -> f64> = if !is_rgb_image {
                    // simply return the value.
                    Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                } else {
                    // convert it back into an rgb value, using the modified intensity value.
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            return hsi2value(h, s, value);
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
                            data[col as usize] = output_fn(row, col, z_final / sum);
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
        output.add_metadata_entry(format!("Sigma: {}", sigma_d));
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

fn value2hsi(value: f64) -> (f64, f64, f64) {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    let i = (r + g + b) / 3f64;

    let rn = r / (r + g + b);
    let gn = g / (r + g + b);
    let bn = b / (r + g + b);

    let mut h = if rn != gn || rn != bn {
        ((0.5 * ((rn - gn) + (rn - bn))) / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt())
            .acos()
    } else {
        0f64
    };
    if b > g {
        h = 2f64 * PI - h;
    }

    let s = 1f64 - 3f64 * rn.min(gn).min(bn);

    (h, s, i)
}

fn hsi2value(h: f64, s: f64, i: f64) -> f64 {
    let mut r: u32;
    let mut g: u32;
    let mut b: u32;

    let x = i * (1f64 - s);

    if h < 2f64 * PI / 3f64 {
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (y * 255f64).round() as u32;
        g = (z * 255f64).round() as u32;
        b = (x * 255f64).round() as u32;
    } else if h < 4f64 * PI / 3f64 {
        let h = h - 2f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (x * 255f64).round() as u32;
        g = (y * 255f64).round() as u32;
        b = (z * 255f64).round() as u32;
    } else {
        let h = h - 4f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (z * 255f64).round() as u32;
        g = (x * 255f64).round() as u32;
        b = (y * 255f64).round() as u32;
    }

    if r > 255u32 {
        r = 255u32;
    }
    if g > 255u32 {
        g = 255u32;
    }
    if b > 255u32 {
        b = 255u32;
    }

    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}
