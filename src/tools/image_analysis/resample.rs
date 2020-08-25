/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/01/2018
Last Modified: 25/08/2020
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

/// This tool can be used to modify the grid resolution of one or more rasters. The user 
/// specifies the names of one or more input rasters (`--inputs`) and the output raster
/// (`--output`). The resolution of the output raster is determined either using a 
/// specified `--cell_size` parameter, in which case the output extent is determined by the
/// combined extent of the inputs, or by an optional base raster (`--base`), in which case
/// the output raster spatial extent matches that of the base file. This operation is similar 
/// to the `Mosaic` tool, except that `Resample` modifies the output resolution. The `Resample` 
/// tool may also be used with a single input raster (when the user wants to modify its 
/// spatial resolution, whereas, `Mosaic` always includes multiple inputs. 
///
/// If the input source images are more extensive than the base image (if optionally specified),
/// these areas will not be represented in the output image. Grid cells in the
/// output image that are not overlapping with any of the input source images will not be
/// assigned the NoData value, which will be the same as the first input image. Grid cells in
/// the output image that overlap with multiple input raster cells will be assigned the last
/// input value in the stack. Thus, the order of input images is important.
/// 
/// # See Also
/// `Mosaic`
pub struct Resample {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Resample {
    pub fn new() -> Resample {
        // public constructor
        let name = "Resample".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description =
            "Resamples one or more input images into a destination image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input raster files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
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
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Base Raster File (optional)".to_owned(), 
            flags: vec!["--base".to_owned()], 
            description: "Optionally specified input base raster file. Not used when a cell size is specified.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Resampling Method".to_owned(), 
            flags: vec!["--method".to_owned()], 
            description: "Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution)".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["nn".to_owned(), "bilinear".to_owned(), "cc".to_owned()]),
            default_value: Some("cc".to_owned()),
            optional: true
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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' --destination=dest.tif --method='cc", short_exe, name).replace("*", &sep);

        Resample {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Resample {
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
        let mut input_files = String::new();
        let mut output_file = String::new();
        let mut base_file = String::new();
        let mut cell_size = 0f64;
        let mut cell_size_specified = false;
        let mut base_file_specified = false;
        let mut method = String::from("cc");

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
            if flag_val == "-i" || flag_val == "-inputs" {
                input_files = if keyval {
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
            } else if flag_val == "-cell_size" {
                cell_size = if keyval {
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
                if cell_size > 0f64 {
                    cell_size_specified = true;
                } else {
                    panic!("Error, when specified, the cell_size parameter must be larger than 0.0.");
                }
            } else if flag_val == "-base" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                base_file_specified = true;
            } else if flag_val == "-method" {
                method = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                if method.to_lowercase().contains("nn") || method.to_lowercase().contains("nearest")
                {
                    method = "nn".to_string();
                } else if method.to_lowercase().contains("bilinear")
                    || method.to_lowercase().contains("bi")
                {
                    method = "bilinear".to_string();
                } else if method.to_lowercase().contains("cc")
                    || method.to_lowercase().contains("cubic")
                {
                    method = "cc".to_string();
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

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // see if the destination file exists.
        if base_file_specified && !path::Path::new(&base_file).exists() {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The base raster file (--base) does not exist."));
        }

        if !base_file_specified && !cell_size_specified {
            return Err(Error::new(ErrorKind::InvalidInput,
                "Either an existing base raster (--base) or an output raster cell size (--cell_size) must be specified."));
        }

        let mut cmd = input_files.split(";");
        let mut input_vec = cmd.collect::<Vec<&str>>();
        if input_vec.len() == 1 {
            cmd = input_files.split(",");
            input_vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_vec.len();
        if num_files < 1 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect about the input files. At least one input is required to operate this tool."));
        }

        let start = Instant::now();

        // read the input files
        if verbose {
            println!("Reading data...")
        };
        let mut inputs: Vec<Raster> = Vec::with_capacity(num_files);
        let mut nodata_vals: Vec<f64> = Vec::with_capacity(num_files);
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut num_images = 0usize;
        for i in 0..num_files {
            let value = input_vec[i];
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                inputs.push(Raster::new(&input_file, "r")
                    .expect(&format!("Error reading image file {}", input_file)));
                num_images += 1;
                nodata_vals.push(inputs[i].configs.nodata);
                if inputs[num_images-1].configs.west < min_x { min_x = inputs[num_images-1].configs.west; }
                if inputs[num_images-1].configs.south < min_y { min_y = inputs[num_images-1].configs.south; }
                if inputs[num_images-1].configs.east > max_x { max_x = inputs[num_images-1].configs.east; }
                if inputs[num_images-1].configs.north > max_y { max_y = inputs[num_images-1].configs.north; }
            } else {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "There is a problem with the list of input files. At least one specified input is empty."));
            }
        }

        // Create the output raster. The process of doing this will
        // depend on whether a cell size or a base raster were specified.
        // If both are specified, the base raster takes priority.

        let mut output = if base_file_specified || cell_size <= 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let base = Raster::new(&base_file, "r")?;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            // base the output raster on the cell_size and the
            // extent of the input vector.
            let west: f64 = min_x;
            let north: f64 = max_y;
            let rows: isize = (((north - min_y) / cell_size).ceil()) as isize;
            let columns: isize = (((max_x - west) / cell_size).ceil()) as isize;
            let south: f64 = north - rows as f64 * cell_size;
            let east = west + columns as f64 * cell_size;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows as usize;
            configs.columns = columns as usize;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = cell_size;
            configs.resolution_y = cell_size;
            configs.nodata = nodata_vals[0];
            configs.data_type = inputs[0].configs.data_type;
            configs.photometric_interp = PhotometricInterpretation::Continuous;
            configs.projection = inputs[0].configs.projection.clone();

            Raster::initialize_using_config(&output_file, &configs)
        };

        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;
        let nodata = output.configs.nodata;

        // create the x and y arrays
        let mut x: Vec<f64> = Vec::with_capacity(columns as usize);
        for col in 0..columns {
            x.push(output.get_x_from_column(col));
        }

        let mut y: Vec<f64> = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            y.push(output.get_y_from_row(row));
        }

