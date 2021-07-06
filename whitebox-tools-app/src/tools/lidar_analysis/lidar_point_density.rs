/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/07/2017
Last Modified: 19/05/2020
License: MIT

NOTES:
1. This tool is designed to work either by specifying a single input and output file or
    a working directory containing multiple input LAS files.
2. Need to add the ability to exclude points based on max scan angle divation.
*/

use whitebox_lidar::*;
use whitebox_raster::*;
use whitebox_common::structures::{BoundingBox, DistanceMetric, FixedRadiusSearch2D, Point3D};
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct LidarPointDensity {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarPointDensity {
    pub fn new() -> LidarPointDensity {
        // public constructor
        let name = "LidarPointDensity".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Calculates the spatial pattern of point density for a LiDAR data set. When the input/output parameters are not specified, the tool grids all LAS files contained within the working directory."
            .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file (including extension).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file (including extension).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Point Returns Included".to_owned(),
            flags: vec!["--returns".to_owned()],
            description:
                "Point return types to include; options are 'all' (default), 'last', 'first'."
                    .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "all".to_owned(),
                "last".to_owned(),
                "first".to_owned(),
            ]),
            default_value: Some("all".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Search Radius".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search radius.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Exclusion Classes (0-18, based on LAS spec; e.g. 3,4,5,6,7)".to_owned(), 
            flags: vec!["--exclude_cls".to_owned()], 
            description: "Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true
        });

        // parameters.push(ToolParameter{
        //     name: "Palette Name (Whitebox raster outputs only)".to_owned(),
        //     flags: vec!["--palette".to_owned()],
        //     description: "Optional palette name (for use with Whitebox raster files).".to_owned(),
        //     parameter_type: ParameterType::String,
        //     default_value: None,
        //     optional: true
        // });

        parameters.push(ToolParameter {
            name: "Minimum Elevation Value (optional)".to_owned(),
            flags: vec!["--minz".to_owned()],
            description: "Optional minimum elevation for inclusion in interpolation.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Elevation Value (optional)".to_owned(),
            flags: vec!["--maxz".to_owned()],
            description: "Optional maximum elevation for inclusion in interpolation.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0 --radius=5.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=5.0 --radius=2.0 --exclude_cls='3,4,5,6,7,18' --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        LidarPointDensity {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarPointDensity {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut return_type = "all".to_string();
        let mut grid_res: f64 = 1.0;
        let mut search_radius = 2.5f64;
        let mut include_class_vals = vec![true; 256];
        let mut palette = "default".to_string();
        let mut exclude_cls_str = String::new();
        let mut max_z = f64::INFINITY;
        let mut min_z = f64::NEG_INFINITY;

        // read the arguments
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
            } else if flag_val == "-returns" {
                if keyval {
                    return_type = vec[1].to_string();
                } else {
                    return_type = args[i + 1].to_string();
                }
            } else if flag_val == "-resolution" {
                if keyval {
                    grid_res = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    grid_res = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-radius" {
                if keyval {
                    search_radius = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    search_radius = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-palette" {
                if keyval {
                    palette = vec[1].to_string();
                } else {
                    palette = args[i + 1].to_string();
                }
            } else if flag_val == "-exclude_cls" {
                if keyval {
                    exclude_cls_str = vec[1].to_string();
                } else {
                    exclude_cls_str = args[i + 1].to_string();
                }
                let mut cmd = exclude_cls_str.split(",");
                let mut vec = cmd.collect::<Vec<&str>>();
                if vec.len() == 1 {
                    cmd = exclude_cls_str.split(";");
                    vec = cmd.collect::<Vec<&str>>();
                }
                for value in vec {
                    if !value.trim().is_empty() {
                        let c = value.trim().parse::<usize>().unwrap();
                        include_class_vals[c] = false;
                    }
                }
            } else if flag_val == "-minz" {
                if keyval {
                    min_z = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    min_z = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-maxz" {
                if keyval {
                    max_z = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    max_z = args[i + 1]
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

        let start = Instant::now();

        let (all_returns, late_returns, early_returns): (bool, bool, bool);
        if return_type.contains("last") {
            all_returns = false;
            late_returns = true;
            early_returns = false;
        } else if return_type.contains("first") {
            all_returns = false;
            late_returns = false;
            early_returns = true;
        } else {
            // all
            all_returns = true;
            late_returns = false;
            early_returns = false;
        }

        let mut inputs = vec![];
        let mut outputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            // match fs::read_dir(working_directory) {
            //     Err(why) => println!("! {:?}", why.kind()),
            //     Ok(paths) => {
            //         for path in paths {
            //             let s = format!("{:?}", path.unwrap().path());
            //             if s.replace("\"", "").to_lowercase().ends_with(".las") {
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 outputs.push(
            //                     inputs[inputs.len() - 1]
            //                         .replace(".las", ".tif")
            //                         .replace(".LAS", ".tif"),
            //                 )
            //             } else if s.replace("\"", "").to_lowercase().ends_with(".zip") {
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 outputs.push(
            //                     inputs[inputs.len() - 1]
            //                         .replace(".zip", ".tif")
            //                         .replace(".ZIP", ".tif"),
            //                 )
            //             }
            //         }
            //     }
            // }
            if std::path::Path::new(&working_directory).is_dir() {
                for entry in fs::read_dir(working_directory.clone())? {
                    let s = entry?
                        .path()
                        .into_os_string()
                        .to_str()
                        .expect("Error reading path string")
                        .to_string();
                    if s.to_lowercase().ends_with(".las") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".las", ".tif")
                                .replace(".LAS", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zlidar", ".tif")
                                .replace(".ZLIDAR", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zip", ".tif")
                                .replace(".ZIP", ".tif"),
                        )
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("The input directory ({}) is incorrect.", working_directory),
                ));
            }
        } else {
            if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
                input_file = format!("{}{}", working_directory, input_file);
            }
            inputs.push(input_file.clone());
            if output_file.is_empty() {
                output_file = input_file
                    .clone()
                    .replace(".las", ".tif")
                    .replace(".LAS", ".tif")
                    .replace(".zlidar", ".tif");
            }
            if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }
            outputs.push(output_file);
        }

        /*
        If multiple files are being interpolated, we will need to know their bounding boxes,
        in order to retrieve points from adjacent tiles. This is so that there are no edge
        effects.
        */
        let mut bounding_boxes = vec![];
        for in_file in &inputs {
            let header = LasHeader::read_las_header(&in_file.replace("\"", ""))?;
            bounding_boxes.push(BoundingBox {
                min_x: header.min_x,
                max_x: header.max_x,
                min_y: header.min_y,
                max_y: header.max_y,
            });
        }

        if verbose {
            println!("Performing analysis...");
        }

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let inputs = Arc::new(inputs);
        let outputs = Arc::new(outputs);
        let bounding_boxes = Arc::new(bounding_boxes);
        let num_procs2 = num_cpus::get() as isize;
        let (tx2, rx2) = mpsc::channel();
        for _ in 0..num_procs2 {
            let inputs = inputs.clone();
            let outputs = outputs.clone();
            let bounding_boxes = bounding_boxes.clone();
            let tile_list = tile_list.clone();
            // copy over the string parameters
            let palette = palette.clone();
            let return_type = return_type.clone();
            let tool_name = self.get_tool_name();
            let exclude_cls_str = exclude_cls_str.clone();
            let include_class_vals = include_class_vals.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let mut tile = 0;
                while tile < num_tiles {
                    // Get the next tile up for interpolation
                    {
                        tile = match tile_list.lock().unwrap().next() {
                            Some(val) => val,
                            None => break, // There are no more tiles to interpolate
                        };
                    }
                    let start_run = Instant::now();

                    let input_file = inputs[tile].replace("\"", "").clone();
                    let output_file = outputs[tile].replace("\"", "").clone();

                    // Expand the bounding box to include the areas of overlap
                    let bb = BoundingBox {
                        min_x: bounding_boxes[tile].min_x - search_radius as f64,
                        max_x: bounding_boxes[tile].max_x + search_radius as f64,
                        min_y: bounding_boxes[tile].min_y - search_radius as f64,
                        max_y: bounding_boxes[tile].max_y + search_radius as f64,
                    };

                    let mut frs: FixedRadiusSearch2D<u8> =
                        FixedRadiusSearch2D::new(search_radius, DistanceMetric::SquaredEuclidean);

                    if verbose && inputs.len() == 1 {
                        println!("Reading input LAS file...");
                    }

                    let mut progress: i32;
                    let mut old_progress: i32 = -1;

                    for m in 0..inputs.len() {
                        if bounding_boxes[m].overlaps(bb) {
                            let input =
                                match LasFile::new(&inputs[m].replace("\"", "").clone(), "r") {
                                    Ok(lf) => lf,
                                    Err(err) => panic!(
                                        "Error reading file {}: {}",
                                        inputs[m].replace("\"", ""),
                                        err
                                    ),
                                };

                            let n_points = input.header.number_of_points as usize;
                            let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only
                            let mut p: Point3D;
                            let mut pd: PointData;
                            for i in 0..n_points {
                                p = input.get_transformed_coords(i);
                                pd = input[i];
                                if !pd.withheld() {
                                    if all_returns
                                        || (pd.is_late_return() & late_returns)
                                        || (pd.is_early_return() & early_returns)
                                    {
                                        if include_class_vals[pd.classification() as usize] {
                                            if bb.is_point_in_box(p.x, p.y)
                                                && p.z >= min_z
                                                && p.z <= max_z
                                            {
                                                frs.insert(p.x, p.y, 1u8);
                                            }
                                        }
                                    }
                                }
                                if verbose && inputs.len() == 1 {
                                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                                    if progress != old_progress {
                                        println!("Binning points: {}%", progress);
                                        old_progress = progress;
                                    }
                                }
                            }
                        }
                    }

                    let west: f64 = bounding_boxes[tile].min_x;
                    let north: f64 = bounding_boxes[tile].max_y;
                    let rows: isize =
                        (((north - bounding_boxes[tile].min_y) / grid_res).ceil()) as isize;
                    let columns: isize =
                        (((bounding_boxes[tile].max_x - west) / grid_res).ceil()) as isize;
                    let south: f64 = north - rows as f64 * grid_res;
                    let east = west + columns as f64 * grid_res;
                    let nodata = -32768.0f64;

                    let mut configs = RasterConfigs {
                        ..Default::default()
                    };
                    configs.rows = rows as usize;
                    configs.columns = columns as usize;
                    configs.north = north;
                    configs.south = south;
                    configs.east = east;
                    configs.west = west;
                    configs.resolution_x = grid_res;
                    configs.resolution_y = grid_res;
                    configs.nodata = nodata;
                    configs.data_type = DataType::F64;
                    configs.photometric_interp = PhotometricInterpretation::Continuous;
                    configs.palette = palette.clone();

                    let mut output = Raster::initialize_using_config(&output_file, &configs);

                    let search_area = f64::consts::PI * (search_radius * search_radius) as f64;

                    if num_tiles > 1 {
                        let (mut x, mut y): (f64, f64);
                        for row in 0..rows {
                            for col in 0..columns {
                                x = west + col as f64 * grid_res + 0.5;
                                y = north - row as f64 * grid_res - 0.5;
                                let ret = frs.search(x, y);
                                output.set_value(row, col, ret.len() as f64 / search_area);
                            }
                            if verbose && inputs.len() == 1 {
                                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                                if progress != old_progress {
                                    println!("Progress: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }
                    } else {
                        // there's only one tile, so use all cores to interpolate this one tile.
                        let frs = Arc::new(frs); // wrap FRS in an Arc
                        let mut num_procs = num_cpus::get() as isize;
                        let configs = whitebox_common::configs::get_configs().unwrap();
                        let max_procs = configs.max_procs;
                        if max_procs > 0 && max_procs < num_procs {
                            num_procs = max_procs;
                        }
                        let (tx, rx) = mpsc::channel();
                        for tid in 0..num_procs {
                            let frs = frs.clone();
                            let tx1 = tx.clone();
                            thread::spawn(move || {
                                let (mut x, mut y): (f64, f64);
                                for row in (0..rows).filter(|r| r % num_procs == tid) {
                                    let mut data = vec![nodata; columns as usize];
                                    for col in 0..columns {
                                        x = west + col as f64 * grid_res + 0.5;
                                        y = north - row as f64 * grid_res - 0.5;
                                        let ret = frs.search(x, y);
                                        data[col as usize] = ret.len() as f64 / search_area;
                                    }
                                    tx1.send((row, data)).unwrap();
                                }
                            });
                        }

                        for row in 0..rows {
                            let data = rx.recv().expect("Error receiving data from thread.");
                            output.set_row_data(data.0, data.1);
                            if verbose {
                                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                                if progress != old_progress {
                                    println!("Progress: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }
                    }

                    let elapsed_time_run = get_formatted_elapsed_time(start_run);

                    output.add_metadata_entry(format!(
                        "Created by whitebox_tools\' {} tool",
                        tool_name
                    ));
                    output.add_metadata_entry(format!("Input file: {}", input_file));
                    output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
                    output.add_metadata_entry(format!("Search radius: {}", search_radius));
                    output.add_metadata_entry(format!("Returns: {}", return_type));
                    output.add_metadata_entry(format!("Excluded classes: {}", exclude_cls_str));
                    output.add_metadata_entry(format!(
                        "Elapsed Time (including I/O): {}",
                        elapsed_time_run
                    ));

                    if verbose && inputs.len() == 1 {
                        println!("Saving data...")
                    };

                    let _ = output.write().unwrap();

                    tx2.send(tile).unwrap();
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..inputs.len() {
            let tile_completed = rx2.recv().unwrap();
            if verbose {
                println!(
                    "Finished interpolating {} ({} of {})",
                    inputs[tile_completed]
                        .replace("\"", "")
                        .replace(working_directory, "")
                        .replace(".las", ""),
                    tile + 1,
                    inputs.len()
                );
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (inputs.len() - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
