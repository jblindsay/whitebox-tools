/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/06/2020
Last Modified: 10/06/2020
License: MIT
*/

use self::na::Vector3;
use whitebox_common::algorithms;
use whitebox_lidar::*;
use crate::na;
use whitebox_common::structures::{DistanceMetric, FixedRadiusSearch3D, Point2D, Point3D, RectangleWithData};
use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use rand::seq::SliceRandom;
use rstar::RTree;
use std::io::{Error, ErrorKind};
use std::ops::AddAssign;
use std::path;
use std::sync::{mpsc, Arc};
use std::thread;
use std::{env, fs};
const EPSILON: f64 = std::f64::EPSILON;

/// This tool can be used to identify roof segments in a LiDAR point cloud.
///
/// # See Also
/// `ClassifyBuildingsInLidar`, `ClipLidarToPolygon`
pub struct LidarRooftopAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarRooftopAnalysis {
    /// public constructor
    pub fn new() -> LidarRooftopAnalysis {
        let name = "LidarRooftopAnalysis".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Identifies roof segments in a LiDAR point cloud.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Input Building Footprint Polygon File".to_owned(),
            flags: vec!["--buildings".to_owned()],
            description: "Input vector build footprint polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        // parameters.push(ToolParameter {
        //     name: "Output LiDAR File".to_owned(),
        //     flags: vec!["--output_lidar".to_owned()],
        //     description: "Output LiDAR file.".to_owned(),
        //     parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
        //     default_value: None,
        //     optional: false,
        // });

        parameters.push(ToolParameter {
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Search Radius".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Number of Iterations".to_owned(),
            flags: vec!["--num_iter".to_owned()],
            description: "Number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("50".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Number of Sample Points".to_owned(),
            flags: vec!["--num_samples".to_owned()],
            description: "Number of sample points on which to build the model.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("10".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Inlier Threshold".to_owned(),
            flags: vec!["--threshold".to_owned()],
            description: "Threshold used to determine inlier points (in elevation units)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.15".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Acceptable Model Size (points)".to_owned(),
            flags: vec!["--model_size".to_owned()],
            description: "Acceptable model size, in points.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("15".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Planar Slope (degrees)".to_owned(),
            flags: vec!["--max_slope".to_owned()],
            description: "Maximum planar slope, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("65.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Normal Difference Threshold (degrees)".to_owned(),
            flags: vec!["--norm_diff".to_owned()],
            description: "Maximum difference in normal vectors, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: true,
        });

        // parameters.push(ToolParameter{
        //     name: "Maximum Elevation Difference Between Points".to_owned(),
        //     flags: vec!["--maxzdiff".to_owned()],
        //     description: "Maximum difference in elevation (z units) between neighbouring points of the same segment.".to_owned(),
        //     parameter_type: ParameterType::Float,
        //     default_value: Some("1.0".to_owned()),
        //     optional: true
        // });

        parameters.push(ToolParameter {
            name: "Azimuth (degrees)".to_owned(),
            flags: vec!["--azimuth".to_owned()],
            description: "Illumination source azimuth, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("180.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Altitude (degrees)".to_owned(),
            flags: vec!["--altitude".to_owned()],
            description: "Illumination source altitude in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("30.0".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='data.las' --polygons='buildings.shp' -o='rooftops.shp' --radius=10.0 --num_iter=10 --num_samples=5 --threshold=0.25 --max_slope=70.0", short_exe, name).replace("*", &sep);

        LidarRooftopAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarRooftopAnalysis {
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
        // let mut output_lidar_file = String::new();
        let mut search_radius = 2f64;
        let mut num_iter = 50;
        let mut num_samples = 10;
        let mut threshold = 0.15;
        let mut acceptable_model_size = 30;
        let mut max_slope = 65f64;
        let mut max_norm_diff = 2f64;
        let mut max_z_diff = 1f64;
        let mut azimuth = 180.0f64;
        let mut altitude = 30.0f64;

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
            } else if flag_val == "-building" || flag_val == "-buildings" {
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
            // } else if flag_val == "-output_lidar" {
            //     output_lidar_file = if keyval {
            //         vec[1].to_string()
            //     } else {
            //         args[i + 1].to_string()
            //     };
            } else if flag_val == "-radius" {
                search_radius = if keyval {
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
            } else if flag_val == "-num_iter" {
                num_iter = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-num_samples" {
                num_samples = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-threshold" {
                threshold = if keyval {
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
            } else if flag_val == "-model_size" {
                acceptable_model_size = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-max_slope" {
                max_slope = if keyval {
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
                if max_slope < 5f64 {
                    max_slope = 5f64;
                }
            } else if flag_val == "-norm_diff" {
                max_norm_diff = if keyval {
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
            } else if flag_val == "-maxzdiff" {
                max_z_diff = if keyval {
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
            } else if flag_val == "-azimuth" {
                if keyval {
                    azimuth = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    azimuth = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-altitude" {
                if keyval {
                    altitude = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    altitude = args[i + 1]
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

        let mut progress: i32;
        let mut old_progress: i32 = -1;

        azimuth = (azimuth - 90f64).to_radians();
        altitude = altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();

        // if !input_file.contains(&sep) && !input_file.contains("/") {
        //     input_file = format!("{}{}", working_directory, input_file);
        // }
        if !polygons_file.contains(&sep) && !polygons_file.contains("/") {
            polygons_file = format!("{}{}", working_directory, polygons_file);
        }
        // if !output_file.contains(&sep) && !output_file.contains("/") {
        //     output_file = format!("{}{}", working_directory, output_file);
        // }
        // if !output_lidar_file.contains(&sep) && !output_lidar_file.contains("/") {
        //     output_lidar_file = format!("{}{}", working_directory, output_lidar_file);
        // }

        if verbose {
            println!("Reading data...")
        };

        if acceptable_model_size < 5 {
            acceptable_model_size = 5;
            if verbose {
                println!("Warning: The --model_size parameter must be at least 5.");
            }
        }

        if num_samples < 5 {
            num_samples = 5;
            if verbose {
                println!("Warning: The --num_samples parameter must be at least 5.");
            }
        }

        let larger_of_two_samples = num_samples.max(acceptable_model_size);

        if max_norm_diff < 0f64 {
            max_norm_diff = 0f64;
        }
        if max_norm_diff > 90f64 {
            max_norm_diff = 90f64;
        }
        max_norm_diff = max_norm_diff.to_radians();

        let mut inputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
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
                    } else if s.to_lowercase().ends_with(".laz") {
                        inputs.push(s);
                    } else if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
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
                    .replace(".las", ".shp")
                    .replace(".LAS", ".shp")
                    .replace(".zlidar", ".shp");
            }
            if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }
        }

        let polygons =
            Arc::new(Shapefile::read(&polygons_file).expect("Error reading buildings polygon."));
        let num_records = polygons.num_records;

        let start = Instant::now();

        // make sure the input vector file is of polygon type
        if polygons.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type.",
            ));
        }

        // place the axis-aligned bounding boxes of each of the polygons into a vector
        let mut building_aabb = Vec::with_capacity(num_records);
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);
            if record.shape_type != ShapeType::Null {
                building_aabb.push(RectangleWithData::new(
                    record_num,
                    [record.x_min, record.y_min],
                    [record.x_max, record.y_max],
                ));
            }
        }

        let num_buildings = polygons.num_records;

        let building_tree = Arc::new(RTree::bulk_load(building_aabb));

        let num_files = inputs.len();
        if num_files == 0 {
            panic!("No input LiDAR files were found");
        }

        if verbose {
            println!("Clipping lidar to building footprints...")
        };

        // let input = LasFile::new(&inputs[0], "r").expect("Error reading input LiDAR file");
        // let mut output_lidar = LasFile::initialize_using_file(&output_lidar_file, &input);
        // output_lidar.header.system_id = "EXTRACTION".to_string();
        // output_lidar.header.point_format = 2;
        // drop(input);

        // let mut las_points = vec![];
        let mut las_points_transformed: Vec<Point3D> = vec![];
        let mut building_num = vec![];
        let mut frs: FixedRadiusSearch3D<usize> =
            FixedRadiusSearch3D::new(search_radius, DistanceMetric::SquaredEuclidean);
        let mut point_num = 0usize;
        let mut projection = String::new();
        let num_procs = num_cpus::get();
        let mut file_num = 0;
        for input_file in inputs {
            match LasFile::new(&input_file, "r") {
                Ok(mut input) => {
                    if verbose && file_num < 100 {
                        println!("Clipping: {}", input.get_short_filename());
                    }
                    projection = input.get_wkt();
                    let n_points = input.header.number_of_points as usize;
                    let input = Arc::new(input);
                    let (tx, rx) = mpsc::channel();
                    for tid in 0..num_procs {
                        let input = input.clone();
                        let building_tree = building_tree.clone();
                        let polygons = polygons.clone();
                        let tx = tx.clone();
                        thread::spawn(move || {
                            let mut progress: usize;
                            let mut old_progress = 1usize;
                            let mut pd: PointData;
                            let mut p: Point3D;
                            let mut record_num: usize;
                            // let mut record: &ShapefileGeometry;
                            let mut point_in_poly: bool;
                            let mut start_point_in_part: usize;
                            let mut end_point_in_part: usize;
                            let mut building_points = vec![];
                            let mut building_rec: usize;
                            for point_num in
                                (0..n_points).filter(|point_num| point_num % num_procs == tid)
                            {
                                // p = input.get_point_info(point_num);
                                p = input.get_transformed_coords(point_num);
                                pd = input[point_num];
                                if !pd.withheld()
                                    && !pd.is_classified_noise()
                                    && pd.is_late_return()
                                    && (pd.classification() < 2 || pd.classification() > 5)
                                {
                                    point_in_poly = false;
                                    building_rec = 0usize;
                                    let ret = building_tree
                                        .locate_all_at_point(&[p.x, p.y])
                                        .collect::<Vec<_>>();
                                    for a in 0..ret.len() {
                                        record_num = ret[a].data;
                                        let record = polygons.get_record(record_num);
                                        for part in 0..record.num_parts as usize {
                                            if !record.is_hole(part as i32) {
                                                // not holes
                                                start_point_in_part = record.parts[part] as usize;
                                                end_point_in_part =
                                                    if part < record.num_parts as usize - 1 {
                                                        record.parts[part + 1] as usize - 1
                                                    } else {
                                                        record.num_points as usize - 1
                                                    };

                                                if algorithms::point_in_poly(
                                                    &Point2D { x: p.x, y: p.y },
                                                    &record.points[start_point_in_part
                                                        ..end_point_in_part + 1],
                                                ) {
                                                    point_in_poly = true;
                                                    building_rec = record_num;
                                                    break;
                                                }
                                            }
                                        }
                                        for part in 0..record.num_parts as usize {
                                            if record.is_hole(part as i32) {
                                                // holes
                                                start_point_in_part = record.parts[part] as usize;
                                                end_point_in_part =
                                                    if part < record.num_parts as usize - 1 {
                                                        record.parts[part + 1] as usize - 1
                                                    } else {
                                                        record.num_points as usize - 1
                                                    };

                                                if algorithms::point_in_poly(
                                                    &Point2D { x: p.x, y: p.y },
                                                    &record.points[start_point_in_part
                                                        ..end_point_in_part + 1],
                                                ) {
                                                    point_in_poly = false;
                                                    break;
                                                }
                                            }
                                        }
                                    }

                                    if point_in_poly {
                                        building_points.push((point_num, building_rec));
                                    }
                                }

                                if num_files == 1 && tid == 0 && verbose {
                                    progress =
                                        (100.0_f64 * point_num as f64 / n_points as f64) as usize;
                                    if progress != old_progress {
                                        println!("Progress: {}%", progress);
                                        old_progress = progress;
                                    }
                                }
                            }
                            match tx.send(building_points) {
                                Ok(_) => {} // do nothing
                                Err(_) => panic!(
                                    "Error performing clipping operation on file: {}.",
                                    input.get_short_filename()
                                ),
                            };
                        });
                    }

                    let mut point_in_building = vec![false; n_points];
                    let mut building = vec![0; n_points];
                    for _ in 0..num_procs {
                        let in_building_points =
                            rx.recv().expect("Error receiving data from thread.");
                        for p in in_building_points {
                            point_in_building[p.0] = true;
                            building[p.0] = p.1;
                        }
                    }

                    for p in 0..n_points {
                        if point_in_building[p] {
                            // let pr = input.get_record(p);
                            // let pd = pr.get_point_data();
                            let xyz = input.get_transformed_coords(p);
                            // las_points.push(pr);
                            las_points_transformed.push(xyz);
                            building_num.push(building[p]);
                            frs.insert(xyz.x, xyz.y, xyz.z, point_num);
                            point_num += 1
                        }
                        if num_files == 1 && verbose {
                            progress = (100.0_f64 * p as f64 / n_points as f64) as i32;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    if num_files > 1 && verbose {
                        file_num += 1;
                        progress = (100.0_f64 * file_num as f64 / num_files as f64) as i32;
                        if progress != old_progress {
                            println!("Progress ({} of {}): {}%", file_num, num_files, progress);
                            old_progress = progress;
                        }
                    }
                }
                Err(err) => {
                    if verbose {
                        println!("Error reading file {}: {}", input_file, err);
                    }
                }
            };
        }


        ////////////
        // RANSAC //
        ////////////

        if verbose {
            println!("Building roof facet models...");
        }

        let n_points = las_points_transformed.len();
        let num_points: f64 = (las_points_transformed.len() - 1) as f64; // used for progress calculation only

        if n_points == 0 {
            panic!("No LiDAR points were found when clipping to the building footprints. It is possible that the buildings Shapefile is not in the same projection as the LiDAR data. Use LidarInfo to determine.");
        }

        let frs = Arc::new(frs); // wrap FRS in an Arc
        // let las_points = Arc::new(las_points);
        let las_points_transformed = Arc::new(las_points_transformed);
        let building_num = Arc::new(building_num);
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            // let las_points = las_points.clone();
            let las_points_transformed = las_points_transformed.clone();
            let building_num = building_num.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut n: usize;
                // let mut p1: PointData;
                // let mut p2: PointData;
                let mut p1: Point3D;
                let mut p2: Point3D;
                let mut index: usize;
                let mut rng = &mut rand::thread_rng();
                let mut model: Plane;
                let mut better_model: Plane;
                let mut center_point: Vector3<f64>;
                let mut rmse: f64;
                let mut min_rmse = f64::MAX;
                let mut model_contains_center_point: bool;
                let mut building_id: usize;
                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let mut best_model: Plane = Plane::zero();
                    // find the best fitting planar model that contains this point
                    // p1 = input.get_point_info(point_num);
                    // p1 = las_points[point_num].get_point_data();
                    p1 = las_points_transformed[point_num];
                    center_point = Vector3::new(p1.x, p1.y, p1.z);
                    building_id = building_num[point_num];
                    let ret = frs.search(p1.x, p1.y, p1.z);
                    n = ret.len();
                    let mut points: Vec<Vector3<f64>> = Vec::with_capacity(n);
                    let mut model_found = false;
                    let mut model_points: Vec<usize> = Vec::with_capacity(n);
                    for j in 0..n {
                        index = ret[j].0;
                        if building_num[index] == building_id {
                            // p2 = las_points[index].get_point_data();
                            p2 = las_points_transformed[index];
                            points.push(Vector3::new(p2.x, p2.y, p2.z));
                        }
                    }
                    n = points.len();
                    if n > larger_of_two_samples {
                        min_rmse = f64::MAX;
                        let v: Vec<usize> = (0..n).collect();
                        for _ in 0..num_iter {
                            // select n random samples.
                            let samples: Vec<usize> =
                                v.choose_multiple(&mut rng, num_samples).cloned().collect();
                            let data: Vec<Vector3<f64>> =
                                samples.into_iter().map(|a| points[a]).collect();
                            // get the best-fit plane
                            model = Plane::from_points(&data);
                            if model.slope() < max_slope {
                                let mut inliers: Vec<Vector3<f64>> = Vec::with_capacity(n);
                                for j in 0..n {
                                    if model.residual(&points[j]) < threshold {
                                        inliers.push(points[j]);
                                    }
                                }
                                if inliers.len() >= acceptable_model_size {
                                    better_model = Plane::from_points(&inliers);
                                    rmse = better_model.rmse(&inliers);
                                    model_contains_center_point =
                                        better_model.residual(&center_point) < threshold;
                                    if rmse < min_rmse && model_contains_center_point {
                                        min_rmse = rmse;
                                        best_model = better_model;
                                        model_found = true;
                                        if inliers.len() == n || min_rmse == 0f64 {
                                            // You can't get any better than that.
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if model_found {
                        for j in 0..n {
                            index = ret[j].0;
                            if best_model.residual(&points[j]) <= threshold {
                                model_points.push(index);
                            }
                        }
                        if model_points.len() < acceptable_model_size {
                            model_points.clear();
                        }
                    }
                    tx.send((best_model, min_rmse, model_points)).unwrap();
                }
            });
        }

        let mut model_rmse = vec![f64::MAX; n_points];
        let mut planes = vec![Plane::zero(); n_points];
        for i in 0..n_points {
            let (model, rmse, model_points) = rx.recv().expect("Error receiving data from thread.");
            if rmse < f64::MAX {
                for index in model_points {
                    if rmse < model_rmse[index] {
                        model_rmse[index] = rmse;
                        planes[index] = model;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // ////////////////////////////////////////
        // // Perform the segmentation operation //
        ////////////////////////////////////////
        if verbose {
            println!("Segmenting the point cloud...");
        }
        // let mut p: PointData;
        // let mut pn: PointData;
        let mut p: Point3D;
        let mut pn: Point3D;
        let mut segment_id = vec![0usize; n_points];
        let mut building_id: usize = 0;
        let mut building_id_n: usize;
        let mut current_segment = 1usize;
        let mut point_id: usize;
        let mut norm_diff: f64;
        let mut height_diff: f64;
        let mut index: usize;
        let mut solved_points = 0;
        let mut stack = vec![];
        let mut last_seed = 0;
        let mut is_planar: bool;
        let mut is_planar_n: bool;

        // all non-planar points belong to the same segment
        for i in 0..n_points {
            if model_rmse[i] == f64::MAX {
                segment_id[i] = current_segment;
                solved_points += 1;
            }
        }
        current_segment += 1;

        // now do a more fulsome search.
        while solved_points < n_points {
            // Find a seed-point for a segment
            for i in last_seed..n_points {
                if segment_id[i] == 0 {
                    // No segment ID has yet been assigned to this point.
                    // p = las_points[i].get_point_data();
                    current_segment += 1;
                    segment_id[i] = current_segment;
                    building_id = building_num[i];
                    stack.push(i);
                    last_seed = i;
                    break;
                }
            }

            while !stack.is_empty() {
                solved_points += 1;
                if verbose {
                    progress = (100f64 * solved_points as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Segmenting the point cloud: {}%", progress);
                        old_progress = progress;
                    }
                }
                point_id = stack.pop().expect("Error during pop operation.");
                is_planar = if model_rmse[point_id] < f64::MAX {
                    true
                } else {
                    false
                };
                /* Check the neighbours to see if there are any
                points that have similar normal vectors and
                heights. */
                // p = las_points[point_id].get_point_data();
                p = las_points_transformed[point_id];
                let ret = frs.search(p.x, p.y, p.z);
                for j in 0..ret.len() {
                    index = ret[j].0;
                    if segment_id[index] == 0 {
                        // It hasn't already been placed in a segment.
                        is_planar_n = if model_rmse[index] < f64::MAX {
                            true
                        } else {
                            false
                        };
                        building_id_n = building_num[index];
                        if is_planar && is_planar_n && building_id == building_id_n {
                            // pn = las_points[index].get_point_data();
                            pn = las_points_transformed[index];
                            height_diff = (pn.z - p.z).abs();
                            if height_diff < max_z_diff {
                                // check the norm diff angle
                                norm_diff = planes[point_id].angle_between(planes[index]);
                                if norm_diff < max_norm_diff {
                                    segment_id[index] = current_segment;
                                    stack.push(index);
                                }
                            }
                        }
                    }
                }
            }
        }

        /////////////////////////////////////
        // Collect segment characteristics //
        /////////////////////////////////////
        let mut segment_max_elev = vec![0f64; current_segment + 1];
        let mut segment_size = vec![0usize; current_segment + 1];
        let mut segment_plane = vec![Plane::zero(); current_segment + 1];
        let mut segment_points = vec![vec![]; current_segment + 1];
        let mut segment_z_vals = vec![vec![]; current_segment + 1];
        let mut seg_building_id = vec![0usize; current_segment + 1];

        let mut seg_num: usize;
        for point_id in 0..n_points {
            seg_num = segment_id[point_id];
            segment_size[seg_num] += 1;
            // p = las_points[point_id].get_point_data();
            p = las_points_transformed[point_id];
            segment_points[seg_num].push(Point2D::new(p.x, p.y));
            segment_z_vals[seg_num].push(p.z);
            if p.z > segment_max_elev[seg_num] {
                segment_max_elev[seg_num] = p.z;
            }
            seg_building_id[seg_num] = building_num[point_id];
        }

        // drop(las_points);
        drop(las_points_transformed);

        let mut building_segments = vec![vec![]; num_buildings];
        for seg_num in 2..current_segment + 1 {
            if segment_size[seg_num] > 0 {
                building_segments[seg_building_id[seg_num]].push(seg_num);
                // segment_max_elev[seg_num] /= segment_size[seg_num] as f64;
                let data: Vec<Vector3<f64>> = (0..segment_points[seg_num].len())
                    .into_iter()
                    .map(|a| {
                        Vector3::new(
                            segment_points[seg_num][a].x,
                            segment_points[seg_num][a].y,
                            segment_z_vals[seg_num][a],
                        )
                    })
                    .collect();
                segment_plane[seg_num] = Plane::from_points(&data);
            }
        }

        ////////////////////////////////
        // Vectorize the polygon data //
        ////////////////////////////////

        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;
        output.projection = projection.clone();
        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 8u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "BUILDING",
            FieldDataType::Int,
            8u8,
            0u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "MAX_ELEV",
            FieldDataType::Real,
            11u8,
            4u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "HILLSHADE",
            FieldDataType::Real,
            11u8,
            5u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "SLOPE",
            FieldDataType::Real,
            11u8,
            5u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "ASPECT",
            FieldDataType::Real,
            11u8,
            5u8,
        ));
        output
            .attributes
            .add_field(&AttributeField::new("AREA", FieldDataType::Real, 11u8, 5u8));
        // output.attributes.add_field(&AttributeField::new(
        //     "SEGMENT",
        //     FieldDataType::Int,
        //     8u8,
        //     0u8,
        // ));

        // let mut output_pts = Shapefile::new(&output_file.replace(".shp", "_pts.shp"), ShapeType::MultiPointM)?;
        // output_pts.projection = projection;
        // // add the attributes
        // output_pts
        //     .attributes
        //     .add_field(&AttributeField::new("BUILDING", FieldDataType::Int, 8u8, 0u8));
        // output_pts
        //     .attributes
        //     .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));
        //     output_pts.attributes.add_field(&AttributeField::new(
        //     "MAX_ELEV",
        //     FieldDataType::Real,
        //     11u8,
        //     4u8,
        // ));
        // output_pts.attributes.add_field(&AttributeField::new(
        //     "HILLSHADE",
        //     FieldDataType::Real,
        //     11u8,
        //     5u8,
        // ));
        // output_pts.attributes.add_field(&AttributeField::new(
        //     "SLOPE",
        //     FieldDataType::Real,
        //     11u8,
        //     5u8,
        // ));
        // output_pts.attributes.add_field(&AttributeField::new(
        //     "ASPECT",
        //     FieldDataType::Real,
        //     11u8,
        //     5u8,
        // ));
        // output_pts.attributes.add_field(&AttributeField::new(
        //     "AREA",
        //     FieldDataType::Real,
        //     11u8,
        //     5u8,
        // ));

        let min_seg_size = 10;
        let mut fid = 1;
        // let mut azimuth = 180.0f64; //31
        let (mut fx, mut fy, mut tan_slope, mut aspect): (f64, f64, f64, f64);
        let (mut term1, mut term2, mut term3): (f64, f64, f64);
        let mut hillshade = 0f64;
        let mut area: f64;

        // let mut num_pts_in_building: usize;
        let (mut p1, mut p2, mut p3): (usize, usize, usize);
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        let (mut x, mut y): (f64, f64);
        const EMPTY: usize = usize::MAX;
        let precision = EPSILON * 1000.0;
        for building_id in 0..num_buildings {
            let mut points = vec![];
            let mut points_id = vec![];
            for v in 0..building_segments[building_id].len() {
                seg_num = building_segments[building_id][v];
                if segment_size[seg_num] > min_seg_size {
                    tan_slope = 0f64;
                    if segment_plane[seg_num].c != 0f64 {
                        fx = -segment_plane[seg_num].a / segment_plane[seg_num].c;
                        if fx != 0f64 {
                            fy = -segment_plane[seg_num].b / segment_plane[seg_num].c;

                            tan_slope = (fx * fx + fy * fy).sqrt().atan().to_degrees();
                        }
                    }
                    if tan_slope < max_slope {
                        for a in 0..segment_size[seg_num] {
                            points.push(segment_points[seg_num][a]);
                            points_id.push(seg_num);
                        }
                    }
                }
            }

            let mut kdtree = KdTree::with_capacity(2, 64);

            if points.len() > 3 {
                let record = polygons.get_record(building_id);
                // num_pts_in_building += record.num_points as usize;

                for part in 0..record.num_parts as usize {
                    start_point_in_part = record.parts[part] as usize;
                    end_point_in_part = if part < record.num_parts as usize - 1 {
                        record.parts[part + 1] as usize - 1
                    } else {
                        record.num_points as usize - 1
                    };

                    // points.push(record.points[start_point_in_part]);
                    // points_id.push(0usize);
                    for i in start_point_in_part + 1..=end_point_in_part {
                        let dist = record.points[i].distance(&record.points[i - 1]);
                        for d in 0..((dist / 0.1f64).floor() as usize) {
                            x = record.points[i - 1].x
                                + ((d as f64) * 0.1f64 / dist)
                                    * (record.points[i].x - record.points[i - 1].x);
                            y = record.points[i - 1].y
                                + ((d as f64) * 0.1f64 / dist)
                                    * (record.points[i].y - record.points[i - 1].y);
                            points.push(Point2D::new(x, y));
                            points_id.push(EMPTY);
                        }
                        if dist.floor() != dist {
                            points.push(record.points[i]);
                            points_id.push(EMPTY);
                        }
                    }
                }

                let result = algorithms::triangulate(&points).expect("No triangulation exists.");
                let mut endnodes = vec![];
                let mut node = 0usize;
                let mut num_polylines = 0usize;
                let mut roof_segments: Vec<[usize; 2]> = vec![];
                for i in (0..result.triangles.len()).step_by(3) {
                    // the points in triangles are counter clockwise ordered and we need clockwise
                    p1 = result.triangles[i + 2];
                    p2 = result.triangles[i + 1];
                    p3 = result.triangles[i];

                    if points_id[p1] != points_id[p2]
                        || points_id[p1] != points_id[p3]
                        || points_id[p2] != points_id[p3]
                    {
                        if points_id[p1] != points_id[p2]
                            && points_id[p1] != points_id[p3]
                            && points_id[p2] != points_id[p3]
                        {
                            let centroid = Point2D::new(
                                (points[p1].x + points[p2].x + points[p3].x) / 3f64,
                                (points[p1].y + points[p2].y + points[p3].y) / 3f64,
                            );

                            x = (points[p1].x + points[p2].x) / 2f64;
                            y = (points[p1].y + points[p2].y) / 2f64;
                            kdtree
                                .add([x, y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(Point2D::new(x, y));
                            roof_segments.push([points_id[p1], points_id[p2]]);
                            node += 1;
                            kdtree
                                .add([centroid.x, centroid.y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(centroid);
                            roof_segments.push([points_id[p1], points_id[p2]]);
                            node += 1;
                            num_polylines += 1;

                            x = (points[p1].x + points[p3].x) / 2f64;
                            y = (points[p1].y + points[p3].y) / 2f64;
                            kdtree
                                .add([x, y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(Point2D::new(x, y));
                            roof_segments.push([points_id[p1], points_id[p3]]);
                            node += 1;
                            kdtree
                                .add([centroid.x, centroid.y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(centroid);
                            roof_segments.push([points_id[p1], points_id[p3]]);
                            node += 1;
                            num_polylines += 1;

                            x = (points[p2].x + points[p3].x) / 2f64;
                            y = (points[p2].y + points[p3].y) / 2f64;
                            kdtree
                                .add([x, y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(Point2D::new(x, y));
                            roof_segments.push([points_id[p2], points_id[p3]]);
                            node += 1;
                            kdtree
                                .add([centroid.x, centroid.y], node)
                                .expect("Error adding point to kd-tree.");
                            endnodes.push(centroid);
                            roof_segments.push([points_id[p2], points_id[p3]]);
                            node += 1;
                            num_polylines += 1;
                        } else {
                            if points_id[p1] != points_id[p2] {
                                x = (points[p1].x + points[p2].x) / 2f64;
                                y = (points[p1].y + points[p2].y) / 2f64;
                                kdtree
                                    .add([x, y], node)
                                    .expect("Error adding point to kd-tree.");
                                endnodes.push(Point2D::new(x, y));
                                roof_segments.push([points_id[p1], points_id[p2]]);
                                node += 1;
                            }
                            if points_id[p1] != points_id[p3] {
                                x = (points[p1].x + points[p3].x) / 2f64;
                                y = (points[p1].y + points[p3].y) / 2f64;
                                kdtree
                                    .add([x, y], node)
                                    .expect("Error adding point to kd-tree.");
                                endnodes.push(Point2D::new(x, y));
                                roof_segments.push([points_id[p1], points_id[p3]]);
                                node += 1;
                            }
                            if points_id[p2] != points_id[p3] {
                                x = (points[p2].x + points[p3].x) / 2f64;
                                y = (points[p2].y + points[p3].y) / 2f64;
                                kdtree
                                    .add([x, y], node)
                                    .expect("Error adding point to kd-tree.");
                                endnodes.push(Point2D::new(x, y));
                                roof_segments.push([points_id[p2], points_id[p3]]);
                                node += 1;
                            }

                            num_polylines += 1;
                        }
                    }
                }

                // now trace each of the polygons
                let mut visited = vec![vec![false; 2]; roof_segments.len()];
                // let mut current_neighbour: usize;
                // let mut n: usize;
                // let smoothing_factor = 1.0;
                let mut geometries: Vec<(Vec<Point2D>, usize)> = vec![];
                for p in 0..num_polylines {
                    node = p * 2;
                    for a in 0..2 {
                        // is it a candidate for a trace?
                        if !visited[node][a] && roof_segments[node][a] != EMPTY {
                            seg_num = roof_segments[node][a];
                            // current_neighbour = roof_segments[node][(a+1)%2];
                            visited[node][a] = true;
                            // roof_segments[node][a] = EMPTY;
                            let mut line_points: Vec<Point2D> = vec![];
                            // let mut line_seg: Vec<Point2D> = vec![];
                            loop {
                                line_points.push(endnodes[node]);
                                // line_seg.push(endnodes[node]);

                                // find the next node
                                let mut next_node_found = false;
                                let other_node = if node % 2 == 0 { node + 1 } else { node - 1 }; // other side of the line

                                if roof_segments[other_node][0] == seg_num
                                    && !visited[other_node][0]
                                {
                                    node = other_node;
                                    next_node_found = true;
                                    visited[node][0] = true;

                                // n = roof_segments[node][1];
                                // if n != current_neighbour {
                                //     if line_seg.len() > 4 {
                                //         if line_seg[0].y > line_seg[line_seg.len()-1].y {
                                //             line_seg.reverse();
                                //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                //             line_seg.reverse();
                                //         } else {
                                //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                //         }
                                //     }
                                //     line_points.append(&mut line_seg);
                                //     current_neighbour = n;
                                // }
                                } else if roof_segments[other_node][1] == seg_num
                                    && !visited[other_node][1]
                                {
                                    node = other_node;
                                    next_node_found = true;
                                    visited[node][1] = true;

                                    // n = roof_segments[node][0];
                                    // if n != current_neighbour {
                                    //     if line_seg.len() > 4 {
                                    //         if line_seg[0].y > line_seg[line_seg.len()-1].y {
                                    //             line_seg.reverse();
                                    //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                    //             line_seg.reverse();
                                    //         } else {
                                    //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                    //         }
                                    //     }
                                    //     line_points.append(&mut line_seg);
                                    //     current_neighbour = n;
                                    // }
                                }

                                if !next_node_found {
                                    // see if there is a connected polyline's endnode with the same seg_num
                                    let ret = kdtree
                                        .within(
                                            &[endnodes[node].x, endnodes[node].y],
                                            precision,
                                            &squared_euclidean,
                                        )
                                        .expect("Error performing search on kd-tree.");
                                    for j in 0..ret.len() {
                                        node = *(ret[j].1);
                                        if roof_segments[node][0] == seg_num && !visited[node][0] {
                                            visited[node][0] = true;
                                            next_node_found = true;

                                            // n = roof_segments[node][1];
                                            // if n != current_neighbour {
                                            //     if line_seg.len() > 4 {
                                            //         if line_seg[0].y > line_seg[line_seg.len()-1].y {
                                            //             line_seg.reverse();
                                            //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                            //             line_seg.reverse();
                                            //         } else {
                                            //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                            //         }
                                            //     }
                                            //     line_points.append(&mut line_seg);
                                            //     current_neighbour = n;
                                            // }
                                            break;
                                        } else if roof_segments[node][1] == seg_num
                                            && !visited[node][1]
                                        {
                                            visited[node][1] = true;
                                            next_node_found = true;

                                            // n = roof_segments[node][0];
                                            // if n != current_neighbour {
                                            //     if line_seg.len() > 4 {
                                            //         if line_seg[0].y > line_seg[line_seg.len()-1].y {
                                            //             line_seg.reverse();
                                            //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                            //             line_seg.reverse();
                                            //         } else {
                                            //             line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                                            //         }
                                            //     }
                                            //     line_points.append(&mut line_seg);
                                            //     current_neighbour = n;
                                            // }
                                            break;
                                        }
                                    }
                                    if !next_node_found {
                                        break;
                                    }
                                }
                            }

                            // if line_seg.len() > 4 && !line_points[line_points.len()-1].nearly_equals(&line_points[0]) {
                            //     if line_seg[0].y > line_seg[line_seg.len()-1].y {
                            //         line_seg.reverse();
                            //         line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                            //         line_seg.reverse();
                            //     } else {
                            //         line_seg = algorithms::simplify_rdp(&line_seg, &smoothing_factor);
                            //     }
                            // }
                            // line_points.append(&mut line_seg);

                            // tan_slope = 0f64;
                            // if segment_plane[seg_num].c != 0f64 {
                            //     fx = -segment_plane[seg_num].a / segment_plane[seg_num].c;
                            //     if fx != 0f64 {
                            //         fy = -segment_plane[seg_num].b / segment_plane[seg_num].c;
                            //         tan_slope = (fx * fx + fy * fy).sqrt();
                            //     }
                            // }

                            if line_points[line_points.len() - 1].nearly_equals(&line_points[0]) {
                                line_points.push(line_points[0].clone());
                            }
                            geometries.push((line_points.clone(), seg_num));

                            // area = algorithms::polygon_area(&line_points);
                            // tan_slope = 0f64;
                            // aspect = -1f64;
                            // if segment_plane[seg_num].c != 0f64 {
                            //     fx = -segment_plane[seg_num].a / segment_plane[seg_num].c;
                            //     if fx != 0f64 {
                            //         fy = -segment_plane[seg_num].b / segment_plane[seg_num].c;

                            //         tan_slope = (fx * fx + fy * fy).sqrt();
                            //         aspect = (180f64 - ((fy / fx).atan()).to_degrees()
                            //             + 90f64 * (fx / (fx).abs()))
                            //         .to_radians();
                            //         term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                            //         term2 = sin_theta / tan_slope;
                            //         term3 = cos_theta * (azimuth - aspect).sin();
                            //         hillshade = term1 * (term2 - term3);
                            //     } else {
                            //         hillshade = 0.5;
                            //     }
                            //     hillshade = hillshade; // * 255f64;
                            //     if hillshade < 0f64 {
                            //         hillshade = 0f64;
                            //     }
                            // }

                            // if tan_slope.atan().to_degrees() < max_slope {
                            //     // otherwise, the points are likely co-linear or nearly so.
                            //     let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                            //     if line_points[line_points.len()-1].nearly_equals(&line_points[0]) {
                            //         // println!("I'm here {:?} {:?}", line_points[0], line_points[line_points.len()-1]);
                            //         line_points.push(line_points[0].clone());
                            //     }
                            //     if algorithms::is_clockwise_order(&line_points) {
                            //         sfg.add_part(&line_points);
                            //     } else {
                            //         line_points.reverse();
                            //         sfg.add_part(&line_points);
                            //     }
                            //     output.add_record(sfg);
                            //     if aspect != -1f64 {
                            //         aspect = aspect.to_degrees();
                            //     }
                            //     output.attributes.add_record(
                            //         vec![
                            //             FieldData::Int(fid),
                            //             FieldData::Int(seg_building_id[seg_num] as i32),
                            //             FieldData::Real(segment_max_elev[seg_num]),
                            //             FieldData::Real(hillshade),
                            //             FieldData::Real(tan_slope.atan().to_degrees()),
                            //             FieldData::Real(aspect),
                            //             FieldData::Real(area),
                            //             FieldData::Int(seg_num as i32),
                            //         ],
                            //         false,
                            //     );
                            //     fid += 1;
                            // }
                        }
                    }
                }

                // /*
                let mut already_added = vec![false; geometries.len()];
                for i in 0..geometries.len() {
                    if !already_added[i] {
                        // already_added[i] = true;
                        seg_num = geometries[i].1;

                        // find the index of the geometry with the largest area sharing this seg_num
                        let mut max_area = 0f64; // = algorithms::polygon_area(&geometries[i].0);
                        let mut max_area_idx = 0usize; // = i;
                        let mut num_parts = 0;
                        for j in 0..geometries.len() {
                            if geometries[j].1 == seg_num {
                                already_added[j] = true;
                                num_parts += 1;
                                area = algorithms::polygon_area(&geometries[j].0);
                                if area > max_area {
                                    max_area = area;
                                    max_area_idx = j;
                                }
                            }
                        }

                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                        if algorithms::is_clockwise_order(&geometries[max_area_idx].0) {
                            sfg.add_part(&geometries[max_area_idx].0);
                        } else {
                            geometries[max_area_idx].0.reverse();
                            sfg.add_part(&geometries[max_area_idx].0);
                        }

                        // /*
                        if num_parts > 1 {
                            // It's a multi-part geometry; the largest sized geometry is assumed to be the hull.
                            // Which of the other parts are holes?
                            for j in 0..geometries.len() {
                                if geometries[j].1 == seg_num && j != max_area_idx {
                                    // already_added[j] = true;

                                    // is contained within the hull?
                                    if algorithms::poly_in_poly(
                                        &geometries[j].0,
                                        &geometries[max_area_idx].0,
                                    ) {
                                        // it's a hole and should be in CCW order
                                        if !algorithms::is_clockwise_order(&geometries[j].0) {
                                            sfg.add_part(&geometries[j].0);
                                        } else {
                                            geometries[j].0.reverse();
                                            sfg.add_part(&geometries[j].0);
                                        }
                                        max_area -= algorithms::polygon_area(&geometries[j].0);
                                    } else {
                                        // it's not a hole and should in CW order
                                        if algorithms::is_clockwise_order(&geometries[j].0) {
                                            sfg.add_part(&geometries[j].0);
                                        } else {
                                            geometries[j].0.reverse();

                                            sfg.add_part(&geometries[j].0);
                                        }
                                        max_area += algorithms::polygon_area(&geometries[j].0);
                                    }
                                }
                            }
                        }
                        // */
                        // output this shape.
                        output.add_record(sfg);

                        tan_slope = 0f64;
                        aspect = -1f64;
                        if segment_plane[seg_num].c != 0f64 {
                            fx = -segment_plane[seg_num].a / segment_plane[seg_num].c;
                            if fx != 0f64 {
                                fy = -segment_plane[seg_num].b / segment_plane[seg_num].c;

                                tan_slope = (fx * fx + fy * fy).sqrt();
                                aspect = (180f64 - ((fy / fx).atan()).to_degrees()
                                    + 90f64 * (fx / (fx).abs()))
                                .to_radians();
                                term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                                term2 = sin_theta / tan_slope;
                                term3 = cos_theta * (azimuth - aspect).sin();
                                hillshade = term1 * (term2 - term3);
                            } else {
                                hillshade = 0.5;
                            }
                            hillshade = hillshade; // * 255f64;
                            if hillshade < 0f64 {
                                hillshade = 0f64;
                            }
                        }
                        tan_slope = tan_slope.atan().to_degrees();
                        if aspect != -1f64 {
                            aspect = aspect.to_degrees();
                        }

                        output.attributes.add_record(
                            vec![
                                FieldData::Int(fid),
                                FieldData::Int(seg_building_id[seg_num] as i32),
                                FieldData::Real(segment_max_elev[seg_num]),
                                FieldData::Real(hillshade),
                                FieldData::Real(tan_slope),
                                FieldData::Real(aspect),
                                FieldData::Real(max_area),
                                // FieldData::Int(seg_num as i32),
                            ],
                            false,
                        );
                        fid += 1;
                    }
                }
                // */
            }
            if verbose {
                progress = (100.0_f64 * building_id as f64 / num_buildings as f64) as i32;
                if progress != old_progress {
                    println!("Creating roof segment polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        /*
        let mut clrs: Vec<(u16, u16, u16)> = Vec::new();
        let mut rng = rand::thread_rng();
        let (mut r, mut g, mut b): (u16, u16, u16); // = (0u16, 0u16, 0u16);
        let range: Vec<u32> = (0..16777215).collect();
        let raw_clrs: Vec<u32> = range
            .choose_multiple(&mut rng, current_segment + 1)
            .cloned()
            .collect();
        for i in 0..current_segment + 1 as usize {
            r = (raw_clrs[i] as u32 & 0xFF) as u16;
            g = ((raw_clrs[i] as u32 >> 8) & 0xFF) as u16;
            b = ((raw_clrs[i] as u32 >> 16) & 0xFF) as u16;

            clrs.push((r, g, b));
        }

        for point_num in 0..n_points {
            let p = las_points[point_num].get_point_data();
            seg_num = segment_id[point_num];
            if seg_num > 1 {
                // don't include the non-planar points
                let rgb: ColourData = ColourData {
                    red: clrs[seg_num].0,
                    green: clrs[seg_num].1,
                    blue: clrs[seg_num].2,
                    nir: 0u16,
                };
                let lpr: LidarPointRecord = LidarPointRecord::PointRecord2 {
                    point_data: p,
                    colour_data: rgb,
                };
                output_lidar.add_point_record(lpr);
                if verbose {
                    progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }
        */

        if verbose {
            println!("Saving polygon data...");
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        // let _ = match output_pts.write() {
        //     Ok(_) => {
        //         if verbose {
        //             println!("Output file written")
        //         }
        //     }
        //     Err(e) => return Err(e),
        // };

        /*
        if output_lidar.header.number_of_points > 0 {
            let _ = match output_lidar.write() {
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
        */

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

// Equation of plane:
// ax + by + cz + d = 0
#[derive(Default, Clone, Copy)]
struct Plane {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Plane {
    fn new(a: f64, b: f64, c: f64, d: f64) -> Plane {
        Plane {
            a: a,
            b: b,
            c: c,
            d: d,
        }
    }

    fn zero() -> Plane {
        Plane {
            a: 0f64,
            b: 0f64,
            c: 0f64,
            d: 0f64,
        }
    }

    // fn is_zero(&self) -> bool {
    //     if self.a == 0f64 && self.b == 0f64 && self.c == 0f64 && self.d == 0f64 {
    //         return true;
    //     }

    //     false
    // }

    // Constructs a plane from a collection of points
    // so that the summed squared distance to all points is minimized
    fn from_points(points: &Vec<Vector3<f64>>) -> Plane {
        let n = points.len();
        // assert!(n >= 3, "At least three points required");
        if n < 3 {
            return Plane::zero();
        }

        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        for p in points {
            sum = sum + *p;
        }
        let centroid = sum * (1.0 / (n as f64));

        // Calc full 3x3 covariance matrix, excluding symmetries:
        let mut xx = 0.0;
        let mut xy = 0.0;
        let mut xz = 0.0;
        let mut yy = 0.0;
        let mut yz = 0.0;
        let mut zz = 0.0;

        for p in points {
            let r = p - &centroid;
            xx += r.x * r.x;
            xy += r.x * r.y;
            xz += r.x * r.z;
            yy += r.y * r.y;
            yz += r.y * r.z;
            zz += r.z * r.z;
        }

        let det_x = yy * zz - yz * yz;
        let det_y = xx * zz - xz * xz;
        let det_z = xx * yy - xy * xy;

        let det_max = det_x.max(det_y).max(det_z);

        // Pick path with best conditioning:
        let (mut a, mut b, mut c) = if det_max == det_x {
            (
                1.0,
                (xz * yz - xy * zz) / det_x,
                (xy * yz - xz * yy) / det_x,
            )
        } else if det_max == det_y {
            (
                (yz * xz - xy * zz) / det_y,
                1.0,
                (xy * xz - yz * xx) / det_y,
            )
        } else {
            (
                (yz * xy - xz * yy) / det_z,
                (xz * xy - yz * xx) / det_z,
                1.0,
            )
        };

        // Derive the plane from the a,b,c normal and the centroid (x0, y0, z0)
        // a(x−x0)+b(y−y0)+c(z−z0)=0
        // d = -a*x0 + -b*y0 + -c*z0

        let norm = (a * a + b * b + c * c).sqrt();
        a /= norm;
        b /= norm;
        c /= norm;
        let d = -a * centroid.x + -b * centroid.y + -c * centroid.z;
        Plane::new(a, b, c, d)
    }

    // // solves for the value of z at point (x0,y0)
    // // z = -(d + ax + by) / c
    // fn solve_xy(&self, x0: f64, y0: f64) -> Option<f64> {
    //     if self.c != 0f64 {
    //         return Some(-(self.d + self.a * x0 + self.b * y0) / self.c);
    //     }
    //     None
    // }

    // calculates the residual z value at point (x0,y0,z0)
    // z = -(d + ax0 + by0) / c
    // residual = z0 - z
    fn residual(&self, p: &Vector3<f64>) -> f64 {
        // let z = -(self.d + self.a*p.x + self.b*p.y) / self.c;
        // p.z - z

        // We need to use the reduced major axis distance instead of z residuals because the later can't handle a
        // vertical plane, of which there may be many in a point cloud.
        (self.a * p.x + self.b * p.y + self.c * p.z + self.d).abs() / self.norm_length()
    }

    fn rmse(&self, points: &Vec<Vector3<f64>>) -> f64 {
        let mut rmse = 0f64;
        let mut z: f64;
        // for p in points {
        //     z = -(self.d + self.a*p.x + self.b*p.y) / self.c;
        //     rmse += (p.z - z)*(p.z - z);
        // }
        // (rmse / points.len() as f64).sqrt()

        // We need to use the reduced major axis distance instead of z residuals because the later can't handle a
        // vertical plane, of which there may be many in a point cloud.
        let norm = self.norm_length();
        for p in points {
            z = (self.a * p.x + self.b * p.y + self.c * p.z + self.d) / norm;
            rmse += z * z;
        }
        (rmse / points.len() as f64).sqrt()
    }

    fn norm_length(&self) -> f64 {
        (self.a * self.a + self.b * self.b + self.c * self.c).sqrt()
    }

    fn slope(&self) -> f64 {
        // (self.a*self.a + self.b*self.b).sqrt().atan().to_degrees()
        self.c.abs().acos().to_degrees()
    }

    fn angle_between(self, other: Plane) -> f64 {
        let numerator = self.a * other.a + self.b * other.b + self.c * other.c;
        let denom1 = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        let denom2 = (other.a * other.a + other.b * other.b + other.c * other.c).sqrt();
        if denom1 * denom2 != 0f64 {
            return (numerator / (denom1 * denom2)).acos();
        }
        f64::NEG_INFINITY
    }
}

impl AddAssign for Plane {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c + other.c,
            d: self.d + other.d,
        };
    }
}

// impl SubAssign for Plane {
//     fn sub_assign(&mut self, other: Self) {
//         *self = Self {
//             a: self.a - other.a,
//             b: self.b - other.b,
//             c: self.c - other.c,
//             d: self.d - other.d,
//         };
//     }
// }