        let x = Arc::new(x);
        let y = Arc::new(y);
        let inputs = Arc::new(inputs);
        let nodata_vals = Arc::new(nodata_vals);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        if method == "nn" {
            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let (mut col_src, mut row_src): (isize, isize);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            for i in 0..num_files {
                                row_src = inputs[i].get_row_from_y(y[row as usize]);
                                col_src = inputs[i].get_column_from_x(x[col as usize]);
                                // row_src = ((inputs[i].configs.north - y[row as usize]) / inputs[i].configs.resolution_y).round() as isize;
                                // col_src = ((x[col as usize] - inputs[i].configs.west) / inputs[i].configs.resolution_x).round() as isize;
                                z = inputs[i].get_value(row_src, col_src);
                                if z != nodata_vals[i] {
                                    data[col as usize] = z;
                                    break;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns {
                    if data[col as usize] != nodata {
                        output.set_value(row, col, data[col as usize]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else if method == "cc" {
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;

            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let shift_x = [-1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2];
                    let shift_y = [-1, -1, -1, -1, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];
                    let num_neighbours = 16;
                    let mut neighbour = [[0f64; 2]; 16];
                    let (mut col_src, mut row_src): (f64, f64);
                    let (mut col_n, mut row_n): (isize, isize);
                    let (mut origin_row, mut origin_col): (isize, isize);
                    let (mut dx, mut dy): (f64, f64);
                    let mut sum_dist: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            let mut flag = true;
                            for i in 0..num_files {
                                if !flag {
                                    break;
                                }
                                // row_src = inputs[i].get_row_from_y(y[row as usize]);
                                // col_src = inputs[i].get_column_from_x(x[col as usize]);
                                row_src = (inputs[i].configs.north - y[row as usize])
                                    / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west)
                                    / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);
                                    dy = row_n as f64 - row_src;
                                    dx = col_n as f64 - col_src;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata_vals[i] {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata_vals[i] {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        data[col as usize] = neighbour[n][0];
                                        flag = false;
                                    }
                                }

                                if sum_dist > 0f64 {
                                    z = 0f64;
                                    for n in 0..num_neighbours {
                                        z += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                    data[col as usize] = z;
                                    flag = false;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        output.set_value(row, col as isize, data[col]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // bilinear
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;
            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let shift_x = [0, 1, 0, 1];
                    let shift_y = [0, 0, 1, 1];
                    let num_neighbours = 4;
                    let mut neighbour = [[0f64; 2]; 4];
                    let (mut col_src, mut row_src): (f64, f64);
                    let (mut col_n, mut row_n): (isize, isize);
                    let (mut origin_col, mut origin_row): (isize, isize);
                    let (mut dx, mut dy): (f64, f64);
                    let mut sum_dist: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            let mut flag = true;
                            for i in 0..num_files {
                                if !flag {
                                    break;
                                }
                                row_src = (inputs[i].configs.north - y[row as usize])
                                    / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west)
                                    / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);
                                    dy = row_n as f64 - row_src;
                                    dx = col_n as f64 - col_src;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata_vals[i] {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata_vals[i] {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        data[col as usize] = neighbour[n][0];
                                        flag = false;
                                    }
                                }

                                if sum_dist > 0f64 {
                                    z = 0f64;
                                    for n in 0..num_neighbours {
                                        z += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                    data[col as usize] = z;
                                    flag = false;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        output.set_value(row, col as isize, data[col]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Modified by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));

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
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
