/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/06/2017
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

/// This tool can be used to perform an edge-preserving smoothing filter, or bilateral filter, on an image. A bilateral 
/// filter can be used to emphasize the longer-range variability in an image, effectively acting to smooth the image, 
/// while reducing the edge blurring effect common with other types of smoothing filters. As such, this filter is very 
/// useful for reducing the noise in an image. Bilateral filtering is a non-linear filtering technique introduced by 
/// Tomasi and Manduchi (1998). The algorithm operates by convolving a kernel of weights with each grid cell and its 
/// neighbours in an image. The bilateral filter is related to Gaussian smoothing, in that the weights of the convolution 
/// kernel are partly determined by the 2-dimensional Gaussian (i.e. normal) curve, which gives stronger weighting to 
/// cells nearer the kernel centre. Unlike the `GaussianFilter`, however, the bilateral kernel weightings are also 
/// affected by their similarity to the intensity value of the central pixel. Pixels that are very different in intensity 
/// from the central pixel are weighted less, also based on a Gaussian weight distribution. Therefore, this non-linear 
/// convolution filter is determined by the spatial and intensity domains of a localized pixel neighborhood.
///
/// The heavier weighting given to nearer and similar-valued pixels makes the bilateral filter an attractive alternative 
/// for image smoothing and noise reduction compared to the much-used Mean filter. The size of the filter is determined 
/// by setting the standard deviation distance parameter (`--sigma_dist`); the larger the standard deviation the larger 
/// the resulting filter kernel. The standard deviation can be any number in the range 0.5-20 and is specified in the 
/// unit of pixels. The standard deviation intensity parameter (`--sigma_int`), specified in the same units as the z-values, 
/// determines the intensity domain contribution to kernel weightings.
/// 
/// # References
/// Tomasi, C., & Manduchi, R. (1998, January). Bilateral filtering for gray and color images. In null (p. 839). IEEE.
/// 
/// # See Also
/// `EdgePreservingMeanFilter`
pub struct BilateralFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BilateralFilter {
    pub fn new() -> BilateralFilter {
        // public constructor
        let name = "BilateralFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "A bilateral filter is an edge-preserving smoothing filter introduced by Tomasi and Manduchi (1998).".to_string();

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
            name: "Distance Standard Deviation (pixels)".to_owned(),
            flags: vec!["--sigma_dist".to_owned()],
            description: "Standard deviation in distance in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.75".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Intensity Standard Deviation (intensity units)".to_owned(),
            flags: vec!["--sigma_int".to_owned()],
            description: "Standard deviation in intensity in pixels.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --sigma_dist=2.5 --sigma_int=4.0", short_exe, name).replace("*", &sep);

        BilateralFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BilateralFilter {
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
        let mut sigma_dist = 0.75;
        let mut sigma_int = 1.0;
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
            } else if vec[0].to_lowercase() == "-sigma_dist"
                || vec[0].to_lowercase() == "--sigma_dist"
            {
                if keyval {
                    sigma_dist = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma_dist = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-sigma_int"
                || vec[0].to_lowercase() == "--sigma_int"
            {
                if keyval {
                    sigma_int = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    sigma_int = args[i + 1].to_string().parse::<f64>().unwrap();
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

        if sigma_dist < 0.5 {
            sigma_dist = 0.5;
        } else if sigma_dist > 20.0 {
            sigma_dist = 20.0;
        }

        if sigma_int < 0.001 {
            sigma_int = 0.001;
        }

        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma_dist);
        let two_sigma_sqr_d = 2.0 * sigma_dist * sigma_dist;

        let recip_root_2_pi_times_sigma_i = 1.0 / ((2.0 * PI).sqrt() * sigma_int);
        let two_sigma_sqr_i = 2.0 * sigma_int * sigma_int;

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
        let mut dx = vec![0isize; num_pixels_in_filter];
        let mut dy = vec![0isize; num_pixels_in_filter];
        let mut weights_d = vec![0.0; num_pixels_in_filter];

        // fill the filter d_x and d_y values and the distance-weights
        let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        let mut a = 0;
        let (mut x, mut y): (isize, isize);
        for row in 0..filter_size {
            for col in 0..filter_size {
                x = col as isize - midpoint;
                y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                weight = recip_root_2_pi_times_sigma_d
                    * (-1.0 * ((x * x + y * y) as f64) / two_sigma_sqr_d).exp();
                weights_d[a] = weight;
                a += 1;
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let dx = Arc::new(dx);
        let dy = Arc::new(dy);
        let weights_d = Arc::new(weights_d);

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

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let dx = dx.clone();
            let dy = dy.clone();
            let weights_d = weights_d.clone();
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
                let mut weight: f64;
                let mut weights_i = vec![0.0; num_pixels_in_filter];

                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input_fn(row, col);
                        if z != nodata {
                            //fill weights_i with the appropriate intensity weights
                            sum = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_fn(y, x);
                                if zn != nodata {
                                    weight = recip_root_2_pi_times_sigma_i
                                        * (-1.0 * ((zn - z) * (zn - z)) / two_sigma_sqr_i).exp();
                                    weight *= weights_d[a];
                                    weights_i[a] = weight;
                                    sum += weight;
                                }
                            }

                            z_final = 0.0;
                            for a in 0..num_pixels_in_filter {
                                x = col + dx[a];
                                y = row + dy[a];
                                zn = input_fn(y, x);
                                if zn != nodata {
                                    z_final += weights_i[a] * zn / sum;
                                }
                            }

                            data[col as usize] = output_fn(row, col, z_final);
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
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Sigma distance: {}", sigma_dist));
        output.add_metadata_entry(format!("Sigma intensity: {}", sigma_int));
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
