/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/04/2018
Last Modified: 26/07/2019
License: MIT
*/

use whitebox_common::algorithms;
use whitebox_lidar::*;
use whitebox_common::structures::{BoundingBox, Point2D, Point3D};
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use num_cpus;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::{mpsc, Arc};
use std::thread;

/// This tool can be used to isolate, or clip, all of the LiDAR points in a LAS file (`--input`) contained within
/// one or more vector polygon features. The user must specify the name of the input clip file (--polygons), which
/// must be a vector of a Polygon base shape type. The clip file may contain multiple polygon features and polygon hole
/// parts will be respected during clipping, i.e. LiDAR points within polygon holes will be removed from the output LAS
/// file.
///
/// Use the `ErasePolygonFromLidar` tool to perform the complementary operation of removing points from a LAS file
/// that are contained within a set of polygons.
///
/// # See Also
/// `ErasePolygonFromLidar`, `FilterLidar`, `Clip`, `ClipRasterToPolygon`
pub struct ClipLidarToPolygon {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ClipLidarToPolygon {
    /// public constructor
    pub fn new() -> ClipLidarToPolygon {
        let name = "ClipLidarToPolygon".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Clips a LiDAR point cloud to a vector polygon or polygons.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
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

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='data.las' --polygons='lakes.shp' -o='output.las'", short_exe, name).replace("*", &sep);

        ClipLidarToPolygon {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ClipLidarToPolygon {
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
        let mut polygons_file = String::new();
        let mut output_file = String::new();

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
            } else if flag_val == "-polygon" || flag_val == "-polygons" {
                polygons_file = if keyval {
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
        if !polygons_file.contains(&sep) && !polygons_file.contains("/") {
            polygons_file = format!("{}{}", working_directory, polygons_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let lidar_bb = BoundingBox::new(
            input.header.min_x,
            input.header.max_x,
            input.header.min_y,
            input.header.max_y,
        );

        let polygons = Shapefile::read(&polygons_file)?;
        let num_records = polygons.num_records;

        let start = Instant::now();

        // make sure the input vector file is of polygon type
        if polygons.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type.",
            ));
        }

        // place the bounding boxes of each of the polygons into a vector
        let mut bb: Vec<BoundingBox> = Vec::with_capacity(num_records);
        let mut feature_bb;
        let mut record_nums = Vec::with_capacity(num_records);
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);
            feature_bb = BoundingBox::new(record.x_min, record.x_max, record.y_min, record.y_max);
            if feature_bb.overlaps(lidar_bb) {
                bb.push(feature_bb);
                record_nums.push(record_num);
            }
        }

        if verbose {
            println!("Performing clip...")
        };

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let num_procs = num_cpus::get();
        let input = Arc::new(input);
        let polygons = Arc::new(polygons);
        let record_nums = Arc::new(record_nums);
        let bb = Arc::new(bb);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let polygons = polygons.clone();
            let record_nums = record_nums.clone();
            let bb = bb.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                // let mut p: PointData;
                let mut p: Point3D;
                let mut record_num: usize;
                let mut point_in_poly: bool;
                let mut start_point_in_part: usize;
                let mut end_point_in_part: usize;
                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    // p = input.get_point_info(point_num);
                    p = input.get_transformed_coords(point_num);
                    point_in_poly = false;
                    for r in 0..record_nums.len() {
                        record_num = record_nums[r];
                        if bb[r].is_point_in_box(p.x, p.y) {
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

                                    if algorithms::point_in_poly(
                                        &Point2D { x: p.x, y: p.y },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = true;
                                        break;
                                    }
                                }
                            }

                            for part in 0..record.num_parts as usize {
                                if record.is_hole(part as i32) {
                                    // holes
                                    start_point_in_part = record.parts[part] as usize;
                                    end_point_in_part = if part < record.num_parts as usize - 1 {
                                        record.parts[part + 1] as usize - 1
                                    } else {
                                        record.num_points as usize - 1
                                    };

                                    if algorithms::point_in_poly(
                                        &Point2D { x: p.x, y: p.y },
                                        &record.points[start_point_in_part..end_point_in_part + 1],
                                    ) {
                                        point_in_poly = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    match tx.send((point_in_poly, point_num)) {
                        Ok(_) => {} // do nothing
                        Err(_) => panic!(
                            "Error performing clipping operation on point num. {}",
                            point_num
                        ),
                    };
                }
            });
        }

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();
        for i in 0..n_points {
            let data = rx.recv().expect("Error receiving data from thread.");
            if data.0 {
                output.add_point_record(input.get_record(data.1));
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let mut output = LasFile::initialize_using_file(&output_file, &input);
        // output.header.system_id = "EXTRACTION".to_string();

        // let n_points = input.header.number_of_points as usize;
        // let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only
        // let mut point_in_poly: bool;
        // let mut p: PointData;
        // let mut start_point_in_part: usize;
        // let mut end_point_in_part: usize;
        // let mut record_num: usize;
        // for point_num in 0..n_points {
        //     p = input.get_point_info(point_num);
        //     point_in_poly = false;
        //     for r in 0..record_nums.len() {
        //         record_num = record_nums[r];
        //         if bb[r].is_point_in_box(p.x, p.y) {
        //             // it's in the bounding box and worth seeing if it's in the enclosed polygon
        //             let record = polygons.get_record(record_num);
        //             for part in 0..record.num_parts as usize {
        //                 if !record.is_hole(part as i32) {
        //                     // not holes
        //                     start_point_in_part = record.parts[part] as usize;
        //                     end_point_in_part = if part < record.num_parts as usize - 1 {
        //                         record.parts[part + 1] as usize - 1
        //                     } else {
        //                         record.num_points as usize - 1
        //                     };

        //                     if algorithms::point_in_poly(
        //                         &Point2D { x: p.x, y: p.y },
        //                         &record.points[start_point_in_part..end_point_in_part + 1],
        //                     ) {
        //                         point_in_poly = true;
        //                         break;
        //                     }
        //                 }
        //             }

        //             for part in 0..record.num_parts as usize {
        //                 if record.is_hole(part as i32) {
        //                     // holes
        //                     start_point_in_part = record.parts[part] as usize;
        //                     end_point_in_part = if part < record.num_parts as usize - 1 {
        //                         record.parts[part + 1] as usize - 1
        //                     } else {
        //                         record.num_points as usize - 1
        //                     };

        //                     if algorithms::point_in_poly(
        //                         &Point2D { x: p.x, y: p.y },
        //                         &record.points[start_point_in_part..end_point_in_part + 1],
        //                     ) {
        //                         point_in_poly = false;
        //                         break;
        //                     }
        //                 }
        //             }
        //         }
        //     }

        //     if point_in_poly {
        //         output.add_point_record(input.get_record(point_num));
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * point_num as f64 / num_points) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Writing output LAS file...");
        }
        if output.header.number_of_points > 0 {
            let _ = match output.write() {
                Ok(_) => {
                    if verbose {
                        println!("Complete!")
                    }
                }
                Err(e) => println!("error while writing: {:?}", e),
            };
        } else {
            if verbose {
                println!("Warning: the file {} does not appear to contain any points within the clip polygon. No output file has been created.", output.get_short_filename());
            }
        }
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
