/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/08/2018
Last Modified: 19/05/2020
License: MIT
*/

use whitebox_common::algorithms;
use whitebox_lidar::*;
use whitebox_common::structures::{BoundingBox, Point2D};
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool copies LiDAR tiles overlapping with a polygon into an output directory. In actuality, the tool performs
/// point-in-polygon operations, using the four corner points, the center point, and the four mid-edge points of each
/// LiDAR tile bounding box and the polygons. This representation of overlapping geometry aids with performance. This
/// approach generally works well when the polygon size is large relative to the LiDAR tiles. If, however, the input
/// polygon is small relative to the tile size, this approach may miss some copying some tiles. It is advisable to
/// buffer the polygon if this occurs.
///
/// **A note on LAZ file inputs:** While WhiteboxTools does not currently support the reading and writing of the compressed
/// LiDAR format `LAZ`, it is able to read `LAZ` file headers. Because this tool only requires information contained
/// in the input file's header (i.e. the bounding box of the data), it is able to take `LAZ` input files.
///
/// # See Also
/// `LidarTileFootprint`
pub struct SelectTilesByPolygon {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SelectTilesByPolygon {
    pub fn new() -> SelectTilesByPolygon {
        // public constructor
        let name = "SelectTilesByPolygon".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Copies LiDAR tiles overlapping with a polygon into an output directory.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Directory".to_owned(),
            flags: vec!["--indir".to_owned()],
            description: "Input LAS file source directory.".to_owned(),
            parameter_type: ParameterType::Directory,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Directory".to_owned(),
            flags: vec!["--outdir".to_owned()],
            description: "Output directory into which LAS files within the polygon are copied."
                .to_owned(),
            parameter_type: ParameterType::Directory,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Vector Polygon File".to_owned(),
            flags: vec!["--polygons".to_owned()],
            description: "Input vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --indir='*path*to*lidar*' --outdir='*output*path*' --polygons='watershed.shp'", short_exe, name).replace("*", &sep);

        SelectTilesByPolygon {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SelectTilesByPolygon {
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
        let mut input_directory: String = "".to_string();
        let mut output_directory: String = "".to_string();
        let mut polygons_file = String::new();

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
            if flag_val == "-indir" {
                input_directory = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-outdir" {
                output_directory = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-polygon" || flag_val == "-polygons" {
                polygons_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let start = Instant::now();
        let sep: String = path::MAIN_SEPARATOR.to_string();

        if input_directory.is_empty() {
            input_directory = working_directory.clone().to_string();
        }
        if !polygons_file.contains(&sep) && !polygons_file.contains("/") {
            polygons_file = format!("{}{}", working_directory, polygons_file);
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let mut inputs = vec![];

        if std::path::Path::new(&input_directory).is_dir() {
            for entry in fs::read_dir(input_directory.clone())? {
                let s = entry?
                    .path()
                    .into_os_string()
                    .to_str()
                    .expect("Error reading path string")
                    .to_string();
                if s.to_lowercase().ends_with(".las")
                    || s.to_lowercase().ends_with(".laz")
                    || s.to_lowercase().ends_with(".zlidar")
                {
                    inputs.push(s);
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("The input directory ({}) is incorrect.", input_directory),
            ));
        }

        let polygons = Arc::new(Shapefile::read(&polygons_file)?);
        let num_records = polygons.num_records;

        // make sure the input vector file is of polygon type
        if polygons.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type.",
            ));
        }

        // place the bounding boxes of each of the polygons into a vector
        let mut bb: Vec<BoundingBox> = Vec::with_capacity(num_records);
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);
            bb.push(BoundingBox::new(
                record.x_min,
                record.x_max,
                record.y_min,
                record.y_max,
            ));
        }

