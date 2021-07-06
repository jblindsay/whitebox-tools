/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/04/2018
Last Modified: 22/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to reduce vignetting within an image. Vignetting refers to the
/// reducuction of image brightness away from the image centre (i.e. the principal point).
/// Vignetting is a radiometric distortion resulting from lens characteristics. The
/// algorithm calculates the brightness value in the output image (BVout) as:
///
/// BVout = BVin / [cos^n(arctan(d / f))]
///
/// Where d is the photo-distance from the principal point in millimetres, f is the focal
/// length of the camera, in millimeters, and n is a user-specified parameter. Pixel
/// distances are converted to photo-distances (in millimetres) using the specified
/// image width, i.e. distance between left and right edges (mm). For many cameras, 4.0
/// is an appropriate value of the n parameter. A second pass of the image is used to
/// rescale the output image so that it possesses the same minimum and maximum values as
/// the input image.
///
/// If an RGB image is input, the analysis will be performed on the intensity component
/// of the HSI transform.
pub struct CorrectVignetting {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CorrectVignetting {
    pub fn new() -> CorrectVignetting {
        // public constructor
        let name = "CorrectVignetting".to_string();
        let toolbox = "Image Processing Tools/Image Enhancement".to_string();
        let description = "Corrects the darkening of images towards corners.".to_string();

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
            name: "Input Principal Point File".to_owned(),
            flags: vec!["--pp".to_owned()],
            description: "Input principal point file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
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
            name: "Camera Focal Length (mm)".to_owned(),
            flags: vec!["--focal_length".to_owned()],
            description: "Camera focal length, in millimeters.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("304.8".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Distance Between Left-Right Edges (mm)".to_owned(),
            flags: vec!["--image_width".to_owned()],
            description: "Distance between photograph edges, in millimeters.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("228.6".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "n Parameter".to_owned(),
            flags: vec!["-n".to_owned()],
            description: "The 'n' parameter.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("4.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif --pp=princ_pt.shp -o=output.tif --focal_length=304.8 --image_width=228.6 -n=4.0", short_exe, name).replace("*", &sep);

        CorrectVignetting {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CorrectVignetting {
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
        let mut pp_file = String::new();
        let mut output_file = String::new();
        let mut focal_length = 304.8f64;
        let mut image_width = 228.6f64;
        let mut n_param = 4f64;

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
            } else if flag_val == "-pp" {
                pp_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-focal_length" {
                focal_length = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-image_width" {
                image_width = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-n" {
                n_param = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
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
        if !pp_file.contains(&sep) && !pp_file.contains("/") {
            pp_file = format!("{}{}", working_directory, pp_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let scale_factor = image_width / columns as f64;

        let is_rgb_image = if input.configs.data_type == DataType::RGB24
            || input.configs.data_type == DataType::RGBA32
            || input.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        if input.configs.data_type == DataType::RGB48 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "This tool cannot be applied to 48-bit RGB colour-composite images.",
            ));
        }

        let vector_data = Shapefile::read(&pp_file)?;

        // make sure the input vector file is of point base type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of a point base shape type.",
            ));
        }

        let start = Instant::now();

        // get the row/column of the principal point
        let pp_x = input.get_column_from_x(vector_data.get_record(0).points[0].x) as f64;
        let pp_y = input.get_row_from_y(vector_data.get_record(0).points[0].y) as f64;

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
                let mut z_in: f64;
                let mut z_out: f64;
                let mut dist: f64;
                let mut theta: f64;
                let mut min_in = f64::INFINITY;
                let mut max_in = f64::NEG_INFINITY;
                let mut min_out = f64::INFINITY;
                let mut max_out = f64::NEG_INFINITY;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = input_fn(row, col);
                        if z_in != nodata {
                            dist = ((row as f64 - pp_y) * (row as f64 - pp_y)
                                + (col as f64 - pp_x) * (col as f64 - pp_x))
                                .sqrt();
                            theta = (dist * scale_factor / focal_length).atan();
                            z_out = z_in / theta.cos().powf(n_param);
                            data[col as usize] = z_out;
                            if z_in < min_in {
                                min_in = z_in;
                            }
                            if z_in > max_in {
                                max_in = z_in;
                            }
                            if z_out < min_out {
                                min_out = z_out;
                            }
                            if z_out > max_out {
                                max_out = z_out;
                            }
                        }
                    }
                    tx.send((row, data, min_in, max_in, min_out, max_out))
                        .unwrap();
                }
            });
        }

        let mut unscaled_data: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        let mut min_in = f64::INFINITY;
        let mut max_in = f64::NEG_INFINITY;
        let mut min_out = f64::INFINITY;
        let mut max_out = f64::NEG_INFINITY;
        for r in 0..rows {
            let (row, data, a, b, c, d) = rx.recv().expect("Error receiving data from thread.");
            if a < min_in {
                min_in = a;
            }
            if b > max_in {
                max_in = b;
            }
            if c < min_out {
                min_out = c;
            }
            if d > max_out {
                max_out = d;
            }
            unscaled_data.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }
        let range_in = max_in - min_in;
        let range_out = max_out - min_out;

        let unscaled_data = Arc::new(unscaled_data);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let unscaled_data = unscaled_data.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let output_fn: Box<dyn Fn(isize, isize, f64) -> f64> = if !is_rgb_image {
                    // simply return the value.
                    Box::new(|_: isize, _: isize, value: f64| -> f64 { value })
                } else {
                    // convert it back into an rgb value, using the modified intensity value.
                    Box::new(|row: isize, col: isize, value: f64| -> f64 {
                        if value != nodata {
                            let (h, s, _) = value2hsi(input.get_value(row, col));
                            let ret = hsi2value(h, s, value);
                            return ret;
                        }
                        nodata
                    })
                };
                let mut z_in: f64;
                let mut z_out: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z_in = unscaled_data.get_value(row, col);
                        if z_in != nodata {
                            z_out = min_in + (z_in - min_out) / range_out * range_in;
                            data[col as usize] = output_fn(row, col, z_out);
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        if is_rgb_image {
            output.configs.photometric_interp = PhotometricInterpretation::RGB;
            output.configs.data_type = DataType::RGBA32;
        }
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
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
        output.add_metadata_entry(format!("PP file: {}", pp_file));
        output.add_metadata_entry(format!("Focal length: {}", focal_length));
        output.add_metadata_entry(format!("Image width: {}", image_width));
        output.add_metadata_entry(format!("n-parameter: {}", n_param));
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
