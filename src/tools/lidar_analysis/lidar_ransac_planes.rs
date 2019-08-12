/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/07/2019
Last Modified: 23/07/2019
License: MIT
*/
// extern crate kdtree;

use self::na::Vector3;
use crate::lidar::*;
use crate::na;
use crate::structures::{DistanceMetric, FixedRadiusSearch3D};
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

/// This tool uses the [random sample concensus (RANSAC)](https://en.wikipedia.org/wiki/Random_sample_consensus) 
/// method to identify points within a LiDAR point cloud that belong to linear planes. RANSAC is a common
/// method used in the field of computer vision to identify a subset of inliner points in a noisy data set 
/// containing abundant outlier points. Because LiDAR point clouds often contain vegetation points that do not
/// form planar surfaces, this tool can be used to largely strip vegetation points from the point cloud, leaving
/// behind the ground returns, buildings, and other points belonging to planar surfaces. If the `--classify` flag
/// is used, non-planar points will not be removed but rather will be assigned a different class (1) than the 
/// planar points (0).
/// 
/// # See Also
/// `LidarGroundPointFilter`
pub struct LidarRansacPlanes {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarRansacPlanes {
    pub fn new() -> LidarRansacPlanes {
        // public constructor
        let name = "LidarRansacPlanes".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Removes outliers (high and low points) in a LiDAR point cloud.".to_string();

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
            default_value: Some("5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Inliner Threshold".to_owned(),
            flags: vec!["--threshold".to_owned()],
            description: "Threshold used to determine inliner points.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.35".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Acceptable Model Size".to_owned(),
            flags: vec!["--model_size".to_owned()],
            description: "Acceptable model size.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("8".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Classify Points".to_owned(),
            flags: vec!["--classify".to_owned()],
            description: "Classify points as ground (2) or off-ground (1).".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0 --num_iter=10 --num_samples=5 --threshold=0.25", short_exe, name).replace("*", &sep);

        LidarRansacPlanes {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarRansacPlanes {
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
        let mut num_iter = 30;
        let mut num_samples = 10;
        let mut threshold = 0.15;
        let mut acceptable_model_size = 30;
        let mut filter = true;

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
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-num_iter" {
                num_iter = if keyval {
                    vec[1].to_string().parse::<usize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<usize>().unwrap()
                };
            } else if flag_val == "-num_samples" {
                num_samples = if keyval {
                    vec[1].to_string().parse::<usize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<usize>().unwrap()
                };
            } else if flag_val == "-threshold" {
                threshold = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-model_size" {
                acceptable_model_size = if keyval {
                    vec[1].to_string().parse::<usize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<usize>().unwrap()
                };
            } else if flag_val == "-classify" {
                filter = false;
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        if acceptable_model_size < num_samples {
            acceptable_model_size = num_samples;
            if verbose {
                println!("Warning: The --model_size parameter must be equal to or larger than num_samples.");
            }
        }

        let start = Instant::now();

        if verbose {
            println!("Performing analysis...");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch3D<usize> = FixedRadiusSearch3D::new(search_radius, DistanceMetric::SquaredEuclidean);
        for (i, p) in (&input).into_iter().enumerate() {
            frs.insert(p.x, p.y, p.z, i);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Adding points to search tree: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let dimensions = 2;
        // let capacity_per_node = 64;
        // let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);
        // for (i, p) in (&input).into_iter().enumerate() {
        //     kdtree.add([p.x, p.y], i).unwrap();
        //     if verbose {
        //         progress = (100.0_f64 * i as f64 / num_points) as i32;
        //         if progress != old_progress {
        //             println!("Adding points to search tree: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

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
                let mut p1: PointData;
                let mut p2: PointData;
                let mut index: usize;
                let mut rng = &mut rand::thread_rng();

                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    p1 = input.get_point_info(point_num);
                    let ret = frs.search(p1.x, p1.y, p1.z);
                    // let ret = kdtree
                    //             .within(&[p1.x, p1.y], search_radius, &squared_euclidean)
                    //             .unwrap();
                    n = ret.len();
                    let mut points: Vec<Vector3<f64>> = Vec::with_capacity(n);
                    let mut model: Plane;
                    let mut better_model: Plane;
                    let mut best_model: Plane = Plane{..Default::default()};
                    let mut rmse: f64;
                    let mut min_rmse: f64;
                    // let mut best_model_num_points = 0;
                    let mut model_found = false;
                    if n > num_samples {
                        for j in 0..n {
                            index = ret[j].0;
                            // index = *ret[j].1;
                            p2 = input.get_point_info(index);
                            points.push(Vector3::new(p2.x, p2.y, p2.z));
                        }

                        min_rmse = f64::MAX;
                        let v: Vec<usize> = (0..n).collect();
                        for _ in 0..num_iter {
                            // select n random samples.
                            let samples: Vec<usize> = v.choose_multiple(&mut rng, num_samples).cloned().collect();
                            let data: Vec<Vector3<f64>> = samples.into_iter().map(|a| { points[a] } ).collect();
                            // get the best-fit plane
                            model = plane_from_points(&data);
                            let mut inliners: Vec<Vector3<f64>> = Vec::with_capacity(n);
                            for j in 0..n {
                                if model.residual(points[j]).abs() <= threshold {
                                    inliners.push(points[j]);
                                }
                            }
                            if inliners.len() >= acceptable_model_size {
                                better_model = plane_from_points(&inliners);
                                rmse = better_model.rmse(&inliners);
                                if rmse < min_rmse {
                                    min_rmse = rmse;
                                    best_model = better_model;
                                    model_found = true;
                                    if min_rmse == 0f64 {
                                        // You can't get any better than that.
                                        break;
                                    }
                                }
                                // if inliners.len() > best_model_num_points {
                                //     best_model_num_points = inliners.len();
                                //     best_model = better_model;
                                // }
                            }
                        }
                    }
                    if model_found {
                        if best_model.residual(Vector3::new(p1.x, p1.y, p1.z)) <= threshold {
                            tx.send((point_num, true)).unwrap();
                        } else {
                            tx.send((point_num, false)).unwrap();
                        }
                    } else {
                        tx.send((point_num, false)).unwrap();
                    }
                }
            });
        }

        let mut is_a_planar_surface = vec![false; n_points];
        for i in 0..n_points {
            let data = rx.recv().unwrap();
            is_a_planar_surface[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();
        let mut num_points_filtered = 0;
        if filter {
            for i in 0..n_points {
                if is_a_planar_surface[i] {
                    output.add_point_record(input.get_record(i));
                } else {
                    num_points_filtered += 1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // classify
            for point_num in 0..n_points {
                let class_val = match is_a_planar_surface[point_num] {
                    true => 0,
                    false => 1,
                };
                let pr = input.get_record(point_num);
                let pr2: LidarPointRecord;
                match pr {
                    LidarPointRecord::PointRecord0 { mut point_data } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord0 {
                            point_data: point_data,
                        };
                    }
                    LidarPointRecord::PointRecord1 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord1 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord2 {
                        mut point_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord2 {
                            point_data: point_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord3 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord3 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord4 {
                        mut point_data,
                        gps_data,
                        wave_packet,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord4 {
                            point_data: point_data,
                            gps_data: gps_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord5 {
                        mut point_data,
                        gps_data,
                        colour_data,
                        wave_packet,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord5 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord6 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord6 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord7 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord7 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord8 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord8 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord9 {
                        mut point_data,
                        gps_data,
                        wave_packet,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord9 {
                            point_data: point_data,
                            gps_data: gps_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord10 {
                        mut point_data,
                        gps_data,
                        colour_data,
                        wave_packet,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord10 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                }
                output.add_point_record(pr2);
                if verbose {
                    progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            num_points_filtered = 1; // so it passes the saving
        }
        
        let elapsed_time = get_formatted_elapsed_time(start);

        if num_points_filtered == 0 {
            println!("Warning: No outlier points were filtered from the point cloud.");
        } else if num_points_filtered == n_points {
            println!("Warning: All of the points have been filtered from the point cloud.")
        }

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
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

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
#[inline]
fn plane_from_points(points: &Vec<Vector3<f64>>) -> Plane {
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
    let (a, b, c) = if det_max == det_x {
        (1.0,
        (xz * yz - xy * zz) / det_x,
        (xy * yz - xz * yy) / det_x)
        // Vector3::new(1.0, a, b)
    } else if det_max == det_y {
        ((yz * xz - xy * zz) / det_y,
        1.0,
        (xy * xz - yz * xx) / det_y)
        // Vector3::new(a, 1.0, b)
    } else {
        ((yz * xy - xz * yy) / det_z,
        (xz * xy - yz * xx) / det_z,
        1.0)
        // Vector3::new(a, b, 1.0)
    };

    // Derive the plane from the a,b,c normal and the centroid (x0, y0, z0)

    // a(x−x0)+b(y−y0)+c(z−z0)=0
    // d = -a*x0 + -b*y0 + -c*z0
    let d = -a*centroid.x + -b*centroid.y + -c*centroid.z;

    Plane::new(a, b, c, d)
    // normalize(dir)
}

// #[inline]
// fn normalize(v: Vector3<f64>) -> Vector3<f64> {
//     let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
//     Vector3::new(v.x / norm, v.y / norm, v.z / norm)
// }

// Equation of plane:
// ax + by + cz + d = 0
#[derive(Default, Clone, Copy)]
struct Plane {
    a: f64,
    b: f64,
    c: f64,
    d: f64
}

impl Plane {
    fn new(a: f64, b: f64, c: f64, d: f64) -> Plane {
        Plane { a: a, b: b, c: c, d: d }
    }

    fn zero() -> Plane {
        Plane { a: 0f64, b: 0f64, c: 0f64, d: 0f64 }
    }

    // // solves for the value of z at point (x0,y0)
    // // z = -(d + ax + by) / c
    // fn solve_xy(&self, x0: f64, y0: f64) -> f64 {
    //     -(self.d + self.a*x0 + self.b*y0) / self.c
    // }

    // calculates the residual z value at point (x0,y0,z0)
    // z = -(d + ax0 + by0) / c
    // residual = z0 - z
    fn residual(&self, p: Vector3<f64>) -> f64 {
        let z = -(self.d + self.a*p.x + self.b*p.y) / self.c;
        p.z - z
    }

    fn rmse(&self, points: &Vec<Vector3<f64>>) -> f64 {
        let mut rmse = 0f64;
        let mut z: f64;
        for p in points {
            z = -(self.d + self.a*p.x + self.b*p.y) / self.c;
            rmse += (p.z - z)*(p.z - z);
        }
        (rmse / points.len() as f64).sqrt()
    }
}
