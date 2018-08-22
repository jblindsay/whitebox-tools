/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/08/2018
Last Modified: 02/08/2018
License: MIT
*/

use lidar::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use structures::BoundingBox;
use time;
use tools::*;
use vector;
use vector::{Point2D, ShapeType, Shapefile};

/// Creates vector polygons for the extents of a set of LiDAR tiles.
pub struct LidarTileFootprint {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarTileFootprint {
    pub fn new() -> LidarTileFootprint {
        // public constructor
        let name = "LidarTileFootprint".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Creates vector polygons for the extents of a set of LiDAR tiles.".to_string();

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
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --indir='*path*to*lidar*' --outdir='*output*path*' --polygons='watershed.shp'", short_exe, name).replace("*", &sep);

        LidarTileFootprint {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarTileFootprint {
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

        let start = time::now();
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

        match fs::read_dir(input_directory.clone()) {
            Err(why) => println!("{:?}", why.kind()),
            Ok(paths) => for path in paths {
                let s = format!("{:?}", path.unwrap().path());
                if s.replace("\"", "").to_lowercase().ends_with(".las") {
                    inputs.push(format!("{:?}", s.replace("\"", "")));
                }
            },
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

        let inputs = Arc::new(inputs);
        let bb = Arc::new(bb);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        for _ in 0..num_procs {
            let inputs = inputs.clone();
            let polygons = polygons.clone();
            let bb = bb.clone();
            let tile_list = tile_list.clone();
            let tx = tx.clone();
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
                    let input = match LasFile::new(&input_file, "rh") {
                        Ok(lf) => lf,
                        Err(err) => panic!(format!("Error reading file {}: {}", input_file, err)),
                    };
                    let west = input.header.min_x;
                    let east = input.header.max_x;
                    let north = input.header.max_y;
                    let south = input.header.min_y;

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
                                if !record.is_hole(part as i32) {
                                    // not holes
                                    start_point_in_part = record.parts[part] as usize;
                                    end_point_in_part = if part < record.num_parts as usize - 1 {
                                        record.parts[part + 1] as usize - 1
                                    } else {
                                        record.num_points as usize - 1
                                    };

                                    if vector::point_in_poly(
                                        &Point2D { x: east, y: north },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if vector::point_in_poly(
                                        &Point2D { x: west, y: north },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if vector::point_in_poly(
                                        &Point2D { x: east, y: south },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }

                                    if vector::point_in_poly(
                                        &Point2D { x: west, y: south },
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
                            Ok(_) => println!(
                                "Copied \"{}\" to \"{}\"",
                                input_file.replace(&input_directory, "").clone(),
                                output_file.replace(&output_directory, "").clone()
                            ),
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
            let in_poly = rx.recv().unwrap();
            if in_poly {
                num_files_copied += 1;
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (num_tiles - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!("Number of files copied: {}", num_files_copied);

            println!(
                "{}",
                &format!("Elapsed Time: {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
