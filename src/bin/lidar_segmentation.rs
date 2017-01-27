extern crate whitebox_tools;
extern crate nalgebra as na;
extern crate kdtree;
extern crate rand;

use std::env;
use std::f64;
use std::cmp;
use std::path;
use std::default::Default;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;
use whitebox_tools::structures::fixed_radius_search::FixedRadiusSearch;
use na::{ Dot, Vector3 };
use kdtree::KdTree;
use kdtree::distance::squared_euclidean;
use rand::Rng;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut search_dist: f64 = 5.0;
    let mut num_neighbouring_points: usize = 10;
    let mut max_normal_angle = 2.0f64;
    let mut max_z_diff = 1.0;
    let mut verbose: bool = false;
    let mut variable_dist = true;
    let mut detrend_surface = 0.0f64;
    let mut classify_ground = false;

    // read the arguments
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        let mut keyval = false;
        if vec.len() > 1 { keyval = true; }
        if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--i" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--o" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-dist" || vec[0].to_lowercase() == "--dist" {
            variable_dist = false;
            if keyval {
                search_dist = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                search_dist = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-num_points" || vec[0].to_lowercase() == "--num_points" {
            if keyval {
                num_neighbouring_points = vec[1].to_string().parse::<usize>().unwrap();
            } else {
                num_neighbouring_points = args[i+1].to_string().parse::<usize>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-max_norm_angle" || vec[0].to_lowercase() == "--max_norm_angle" {
            if keyval {
                max_normal_angle = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                max_normal_angle = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-maxzdiff" || vec[0].to_lowercase() == "--maxzdiff" {
            if keyval {
                max_z_diff = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                max_z_diff = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-detrend" || vec[0].to_lowercase() == "--detrend" {
            if keyval {
                detrend_surface = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                detrend_surface = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-classify_ground" || vec[0].to_lowercase() == "--classify_ground" {
            classify_ground = true;
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
            s.push_str("-i               Input LAS file.\n");
            s.push_str("-o               Output LAS file.\n");
            s.push_str("-wd              Optional working directory. If specified, input and output filenames need not include a full path.\n");
            s.push_str("-dist            Optional search distance in xy units; default is variable, determined by num_points.\n");
            s.push_str("-num_points      Number (integer) of nearest-neighbour points used for plane fitting; default is 10.\n");
            s.push_str("-max_norm_angle  Maximum deviation (degrees) in normal vectors between neighbouring points of the same segment; default is 2.0.\n");
            s.push_str("-maxzdiff        Maximum difference in elevation (z units) between neighbouring points of the same segment; defuault is 1.0.\n");
            s.push_str("-classify_ground Optional mode. Surface in contact with the opening surface will be classified as ground points.");
            s.push_str("-v               Optional verbose mode. Tool will report progress if this flag is provided.\n");
            s.push_str("-version         Prints the tool version number.\n");
            s.push_str("-h               Prints help information.\n\n");
            s.push_str("Example usage:\n\n");
            s.push_str(&">> .*lidar_segmentation -wd *path*to*data* -i input.las -o output.las -num_points 15 -max_norm_angle 3.5 -max_z_diff 0.5 -v\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    let sep = std::path::MAIN_SEPARATOR;
    if !working_directory.ends_with(sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }

    lidar_segmentation(input_file, output_file, search_dist, num_neighbouring_points,
        max_normal_angle, max_z_diff, verbose, variable_dist, detrend_surface, classify_ground);
}

fn lidar_segmentation(input_file: String, output_file: String, search_dist: f64, num_neighbouring_points: usize,
                      mut max_angle: f64, mut max_z_diff: f64, verbose: bool, variable_dist: bool,
                      detrend_surface: f64, classify_ground: bool) {
    if verbose {
        println!("*********************************");
        println!("* Welcome to lidar_segmentation *");
        println!("*********************************");
    }

    max_angle = max_angle.to_radians();
    max_z_diff = max_z_diff * max_z_diff;

    if verbose { println!("Reading input LAS file..."); }
    //let input = las::LasFile::new(&input_file, "r");
    let input = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error reading file {}: {}", input_file, err),
    };

    if verbose { println!("Performing analysis..."); }

    let dimensions = 3;
    let capacity_per_node = 128;
    let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let mut progress: i32;
    let mut old_progress: i32 = -1;

    let mut residuals = vec![0.0f64; n_points];
    if detrend_surface > 0.0 {
        let mut zn: f64;
        let mut index_n: usize;
        let mut neighbourhood_min = vec![f64::INFINITY; n_points];
        let mut neighbourhood_max_min = vec![f64::NEG_INFINITY; n_points];

        if !variable_dist {
            let mut frs: FixedRadiusSearch<usize> = FixedRadiusSearch::new(detrend_surface);
            for i in 0..n_points {
                let p: PointData = input.get_point_info(i);
                frs.insert(p.x, p.y, i);
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Binning points: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            for i in 0..n_points {
                let p: PointData = input.get_point_info(i);
                let ret = frs.search(p.x, p.y);
                for j in 0..ret.len() {
                    index_n = ret[j].0;
                    let pn: PointData = input.get_point_info(index_n);
                    zn = pn.z;
                    if zn < neighbourhood_min[i] {
                        neighbourhood_min[i] = zn;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Performing erosion: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            for i in 0..n_points {
                let p: PointData = input.get_point_info(i);
                let ret = frs.search(p.x, p.y);
                for j in 0..ret.len() {
                    index_n = ret[j].0;
                    if neighbourhood_min[index_n] > neighbourhood_max_min[i] {
                        neighbourhood_max_min[i] = neighbourhood_min[index_n];
                    }
                }
                residuals[i] = p.z - neighbourhood_max_min[i];
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Performing dilation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {

        }
    } else {
        for i in 0..n_points {
            let p: PointData = input[i];
            residuals[i] = p.z;
        }
    }

    let mut frs: FixedRadiusSearch<usize> = FixedRadiusSearch::new(search_dist);

    if !variable_dist {
        // use a fixed radius search
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            frs.insert(p.x, p.y, i);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            let coords: [f64; 3] = [ p.x, p.y, p.z ];
            kdtree.add(coords.clone(), i).unwrap();
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Creating tree: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    // let search_dist_sqrd = search_dist * search_dist;

    let mut index_n: usize;
    let mut normal_vectors: Vec<Vector3<f64>> = vec![];
    if !variable_dist {
        // use a fixed radius search
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            let ret = frs.search(p.x, p.y);
            let mut data: Vec<Vector3<f64>> = vec![];;
            for j in 0..ret.len() {
                index_n = ret[j].0;
                let p2: PointData = input.get_point_info(index_n);
                data.push(Vector3 { x: p2.x, y: p2.y, z: residuals[index_n]}); //p2.z });
            }
            normal_vectors.push(plane_from_points(&data));
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Calculating point normals: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbouring_points, &squared_euclidean).unwrap();
            let mut data: Vec<Vector3<f64>> = vec![];;
            for j in 0..ret.len() {
                index_n = *ret[j].1;
                let p2: PointData = input.get_point_info(index_n);
                data.push(Vector3 { x: p2.x, y: p2.y, z: residuals[index_n]}); //p2.z });
            }
            normal_vectors.push(plane_from_points(&data));
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Calculating point normals: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    // Sort the data based on elevation.
    println!("Sorting data...");
    let mut sorted_data: Vec<(f64, usize)> = Vec::new();
    for i in 0..n_points {
        // let p: PointData = input.get_point_info(i);
        sorted_data.push((residuals[i], i)); //(p.z, i));
    }
    // The data are actually sorted from highest to lowest so that
    // the lowet point can be popped from the vector.
    sorted_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    let mut is_assigned = vec![false; n_points];
    let mut segment_id = vec![0usize; n_points];
    let mut num_solved_points = 0.0f64;
    let mut seed_id = 0usize;
    let mut current_seg_id: usize = 0;

    let mut segment_histo: Vec<usize> = vec![];
    segment_histo.push(0);

    while num_solved_points < num_points {
        // Pop points from sorted_data until one is found that has not
        // been assigned a segment_id.
        let mut seed_found = false;
        while !seed_found && !sorted_data.is_empty() {
            let val = sorted_data.pop().unwrap().1;
            if !is_assigned[val] {
                seed_id = val;
                seed_found = true;
            }
        }

        if seed_found {
            let mut stack: Vec<usize> = Vec::new();

            // push the seed point
            stack.push(seed_id);
            is_assigned[seed_id] = true;
            current_seg_id += 1;
            segment_id[seed_id] = current_seg_id;
            segment_histo.push(1);
            num_solved_points += 1.0;

            if !variable_dist {
                while !stack.is_empty() {
                    let i = stack.pop().unwrap();
                    let p = input[i];
                    let ret = frs.search(p.x, p.y);
                    let z = residuals[i]; //p.z;
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        if !is_assigned[index_n] && index_n != i {
                            let a = angle_between(normal_vectors[i], normal_vectors[index_n]);
                            let zn = residuals[index_n]; //input.get_point_info(index_n).z;
                            if a < max_angle && ((z - zn)*(z - zn) <= max_z_diff) {
                                stack.push(index_n);
                                is_assigned[index_n] = true;
                                segment_id[index_n] = current_seg_id;
                                num_solved_points += 1.0;
                                segment_histo[current_seg_id] += 1;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * num_solved_points / num_points) as i32;
                        if progress != old_progress {
                            println!("Segmenting point cloud: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            } else {
                while !stack.is_empty() {
                    let i = stack.pop().unwrap();
                    let p = input[i]; //.get_point_info(i);
                    let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbouring_points, &squared_euclidean).unwrap();
                    let z = residuals[i]; //p.z;
                    for j in 0..ret.len() {
                        index_n = *ret[j].1;
                        if !is_assigned[index_n] && index_n != i {
                            let a = angle_between(normal_vectors[i], normal_vectors[index_n]);
                            let zn = residuals[index_n]; //input.get_point_info(index_n).z;
                            if a < max_angle && ((z - zn)*(z - zn) <= max_z_diff) {
                                stack.push(index_n);
                                is_assigned[index_n] = true;
                                segment_id[index_n] = current_seg_id;
                                num_solved_points += 1.0;
                                segment_histo[current_seg_id] += 1;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * num_solved_points / num_points) as i32;
                        if progress != old_progress {
                            println!("Segmenting point cloud: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    let min_segment_size = 10usize;
    for i in 0..n_points {
        let seg_val = segment_id[i];
        if segment_histo[seg_val] < min_segment_size {
            is_assigned[i] = false;
            num_solved_points -= 1.0;
            segment_histo[seg_val] = 0;
        }
    }

    let mut cur_seed_loc: usize = 0;
    while num_solved_points < num_points {
        let mut seed_found = false;
        for i in cur_seed_loc..n_points {
            if !is_assigned[i] {
                seed_id = i;
                cur_seed_loc = i;
                seed_found = true;
                break;
            }
        }

        if seed_found {
            let mut stack: Vec<usize> = Vec::new();

            // push the seed point
            stack.push(seed_id);
            is_assigned[seed_id] = true;
            //current_seg_id += 1;
            current_seg_id = segment_id[seed_id]; // use the seg id of this point
            segment_histo[current_seg_id] = 1;
            num_solved_points += 1.0;

            if !variable_dist {
                while !stack.is_empty() {
                    let i = stack.pop().unwrap();
                    let p = input.get_point_info(i);
                    let ret = frs.search(p.x, p.y);
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        if !is_assigned[index_n] && index_n != i {
                            stack.push(index_n);
                            is_assigned[index_n] = true;
                            segment_id[index_n] = current_seg_id;
                            num_solved_points += 1.0;
                            segment_histo[current_seg_id] += 1;
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * num_solved_points / num_points) as i32;
                        if progress != old_progress {
                            println!("Segmenting point cloud: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            } else {
                while !stack.is_empty() {
                    let i = stack.pop().unwrap();
                    let p = input.get_point_info(i);
                    let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbouring_points, &squared_euclidean).unwrap();
                    for j in 0..ret.len() {
                        index_n = *ret[j].1;
                        if !is_assigned[index_n] && index_n != i {
                            stack.push(index_n);
                            is_assigned[index_n] = true;
                            segment_id[index_n] = current_seg_id;
                            num_solved_points += 1.0;
                            segment_histo[current_seg_id] += 1;
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * num_solved_points / num_points) as i32;
                        if progress != old_progress {
                            println!("Segmenting point cloud: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    // let mut largest_seg: usize = 0;
    // let mut largest_seg_id: usize = 0;
    // for i in 0..segment_histo.len() {
    //     if segment_histo[i] > largest_seg {
    //         largest_seg = segment_histo[i];
    //         largest_seg_id = i;
    //     }
    // }


    // now output the data
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    output.header.point_format = 2;

    let mut clrs: Vec<(u16, u16, u16)> = Vec::new();
    let mut rng = rand::thread_rng();
    let (mut r, mut g, mut b): (u16, u16, u16) = (0u16, 0u16, 0u16);
    current_seg_id = segment_histo.len();
    for _ in 0..current_seg_id+1 as usize {
        let mut flag = false;
        while !flag {
            r = rng.gen::<u8>() as u16 * 256u16;
            g = rng.gen::<u8>() as u16 * 256u16;
            b = rng.gen::<u8>() as u16 * 256u16;
            let max_val = cmp::max(cmp::max(r, g), b);
            //let min_val = cmp::min(cmp::min(r, g), b);
            if max_val >= u16::max_value() / 2 { // && min_val >= u16::max_value() / 4 {
                flag = true;
            }
        }
        // if i != largest_seg_id {
        //     r = rng.gen::<u16>();
        //     g = rng.gen::<u16>();
        //     b = rng.gen::<u16>();
        // } else {
        //     r = 0;
        //     g = u16::max_value();
        //     b = u16::max_value() / 4;
        // }
        clrs.push((r, g, b));
    }

    if classify_ground {
        let mut seg_val: usize;
        let mut class_vals = vec![0u8; n_points];
        let mut segment_min_residual = vec![f64::INFINITY; n_points];
        for i in 0..n_points {
            seg_val = segment_id[i];
            if residuals[i] < segment_min_residual[seg_val] { segment_min_residual[seg_val] = residuals[i]; }
        }
        for i in 0..n_points {
            seg_val = segment_id[i];
            if segment_min_residual[seg_val] <= 0.0 {
                class_vals[i] = 2u8;
            } else {
                class_vals[i] = 1u8;
            }
        }
        for i in 0..n_points {
            let mut p: PointData = input[i];
            p.class_bit_field.set_classification(class_vals[i]);
            let seg_val = segment_id[i];
            let rgb: RgbData = RgbData{ red: clrs[seg_val].0, green: clrs[seg_val].1, blue: clrs[seg_val].2 };
            let lpr: las::LidarPointRecord = las::LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        for i in 0..n_points {
            let p: PointData = input[i];
            let seg_val = segment_id[i];
            let rgb: RgbData = RgbData{ red: clrs[seg_val].0, green: clrs[seg_val].1, blue: clrs[seg_val].2 };
            let lpr: las::LidarPointRecord = las::LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    if verbose { println!("Writing output LAS file..."); }
    let _ = match output.write() {
        Ok(_) => println!("Complete!"),
        Err(e) => println!("error while writing: {:?}", e),
    };
}

#[inline]
fn angle_between(v1: Vector3<f64>, v2: Vector3<f64>) -> f64 {
    let num = v1.dot(&v2);
    let d1 = (v1.x*v1.x + v1.y*v1.y + v1.z*v1.z).sqrt();
    let d2 = (v2.x*v2.x + v2.y*v2.y + v2.z*v2.z).sqrt();
    // let j = v1.dot(&v2) / (Norm::norm(&v1)*Norm::norm(&v2));
    let j = num / (d1 * d2);
    j.acos()
}

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
#[inline]
fn plane_from_points(points: &Vec<Vector3<f64>>) -> Vector3<f64> {
    let n = points.len();
    // assert!(n >= 3, "At least three points required");

    let mut sum = Vector3{ x: 0.0, y: 0.0, z: 0.0 };
    for p in points {
        sum = sum + *p;
    }
    let centroid = sum * (1.0 / (n as f64));

    // Calc full 3x3 covariance matrix, excluding symmetries:
    let mut xx = 0.0; let mut xy = 0.0; let mut xz = 0.0;
    let mut yy = 0.0; let mut yz = 0.0; let mut zz = 0.0;

    for p in points {
        let r = p - &centroid;
        xx += r.x * r.x;
        xy += r.x * r.y;
        xz += r.x * r.z;
        yy += r.y * r.y;
        yz += r.y * r.z;
        zz += r.z * r.z;
    }

    let det_x = yy*zz - yz*yz;
    let det_y = xx*zz - xz*xz;
    let det_z = xx*yy - xy*xy;

    let det_max = det_x.max(det_y).max(det_z);

    // Pick path with best conditioning:
    let dir =
        if det_max == det_x {
            let a = (xz*yz - xy*zz) / det_x;
            let b = (xy*yz - xz*yy) / det_x;
            Vector3{ x: 1.0, y: a, z: b }
        } else if det_max == det_y {
            let a = (yz*xz - xy*zz) / det_y;
            let b = (xy*xz - yz*xx) / det_y;
            Vector3{ x: a, y: 1.0, z: b }
        } else {
            let a = (yz*xy - xz*yy) / det_z;
            let b = (xz*xy - yz*xx) / det_z;
            Vector3{ x: a, y: b, z: 1.0 }
        };

    //plane_from_point_and_normal(centroid, normalize(dir))
    normalize(dir)
}

#[inline]
fn normalize(v: Vector3<f64>) -> Vector3<f64> {
    let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    Vector3 { x: v.x/norm, y: v.y/norm, z: v.z/norm }
}

#[derive(Debug, Clone, Copy)]
pub struct ClusterData {
    mean: Vector3<f64>,
    variance: Vector3<f64>,
    n: f64,
}

impl Default for ClusterData {
    fn default() -> ClusterData {
        ClusterData { mean: Vector3{ x: 0.0, y: 0.0, z: 0.0 }, variance: Vector3{ x: 0.0, y: 0.0, z: 0.0 }, n: 0.0 }
    }
}
