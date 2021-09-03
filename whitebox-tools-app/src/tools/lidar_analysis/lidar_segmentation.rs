/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 05/12/2017
Last Modified: 12/01/2020
License: MIT
*/
// extern crate kdtree;

use self::na::Vector3;
use whitebox_lidar::*;
use crate::na;
use whitebox_common::structures::{DistanceMetric, FixedRadiusSearch3D, Point3D};
use crate::tools::*;
use rand::seq::SliceRandom;
// use kdtree::distance::squared_euclidean;
// use kdtree::KdTree;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to segment a LiDAR point cloud based on differences in the orientation of fitted planar
/// surfaces and point proximity. The algorithm begins by attempting to fit planar surfaces to all of the points within
/// a user-specified radius (`--radius`) of each point in the LiDAR data set. The planar equation is stored for each
/// point for which a suitable planar model can be fit. A region-growing algorithm is then used to assign nearby points
/// with similar planar models. Similarity is based on a maximum allowable angular difference (in degrees) between the
/// two neighbouring points' plane normal vectors (`--norm_diff`). The `--norm_diff` parameter can therefore be thought
/// of as a way of specifying the magnitude of edges mapped by the region-growing algorithm. By setting this value
/// appropriately, it is possible to segment each facet of a building's roof. Segment edges for planar points may also
/// be determined by a maximum allowable height difference (`--maxzdiff`) between neighbouring points on the same plane.
/// Points for which no suitable planar model can be fit are assigned to 'volume' (non-planar) segments (e.g. vegetation
/// points) using a region-growing method that connects neighbouring points based solely on proximity (i.e. all volume
/// points within `radius` distance are considered to belong to the same segment).
///
/// The resulting point cloud will have both planar segments (largely ground surfaces and building roofs and walls) and
/// volume segments (largely vegetation). Each segment is assigned a random red-green-blue (RGB) colour in the output LAS
/// file. The largest segment in any airborne LiDAR dataset will usually belong to the ground surface. This largest segment
/// will always be assigned a dark-green RGB of (25, 120, 0) by the tool.
///
/// This tool uses the [random sample consensus (RANSAC)](https://en.wikipedia.org/wiki/Random_sample_consensus)
/// method to identify points within a LiDAR point cloud that belong to planar surfaces. RANSAC is a common
/// method used in the field of computer vision to identify a subset of inlier points in a noisy data set
/// containing abundant outlier points. Because LiDAR point clouds often contain vegetation points that do not
/// form planar surfaces, this tool can be used to largely strip vegetation points from the point cloud, leaving
/// behind the ground returns, buildings, and other points belonging to planar surfaces. If the `--classify` flag
/// is used, non-planar points will not be removed but rather will be assigned a different class (1) than the
/// planar points (0).
///
/// The algorithm selects a random sample, of a specified size (`--num_samples`) of the points from within the
/// neighbourhood (`--radius`) surrounding each LiDAR point. The sample is then used to parameterize a planar
/// best-fit model. The distance between each neighbouring point and the plane is then evaluated; inliers are
/// those neighbouring points within a user-specified distance threshold (`--threshold`). Models with at least
/// a minimum number of inlier points (`--model_size`) are then accepted. This process of selecting models is
/// iterated a number of user-specified times (`--num_iter`).
///
/// One of the challenges with identifying planar surfaces in LiDAR point clouds is that these data are usually
/// collected along scan lines. Therefore, each scan line can potentially yield a vertical planar surface, which
/// is one reason that some vegetation points may be assigned to planes during the RANSAC plane-fitting method. To cope
/// with this problem, the tool allows the user to specify a maximum planar slope (`--max_slope`) parameter.
/// Planes that have slopes greater than this threshold are rejected by the algorithm. This has the side-effect
/// of removing building walls however.
///
/// ![](../../doc_img/LidarSegmentation.png)
///
/// # References
/// Fischler MA and Bolles RC. 1981. Random sample consensus: a paradigm for model fitting with applications
/// to image analysis and automated cartography. Commun. ACM, 24(6):381–395.
///
/// # See Also
/// `LidarRansacPlanes`, `LidarGroundPointFilter`
pub struct LidarSegmentation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarSegmentation {
    pub fn new() -> LidarSegmentation {
        // public constructor
        let name = "LidarSegmentation".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Segments a LiDAR point cloud based on differences in the orientation of fitted planar surfaces and point proximity.".to_string();

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
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
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
            description: "Threshold used to determine inlier points.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.15".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Acceptable Model Size".to_owned(),
            flags: vec!["--model_size".to_owned()],
            description: "Acceptable model size.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("15".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Planar Slope".to_owned(),
            flags: vec!["--max_slope".to_owned()],
            description: "Maximum planar slope.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("80.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Normal Difference Threshold".to_owned(),
            flags: vec!["--norm_diff".to_owned()],
            description: "Maximum difference in normal vectors, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Maximum Elevation Difference Between Points".to_owned(), 
            flags: vec!["--maxzdiff".to_owned()], 
            description: "Maximum difference in elevation (z units) between neighbouring points of the same segment.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Don't cross class boundaries?".to_owned(),
            flags: vec!["--classes".to_owned()],
            description: "Segments don't cross class boundaries.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Classify largest segment as ground?".to_owned(),
            flags: vec!["--ground".to_owned()],
            description: "Classify the largest segment as ground points?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0 --num_iter=10 --num_samples=5 --threshold=0.25 --max_slope=70.0", short_exe, name).replace("*", &sep);

