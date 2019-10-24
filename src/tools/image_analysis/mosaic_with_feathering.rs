/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/12/2018
Last Modified: 02/01/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::structures::Array2D;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool will create a mosaic from two input images. It is similar in operation to the `Mosaic` tool, 
/// however, this tool is the preferred method of mosaicing images when there is significant overlap between 
/// the images. For areas of overlap, the feathering method will calculate the output value as a weighted 
/// combination of the two input values, where the weights are derived from the squared distance of the 
/// pixel to the edge of the data in each of the input raster files. Therefore, less weight is assigned to 
/// an image's pixel value where the pixel is very near the edge of the image. Note that the distance is 
/// actually calculated to the edge of the grid and not necessarily the edge of the data, which can differ
/// if the image has been rotated during registration.  The result of this feathering method is that the 
/// output mosaic image should have very little evidence of the original image edges within the overlapping 
/// area. 
/// 
/// Unlike the Mosaic tool, which can take multiple input images, this tool only accepts two input images. 
/// Mosaic is therefore useful when there are many, adjacent or only slightly overlapping images, e.g. for 
/// tiled data sets.
/// 
/// Users may want to use the `HistogramMatching` tool prior to mosaicing if the two input images differ 
/// significantly in their radiometric properties. i.e. if image contrast differences exist.
/// 
/// # See Also
/// `Mosaic`, `HistogramMatching`
pub struct MosaicWithFeathering {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MosaicWithFeathering {
    pub fn new() -> MosaicWithFeathering {
        // public constructor
        let name = "MosaicWithFeathering".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Mosaics two images together using a feathering technique in overlapping areas to reduce edge-effects.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File To Modify".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input raster file to modify.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Reference File".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input reference raster file.".to_owned(),
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
            name: "Resampling Method".to_owned(), 
            flags: vec!["--method".to_owned()], 
            description: "Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution)".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["nn".to_owned(), "bilinear".to_owned(), "cc".to_owned()]),
            default_value: Some("cc".to_owned()),
            optional: true
        });

        // parameters.push(ToolParameter {
        //     name: "Perform histogram matching?".to_owned(),
        //     flags: vec!["--histo_match".to_owned()],
        //     description:
        //         "Optional flag indicating whether a histogram-matching contrast enhancement is performed."
        //             .to_owned(),
        //     parameter_type: ParameterType::Boolean,
        //     default_value: Some("true".to_owned()),
        //     optional: true,
        // });

        parameters.push(ToolParameter {
            name: "Distance Weight".to_owned(),
            flags: vec!["--weight".to_owned()],
            description: "".to_owned(),
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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' --input1='image1.tif' --input2='image2.tif' -o='output.tif' --method='cc' --weight=4.0", short_exe, name).replace("*", &sep);

        MosaicWithFeathering {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MosaicWithFeathering {
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
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
        let mut method = String::from("cc");
        let mut distance_weight = 4.0;
        // let mut histo_match = false;

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
            if flag_val == "-i1" || flag_val == "-input1" {
                input_file1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-i2" || flag_val == "-input2" {
                input_file2 = if keyval {
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
            // } else if flag_val == "-histo_match" {
                // if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
            //     histo_match = true;
            //    }
            } else if flag_val == "-weight" {
                distance_weight = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
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

        if !input_file1.contains(&sep) && !input_file1.contains("/") {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) && !input_file2.contains("/") {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        // read the input files
        if verbose {
            println!("Reading data...")
        };

        let input1 = Arc::new(Raster::new(&input_file1, "r")?);
        let input2 = Arc::new(Raster::new(&input_file2, "r")?);

        let rows1 = input1.configs.rows as isize;
        let columns1 = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;

        let rows2 = input2.configs.rows as isize;
        let columns2 = input2.configs.columns as isize;
        let nodata2 = input2.configs.nodata;

        if input1.configs.data_type != input2.configs.data_type {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input images do not share the same data type."));
        }

        let rgb_mode = if input1.configs.data_type == DataType::RGB24
            || input1.configs.data_type == DataType::RGB48
            || input1.configs.data_type == DataType::RGBA32
            || input1.configs.photometric_interp == PhotometricInterpretation::RGB
        {
            true
        } else {
            false
        };

        // what are the dimensions of the combined bounding boxes of the two input rasters?
        let mut extent = input1.get_bounding_box();
        extent.expand_to(input2.get_bounding_box());

        // the output image should have the coarser of the input resolutions
        let resolution_x = input1.configs.resolution_x.max(input2.configs.resolution_x);
        let resolution_y = input1.configs.resolution_y.max(input2.configs.resolution_y);

        // create the output image
        let rows = (extent.get_height() / resolution_y).ceil() as isize;
        let columns = (extent.get_width() / resolution_x).ceil() as isize;
        let south: f64 = extent.max_y - rows as f64 * resolution_y;
        let east = extent.min_x + columns as f64 * resolution_x;
        
        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = extent.max_y;
        configs.south = south;
        configs.east = east;
        configs.west = extent.min_x;
        configs.resolution_x = resolution_x;
        configs.resolution_y = resolution_y;
        configs.nodata = nodata1;
        configs.data_type = input1.configs.data_type;
        configs.photometric_interp = input1.configs.photometric_interp;
        configs.palette = input1.configs.palette.clone();

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();

        // create the minimum edge distance rasters
        for tid in 0..num_procs {
            let tx = tx.clone();
            thread::spawn(move || {
                for row in (0..rows1).filter(|r| r % num_procs == tid) {
                    let mut data = vec![0u32; columns1 as usize];
                    for col in 0..columns1 {
                        data[col as usize] = col.min(row.min((columns1 - col - 1).min(rows1 - row - 1))) as u32;
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }
        let mut dist1_raster: Array2D<u32> = Array2D::new(rows1, columns1, u32::max_value(), u32::max_value())?;
        for row in 0..rows1 {   
            let data = rx.recv().unwrap();
            dist1_raster.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating distances (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for tid in 0..num_procs {
            let tx = tx.clone();
            thread::spawn(move || {
                for row in (0..rows2).filter(|r| r % num_procs == tid) {
                    let mut data = vec![0u32; columns2 as usize];
                    for col in 0..columns2 {
                        data[col as usize] = col.min(row.min((columns2 - col - 1).min(rows2 - row - 1))) as u32;
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }
        let mut dist2_raster: Array2D<u32> = Array2D::new(rows2, columns2, u32::max_value(), u32::max_value())?;
        for row in 0..rows2 {
            let data = rx.recv().unwrap();
            dist2_raster.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating distances (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

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
        let dist1_raster = Arc::new(dist1_raster);
        let dist2_raster = Arc::new(dist2_raster);
        let (tx, rx) = mpsc::channel();
        if method == "nn" {
            for tid in 0..num_procs {
                let input1 = input1.clone();
                let input2 = input2.clone();
                let x = x.clone();
                let y = y.clone();
                let dist1_raster = dist1_raster.clone();
                let dist2_raster = dist2_raster.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let (mut col_src1, mut row_src1): (isize, isize);
                    let (mut col_src2, mut row_src2): (isize, isize);
                    let (mut z1, mut z2): (f64, f64);
                    let (mut dist1, mut dist2): (f64, f64);
                    let (mut w1, mut w2): (f64, f64);
                    let mut sum_dist: f64;
                    let mut val: u32;
                    let (mut red1, mut green1, mut blue1): (f64, f64, f64);
                    let (mut red2, mut green2, mut blue2): (f64, f64, f64);
                    let (mut red, mut green, mut blue): (u32, u32, u32);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            row_src1 = input1.get_row_from_y(y[row as usize]);
                            col_src1 = input1.get_column_from_x(x[col as usize]);
                            z1 = input1.get_value(row_src1, col_src1);

                            row_src2 = input2.get_row_from_y(y[row as usize]);
                            col_src2 = input2.get_column_from_x(x[col as usize]);
                            z2 = input2.get_value(row_src2, col_src2);
                            if z1 != nodata1 && z2 != nodata2 {
                                dist1 = dist1_raster.get_value(row_src1, col_src1) as f64;
                                dist2 = dist2_raster.get_value(row_src2, col_src2) as f64;
                                sum_dist = dist1.powf(distance_weight) + dist2.powf(distance_weight);
                                w1 = dist1.powf(distance_weight) / sum_dist;
                                w2 = dist2.powf(distance_weight) / sum_dist;
                                if !rgb_mode {
                                    data[col as usize] = z1 * w1 + z2 * w2;
                                } else {
                                    val = z1 as u32;
                                    red1 = (val & 0xFF) as f64;
                                    green1 = ((val >> 8) & 0xFF) as f64;
                                    blue1 = ((val >> 16) & 0xFF) as f64;

                                    val = z2 as u32;
                                    red2 = (val & 0xFF) as f64;
                                    green2 = ((val >> 8) & 0xFF) as f64;
                                    blue2 = ((val >> 16) & 0xFF) as f64;

                                    red = (red1 * w1 + red2 * w2) as u32;
                                    green = (green1 * w1 + green2 * w2) as u32;
                                    blue = (blue1 * w1 + blue2 * w2) as u32;

                                    data[col as usize] = ((255u32 << 24) | (blue << 16) | (green << 8) | red) as f64;
                                }
                            } else if z1 != nodata1 {
                                data[col as usize] = z1;
                            } else if z2 != nodata2 {
                                data[col as usize] = z2;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;

            for tid in 0..num_procs {
                let input1 = input1.clone();
                let input2 = input2.clone();
                let method = method.clone();
                let x = x.clone();
                let y = y.clone();
                let dist1_raster = dist1_raster.clone();
                let dist2_raster = dist2_raster.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let shift_x = if method == "cc" {
                        vec![-1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2]
                    } else {
                        vec![0, 1, 0, 1] // bilinear
                    };
                    let shift_y = if method == "cc" {
                        vec![-1, -1, -1, -1, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]
                    } else {
                        vec![0, 0, 1, 1] // bilinear
                    };
                    let num_neighbours = if method == "cc" { 
                        16 
                    } else {
                        4 // bilinear
                    };
                    let mut neighbour = if method == "cc" { 
                        vec![[0f64; 2]; 16]
                    } else {
                        vec![[0f64; 2]; 4] // bilinear
                    };
                    let (mut col_n, mut row_n): (isize, isize);
                    let (mut origin_row1, mut origin_col1): (isize, isize);
                    let (mut origin_row2, mut origin_col2): (isize, isize);
                    let (mut dx, mut dy): (f64, f64);
                    let (mut col_src1, mut row_src1): (f64, f64);
                    let (mut col_src2, mut row_src2): (f64, f64);
                    let (mut z1, mut z2): (f64, f64);
                    let (mut dist1, mut dist2): (f64, f64);
                    let (mut w1, mut w2): (f64, f64);
                    let mut sum_dist: f64;
                    let mut val: u32;
                    let (mut red1, mut green1, mut blue1): (f64, f64, f64);
                    let (mut red2, mut green2, mut blue2): (f64, f64, f64);
                    let (mut red, mut green, mut blue): (u32, u32, u32);
                    let (mut image1_valid, mut image2_valid): (bool, bool);
                    let large_value = 999999.0; // used to apply a large weight to points that are coincident with the cell.
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            row_src1 = (input1.configs.north - y[row as usize])
                                / input1.configs.resolution_y;
                            col_src1 = (x[col as usize] - input1.configs.west)
                                / input1.configs.resolution_x;
                            origin_row1 = row_src1.floor() as isize;
                            origin_col1 = col_src1.floor() as isize;

                            row_src2 = (input2.configs.north - y[row as usize])
                                / input2.configs.resolution_y;
                            col_src2 = (x[col as usize] - input2.configs.west)
                                / input2.configs.resolution_x;
                            origin_row2 = row_src2.floor() as isize;
                            origin_col2 = col_src2.floor() as isize;

                            if !rgb_mode {
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row1 + shift_y[n];
                                    col_n = origin_col1 + shift_x[n];
                                    neighbour[n][0] = input1.get_value(row_n, col_n);;
                                    dy = row_n as f64 - row_src1;
                                    dx = col_n as f64 - col_src1;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata1 {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata1 {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        neighbour[n][1] = large_value;
                                        sum_dist += neighbour[n][1];
                                    }
                                }
                                z1 = 0f64;
                                if sum_dist > 0f64 {
                                    for n in 0..num_neighbours {
                                        z1 += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                }

                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row2 + shift_y[n];
                                    col_n = origin_col2 + shift_x[n];
                                    neighbour[n][0] = input2.get_value(row_n, col_n);;
                                    dy = row_n as f64 - row_src2;
                                    dx = col_n as f64 - col_src2;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata2 {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata2 {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        neighbour[n][1] = large_value;
                                        sum_dist += neighbour[n][1];
                                    }
                                }
                                z2 = 0f64;
                                if sum_dist > 0f64 {
                                    for n in 0..num_neighbours {
                                        z2 += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                }

                                if z1 != 0f64 && z2 != 0f64 {
                                    dist1 = dist1_raster.get_value(origin_row1, origin_col1) as f64;
                                    dist2 = dist2_raster.get_value(origin_row2, origin_col2) as f64;
                                    sum_dist = dist1.powf(distance_weight) + dist2.powf(distance_weight);
                                    w1 = dist1.powf(distance_weight) / sum_dist;
                                    w2 = dist2.powf(distance_weight) / sum_dist;

                                    data[col as usize] = z1 * w1 + z2 * w2;
                                } else if z1 != 0f64 {
                                    data[col as usize] = z1;
                                } else if z2 != 0f64 {
                                    data[col as usize] = z2;
                                }
                            } else {
                                // it's an rgb image and each component will need to be handled seperately. 
                                sum_dist = 0f64;
                                image1_valid = false;
                                for n in 0..num_neighbours {
                                    row_n = origin_row1 + shift_y[n];
                                    col_n = origin_col1 + shift_x[n];
                                    neighbour[n][0] = input1.get_value(row_n, col_n);;
                                    dy = row_n as f64 - row_src1;
                                    dx = col_n as f64 - col_src1;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata1 {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                        image1_valid = true;
                                    } else if neighbour[n][0] == nodata1 {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        neighbour[n][1] = large_value;
                                        sum_dist += neighbour[n][1];
                                        image1_valid = true;
                                    }
                                }
                                red1 = 0f64;
                                green1 = 0f64;
                                blue1 = 0f64;
                                if sum_dist > 0f64 {
                                    for n in 0..num_neighbours {
                                        val = neighbour[n][0] as u32;
                                        red1 += ((val & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                        green1 += (((val >> 8) & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                        blue1 += (((val >> 16) & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                    }
                                }

                                sum_dist = 0f64;
                                image2_valid = false;
                                for n in 0..num_neighbours {
                                    row_n = origin_row2 + shift_y[n];
                                    col_n = origin_col2 + shift_x[n];
                                    neighbour[n][0] = input2.get_value(row_n, col_n);;
                                    dy = row_n as f64 - row_src2;
                                    dx = col_n as f64 - col_src2;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata2 {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                        image2_valid = true;
                                    } else if neighbour[n][0] == nodata2 {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        neighbour[n][1] = large_value;
                                        sum_dist += neighbour[n][1];
                                        image2_valid = true;
                                    }
                                }
                                red2 = 0f64;
                                green2 = 0f64;
                                blue2 = 0f64;
                                if sum_dist > 0f64 {
                                    for n in 0..num_neighbours {
                                        val = neighbour[n][0] as u32;
                                        red2 += ((val & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                        green2 += (((val >> 8) & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                        blue2 += (((val >> 16) & 0xFF) as f64 * neighbour[n][1]) / sum_dist;
                                    }
                                }

                                if image1_valid && image2_valid {
                                    dist1 = dist1_raster.get_value(origin_row1, origin_col1) as f64;
                                    dist2 = dist2_raster.get_value(origin_row2, origin_col2) as f64;
                                    sum_dist = dist1.powf(distance_weight) + dist2.powf(distance_weight);
                                    w1 = dist1.powf(distance_weight) / sum_dist;
                                    w2 = dist2.powf(distance_weight) / sum_dist;

                                    red = (red1 * w1 + red2 * w2) as u32;
                                    green = (green1 * w1 + green2 * w2) as u32;
                                    blue = (blue1 * w1 + blue2 * w2) as u32;

                                    data[col as usize] = ((255u32 << 24) | (blue << 16) | (green << 8) | red) as f64;
                                } else if image1_valid {
                                    red = red1 as u32;
                                    green = green1 as u32;
                                    blue = blue1 as u32;
                                    data[col as usize] = ((255u32 << 24) | (blue << 16) | (green << 8) | red) as f64;
                                } else if image2_valid {
                                    red = red2 as u32;
                                    green = green2 as u32;
                                    blue = blue2 as u32;
                                    data[col as usize] = ((255u32 << 24) | (blue << 16) | (green << 8) | red) as f64;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
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
        output.add_metadata_entry(format!("Resampling method: {}", method));

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