        let report_copy = Arc::new(Mutex::new(true));
        let inputs = Arc::new(inputs);
        let bb = Arc::new(bb);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        for _ in 0..num_procs {
            let inputs = inputs.clone();
            let polygons = polygons.clone();
            let bb = bb.clone();
            let tile_list = tile_list.clone();
            let tx = tx.clone();
            let report_copy = report_copy.clone();
            // copy over the string parameters
            let input_directory = input_directory.clone();
            let output_directory = output_directory.clone();
            thread::spawn(move || {
                let mut point_in_poly: bool;
                let mut start_point_in_part: usize;
                let mut end_point_in_part: usize;
                let mut k = 0;
                while k < num_tiles {
                    // Get the next tile up for examination
                    k = match tile_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to examine
                    };

                    let input_file = inputs[k].replace("\"", "").clone();
                    let header = match LasHeader::read_las_header(&input_file) {
                        Ok(h) => h,
                        Err(err) => panic!("Error reading file {}: {}", input_file, err),
                    };
                    let west = header.min_x;
                    let east = header.max_x;
                    let north = header.max_y;
                    let south = header.min_y;
                    let mid_point_x = (header.max_x - header.min_x) / 2.0;
                    let mid_point_y = (header.max_y - header.min_y) / 2.0;

                    // let input = match LasFile::new(&input_file, "rh") {
                    //     Ok(lf) => lf,
                    //     Err(err) => panic!("Error reading file {}: {}", input_file, err),
                    // };
                    // let west = input.header.min_x;
                    // let east = input.header.max_x;
                    // let north = input.header.max_y;
                    // let south = input.header.min_y;

                    // are any of the tile corners within a polygon?
                    point_in_poly = false;
                    for record_num in 0..polygons.num_records {
                        if bb[record_num].is_point_in_box(east, north)
                            || bb[record_num].is_point_in_box(west, north)
                            || bb[record_num].is_point_in_box(east, south)
                            || bb[record_num].is_point_in_box(west, south)
                        {
                            // it's in the bounding box and worth seeing if it's in the enclosed polygon
                            let record = polygons.get_record(record_num);
                            for part in 0..record.num_parts as usize {
                                if part == 0 || !record.is_hole(part as i32) {
                                    // not holes
                                    start_point_in_part = record.parts[part] as usize;
                                    end_point_in_part = if part < record.num_parts as usize - 1 {
                                        record.parts[part + 1] as usize - 1
                                    } else {
                                        record.num_points as usize - 1
                                    };

                                    if algorithms::point_in_poly(
                                        &Point2D { x: east, y: north },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D { x: west, y: north },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D { x: east, y: south },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D { x: west, y: south },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D {
                                            x: mid_point_x,
                                            y: mid_point_y,
                                        },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D {
                                            x: mid_point_x,
                                            y: south,
                                        },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D {
                                            x: mid_point_x,
                                            y: north,
                                        },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D {
                                            x: east,
                                            y: mid_point_y,
                                        },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if algorithms::point_in_poly(
                                        &Point2D {
                                            x: west,
                                            y: mid_point_y,
                                        },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if point_in_poly {
                        // copy the tile into the output directory
                        let output_file = inputs[k]
                            .replace("\"", "")
                            .replace(&input_directory, &output_directory)
                            .clone();

                        match fs::copy(input_file.clone(), output_file.clone()) {
                            Ok(_) => {
                                if verbose {
                                    // what's the report_copy status?
                                    let report_copy =
                                        report_copy.lock().expect("Error unlocking mutex");
                                    if *report_copy {
                                        println!(
                                            "Copied \"{}\" to \"{}\"",
                                            input_file.replace(&input_directory, "").clone(),
                                            output_directory.clone()
                                        )
                                    }
                                }
                            }
                            Err(e) => panic!("Error copying file {} \n{}", input_file, e),
                        }
                    }

                    tx.send(point_in_poly).unwrap();
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut num_files_copied = 0;
        for tile in 0..num_tiles {
            let in_poly = rx.recv().expect("Error receiving data from thread.");
            if in_poly {
                num_files_copied += 1;
                if num_files_copied == 50 {
                    if verbose {
                        println!("...");
                    }
                    let mut report_copy = report_copy.lock().expect("Error unlocking mutex");
                    *report_copy = false;
                }
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (num_tiles - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Number of files copied: {}", num_files_copied);

            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