        LidarSegmentation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarSegmentation {
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
        let mut search_radius = 2f64;
        let mut num_iter = 50;
        let mut num_samples = 10;
        let mut threshold = 0.15;
        let mut acceptable_model_size = 30;
        let mut max_slope = 75f64;
        let mut max_norm_diff = 2f64;
        let mut max_z_diff = 1f64;
        let mut dont_cross_class_boundaries = false;
        let mut ground_class = false;

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
                input_file = if keyval {
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
            } else if flag_val == "-classes" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    dont_cross_class_boundaries = true;
                }
            } else if flag_val == "-ground" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    ground_class = true;
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

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("reading input LiDAR file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
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

        let start = Instant::now();

        if verbose {
            println!("Performing analysis...");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        // Read the points into a fixed radius search
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch3D<usize> =
            FixedRadiusSearch3D::new(search_radius, DistanceMetric::SquaredEuclidean);
        // for (i, p) in (&input).into_iter().enumerate() {
        for i in 0..n_points {
            let p = input.get_transformed_coords(i);
            let pd = input.get_point_info(i);
            if !pd.withheld() && !pd.is_classified_noise() {
                frs.insert(p.x, p.y, p.z, i);
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Adding points to search tree: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let frs = Arc::new(frs); // wrap FRS in an Arc
                                 // let kdtree = Arc::new(kdtree);
        let input = Arc::new(input); // wrap input in an Arc
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            // let kdtree = kdtree.clone();
            let input = input.clone();
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
                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let mut best_model: Plane = Plane::zero();
                    // find the best fitting planar model that contains this point
                    // p1 = input.get_point_info(point_num);
                    p1 = input.get_transformed_coords(point_num);
                    let pd1 = input[point_num];
                    if !pd1.withheld() && !pd1.is_classified_noise() {
                        center_point = Vector3::new(p1.x, p1.y, p1.z);
                        let ret = frs.search(p1.x, p1.y, p1.z);
                        n = ret.len();
                        let mut points: Vec<Vector3<f64>> = Vec::with_capacity(n);
                        let mut model_found = false;
                        let mut model_points: Vec<usize> = Vec::with_capacity(n);
                        if n > larger_of_two_samples {
                            for j in 0..n {
                                index = ret[j].0;
                                // index = *ret[j].1;
                                // p2 = input.get_point_info(index);
                                p2 = input.get_transformed_coords(index);
                                points.push(Vector3::new(p2.x, p2.y, p2.z));
                            }

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
                    } else {
                        let model_points: Vec<usize> = vec![];
                        tx.send((best_model, f64::MAX, model_points)).unwrap();
                    }
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

        ////////////////////////////////////////
        // Perform the segmentation operation //
        ////////////////////////////////////////
        if verbose {
            println!("Segmenting the point cloud...");
        }
        let mut p: Point3D;
        let mut pd: PointData;
        let mut pn: Point3D;
        let mut pdn: PointData;
        let mut segment_id = vec![0usize; n_points];
        let mut current_segment = 0usize;
        let mut point_id: usize;
        let mut norm_diff: f64;
        let mut height_diff: f64;
        let mut index: usize;
        let mut solved_points = 0;
        let mut stack = vec![];
        let mut last_seed = 0;
        let mut is_planar: bool;
        let mut is_planar_n: bool;
        while solved_points < n_points {
            // Find a seed-point for a segment
            for i in last_seed..n_points {
                if segment_id[i] == 0 {
                    // No segment ID has yet been assigned to this point.
                    pd = input.get_point_info(i);
                    if !pd.withheld() && !pd.is_classified_noise() {
                        current_segment += 1;
                        segment_id[i] = current_segment;
                        stack.push(i);
                        last_seed = i;
                        break;
                    } else {
                        solved_points += 1;
                        current_segment += 1;
                        segment_id[i] = current_segment;
                    }
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
                pd = input.get_point_info(point_id);
                p = input.get_transformed_coords(point_id);
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
                        if is_planar == is_planar_n {
                            pdn = input.get_point_info(index);
                            pn = input.get_transformed_coords(index);

                            if (!dont_cross_class_boundaries
                                || (pd.classification() == pdn.classification()))
                                && !pdn.withheld()
                                && !pdn.is_classified_noise()
                            {
                                if is_planar {
                                    height_diff = (pn.z - p.z).abs();
                                    if height_diff < max_z_diff {
                                        // check the norm diff angle
                                        norm_diff = planes[point_id].angle_between(planes[index]);
                                        if norm_diff < max_norm_diff {
                                            segment_id[index] = current_segment;
                                            stack.push(index);
                                        }
                                    }
                                } else {
                                    // they can be grouped simply based on proximity
                                    segment_id[index] = current_segment;
                                    stack.push(index);
                                }
                            }
                        }
                    }
                }
            }
        }

        /////////////////////
        // Output the data //
        /////////////////////

        if verbose {
            println!("Saving data...");
        }

        let mut segment_size = vec![0usize; current_segment + 1];
        let mut seg_val: usize;
        let mut largest_size = 0usize;
        let mut largest_segment = 0usize;
        for point_num in 0..n_points {
            seg_val = segment_id[point_num];
            segment_size[seg_val] += 1;
            if segment_size[seg_val] > largest_size {
                largest_size = segment_size[seg_val];
                largest_segment = seg_val;
            }
        }

        let mut clrs: Vec<(u16, u16, u16)> = Vec::new();
        let mut rng = rand::thread_rng();
        let (mut r, mut g, mut b): (u16, u16, u16); // = (0u16, 0u16, 0u16);
        let range: Vec<u32> = (0..16777215).collect();
        let raw_clrs: Vec<u32> = range
            .choose_multiple(&mut rng, current_segment + 1)
            .cloned()
            .collect();
        for i in 0..current_segment + 1 as usize {
            if i != largest_segment {
                r = (raw_clrs[i] as u32 & 0xFF) as u16;
                g = ((raw_clrs[i] as u32 >> 8) & 0xFF) as u16;
                b = ((raw_clrs[i] as u32 >> 16) & 0xFF) as u16;
            } else {
                // ground segment; colour it dark green
                r = 25;
                g = 120;
                b = 0;
            }

            clrs.push((r, g, b));
        }

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.point_format = 2;
        for point_num in 0..n_points {
            let mut p: PointData = input[point_num];
            seg_val = segment_id[point_num];
            if ground_class && seg_val == largest_segment {
                p.set_classification(2);
            }
            let rgb: ColourData = ColourData {
                red: clrs[seg_val].0,
                green: clrs[seg_val].1,
                blue: clrs[seg_val].2,
                nir: 0u16,
            };
            let lpr: LidarPointRecord = LidarPointRecord::PointRecord2 {
                point_data: p,
                colour_data: rgb,
            };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Complete!")
                }
            }
            Err(e) => println!("error while writing: {:?}", e),
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

// impl AddAssign for Plane {
//     fn add_assign(&mut self, other: Self) {
//         *self = Self {
//             a: self.a + other.a,
//             b: self.b + other.b,
//             c: self.c + other.c,
//             d: self.d + other.d,
//         };
//     }
// }

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
