extern crate whitebox_tools;
extern crate nalgebra as na;
extern crate kdtree;
extern crate rand;

use std::env;
use std::f64;
use std::path;
use std::default::Default;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;
use na::{ Dot, Vector3 };
use kdtree::KdTree;
use kdtree::distance::squared_euclidean;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut search_dist: f64 = 5.0;
    let mut num_neighbouring_points: usize = 10;
    let mut max_normal_angle = 2.0f64;
    let mut maxzdiff = 1.0;
    let mut minz = f64::NEG_INFINITY;
    let mut filter = true;
    let mut ground_class_value = 2u8;
    let mut oto_class_value = 1u8;
    let mut last_only = false;
    let mut verbose: bool = false;
    let mut variable_dist = true;

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
        } else if vec[0].to_lowercase() == "-max_z_diff" || vec[0].to_lowercase() == "--max_z_diff" {
            if keyval {
                maxzdiff = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                maxzdiff = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-minz" || vec[0].to_lowercase() == "--minz" {
            if keyval {
                minz = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                minz = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
            filter = true;
        } else if vec[0].to_lowercase() == "-class" || vec[0].to_lowercase() == "--class" {
            filter = false;
        } else if vec[0].to_lowercase() == "-groundclass" || vec[0].to_lowercase() == "--groundclass" {
            filter = false;
            if keyval {
                ground_class_value = vec[1].to_string().parse::<u8>().unwrap();
            } else {
                ground_class_value = args[i+1].to_string().parse::<u8>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-otoclass" || vec[0].to_lowercase() == "--otoclass" {
            filter = false;
            if keyval {
                oto_class_value = vec[1].to_string().parse::<u8>().unwrap();
            } else {
                oto_class_value = args[i+1].to_string().parse::<u8>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-last_only" || vec[0].to_lowercase() == "--last_only" {
            last_only = true;
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
            s.push_str("This tool can be used to filter a LiDAR point cloud for ground points. This filtering
is based on a segmentation procedure and entire segments are classed as either
'ground' or 'off-terrain' segments. Off-terrain segments are considered to be those
with mean normal vectors that are not upwards-facing and that are elevated above
neighbouring segments. The tool can either remove off-terrain points within the
output file, or if the optional -class flag is provided, it can simply classify
points in the output file as 'ground' and 'unclassified'.\n");
            s.push_str("\nTool flags:\n");
            s.push_str("-i               Input LAS file.\n");
            s.push_str("-o               Output LAS file.\n");
            s.push_str("-wd              Optional working directory. If specified, input and output filenames need not include a full path.\n");
            s.push_str("-dist            Optional search distance in xy units; default is variable, determined by num_points.\n");
            s.push_str("-num_points      Number (integer) of nearest-neighbour points used for plane fitting; default is 10.\n");
            s.push_str("-max_norm_angle  Maximum deviation (degrees) in normal vectors between neighbouring points of the same segment; default is 2.0.\n");
            s.push_str("-max_z_diff      Maximum difference in elevation (z units) between neighbouring points of the same segment; defuault is 1.0.\n");
            s.push_str("-last_only       Optional boolean indicating whether only last-return points should be considered.\n");
            s.push_str("-class           If this flag is used, the output LAS file will contain all the points of the input, but classified to indicate whether a point belongs to the slice.\n");
            s.push_str("-groundclass     Class value (integer between 0-31) to be assigned to ground points; default is 2.\n");
            s.push_str("-otoclass        Class value (integer between 0-31) to be assigned to off-terrain objects (OTOs); default is 1.\n");
            s.push_str("-v               Optional verbose mode. Tool will report progress if this flag is provided.\n");
            s.push_str("-version         Prints the tool version number.\n");
            s.push_str("-h               Prints help information.\n\n");
            s.push_str("Example usage:\n\n");
            s.push_str(&">> .*lidar_segmentation_based_filter -wd *path*to*data* -i input.las -o output.las -num_points 50 -max_norm_angle 3.5 -max_z_diff 0.5 -v\n".replace("*", &sep));
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
        max_normal_angle, maxzdiff, verbose, variable_dist, minz, filter, ground_class_value,
        oto_class_value, last_only);
}

fn lidar_segmentation(input_file: String, output_file: String, search_dist: f64, num_neighbouring_points: usize,
                      mut max_angle: f64, mut max_z_diff: f64, verbose: bool, variable_dist: bool, minz: f64,
                      filter: bool, ground_class_value: u8, oto_class_value: u8, last_only: bool) {
    if verbose {
        println!("*********************************");
        println!("* Welcome to lidar_segmentation *");
        println!("*********************************");
    }

    max_angle = max_angle.to_radians();
    max_z_diff = max_z_diff * max_z_diff;

    if verbose { println!("Reading input LAS file..."); }
    //let input = las::LasFile::new(&input_file, "r");
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };

    if verbose { println!("Performing analysis..."); }

    let dimensions = 3;
    let capacity_per_node = 128;
    let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let mut is_off_terrain = vec![false; n_points];

    let mut progress: i32;
    let mut old_progress: i32 = -1;
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        if last_only {
            is_off_terrain[i] = !p.is_late_return();
        }
        if p.z > minz {
            let coords: [f64; 3] = [ p.x, p.y, p.z ];
            kdtree.add(coords.clone(), i).unwrap();
        }
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Creating 3D-tree: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let search_dist_sqrd = search_dist * search_dist;

    let mut index_n: usize;
    let mut normal_vectors: Vec<Vector3<f64>> = vec![];
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbouring_points, &squared_euclidean).unwrap();
        let mut data: Vec<Vector3<f64>> = vec![];;
        for j in 0..ret.len() {
            index_n = *ret[j].1;
            let p2: PointData = input.get_point_info(index_n);
            data.push(Vector3 { x: p2.x, y: p2.y, z: p2.z });
        }
        normal_vectors.push(plane_from_points(&data));
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Calculating point normal vectors: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // Sort the data based on elevation.
    println!("Sorting data...");
    let mut sorted_data: Vec<(f64, usize)> = Vec::new();
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        sorted_data.push((p.z, i));
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

    // let mut is_off_terrain_segment: Vec<bool> = vec![];
    // is_off_terrain_segment.push(false);

    let mut segment_mean_norm: Vec<ClusterData> = vec![];
    segment_mean_norm.push(ClusterData{ ..Default::default() });

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

            let mut clust_data = ClusterData{mean: normal_vectors[seed_id].clone(), variance: Vector3{x: 0.0, y: 0.0, z: 0.0}, n: 1.0};

            if !variable_dist {
                while !stack.is_empty() {
                    let i = stack.pop().unwrap();
                    let p = input.get_point_info(i);
                    let ret = kdtree.within(&[ p.x, p.y, p.z ], search_dist_sqrd, &squared_euclidean).unwrap();
                    let z = p.z;
                    for j in 0..ret.len() {
                        index_n = *ret[j].1;
                        if !is_assigned[index_n] && index_n != i {
                            let a = angle_between(normal_vectors[i], normal_vectors[index_n]);
                            let zn = input.get_point_info(index_n).z;
                            if a < max_angle && ((z - zn)*(z - zn) <= max_z_diff) {
                                stack.push(index_n);
                                is_assigned[index_n] = true;
                                segment_id[index_n] = current_seg_id;
                                num_solved_points += 1.0;
                                segment_histo[current_seg_id] += 1;

                                clust_data.n += 1.0;
                                let delta = normal_vectors[index_n] - clust_data.mean;
                                clust_data.mean += delta / clust_data.n;
                                clust_data.variance += delta*(normal_vectors[index_n] - clust_data.mean);
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
                    let p = input.get_point_info(i);
                    let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbouring_points, &squared_euclidean).unwrap();
                    let z = p.z;
                    for j in 0..ret.len() {
                        index_n = *ret[j].1;
                        if !is_assigned[index_n] && index_n != i {
                            let a = angle_between(normal_vectors[i], normal_vectors[index_n]);
                            let zn = input.get_point_info(index_n).z;
                            if a < max_angle && ((z - zn)*(z - zn) <= max_z_diff) {
                                stack.push(index_n);
                                is_assigned[index_n] = true;
                                segment_id[index_n] = current_seg_id;
                                num_solved_points += 1.0;
                                segment_histo[current_seg_id] += 1;

                                clust_data.n += 1.0;
                                let delta = normal_vectors[index_n] - clust_data.mean;
                                clust_data.mean += delta / clust_data.n;
                                clust_data.variance += delta*(normal_vectors[index_n] - clust_data.mean);
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

            if clust_data.n < 2.0 {
                clust_data.variance / (clust_data.n - 1.0);
            }

            segment_mean_norm.push(clust_data);

            // if clust_data.mean.z == get_max_component(clust_data.mean) {
            //     is_off_terrain_segment.push(false);
            // } else {
            //     is_off_terrain_segment.push(true);
            // }

        } else {
            break;
        }
    }

    let num_segments = current_seg_id + 1; //is_off_terrain_segment.len();
    let mut is_off_terrain_segment: Vec<bool> = vec![false; num_segments];

    // Use a simple proximity based region growing clustering method for smaller segments
    let min_segment_size = 10usize;
    for i in 0..n_points {
        let seg_val = segment_id[i];
        if segment_histo[seg_val] < min_segment_size {
            is_assigned[i] = false;
            num_solved_points -= 1.0;
            segment_histo[seg_val] = 0;
            //is_off_terrain[i] = true;
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
                    let ret = kdtree.within(&[ p.x, p.y, p.z ], search_dist_sqrd, &squared_euclidean).unwrap();
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

    //let mut num_actual_segments: usize = 0;
    // for i in 0..num_segments {
    //     if segment_histo[i] > 0 {
    //         num_actual_segments += 1;
    //     }
    // }

    // Create a 2D KD-tree
    let dimensions = 2;
    let capacity_per_node = 128;
    let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);

    for i in 0..n_points {
        let p: PointData = input[i]; //.get_point_info(i);
        if p.z > minz {
            let coords: [f64; 2] = [ p.x, p.y ];
            kdtree.add(coords.clone(), i).unwrap();
        }
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Creating 2D-tree: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // find the maximum elevation difference to a neighbouring or co-located segment for each segment.
    //let mut max_seg_elev_diff = vec![f64::NEG_INFINITY; num_segments];
    let mut num_lower = vec![0.0; num_segments];
    let mut num_higher = vec![0.0; num_segments];
    //let mut k = 0;
    if !variable_dist {
        for i in 0..n_points {
            let seg_val = segment_id[i];
            //if !is_off_terrain[i] {
                let p: PointData = input[i];
                let ret = kdtree.within(&[ p.x, p.y ], search_dist_sqrd, &squared_euclidean).unwrap();
                let mut lowest_neighbour_z = f64::INFINITY;
                let mut lowest_neighbour_id = 0;
                for j in 0..ret.len() {
                    index_n = *ret[j].1;
                    if segment_id[index_n] != seg_val { // && !is_off_terrain[index_n] {
                        let p2: PointData = input[index_n];
                        if p2.z < lowest_neighbour_z {
                            lowest_neighbour_z = p2.z;
                            lowest_neighbour_id = segment_id[index_n];
                        }
                    }
                }
                if lowest_neighbour_z < p.z {
                    num_lower[lowest_neighbour_id] += p.z - lowest_neighbour_z;
                    num_higher[seg_val] += p.z - lowest_neighbour_z;
                }
            //}
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Estimating segment prominance: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        for i in 0..n_points {
            let seg_val = segment_id[i];
            //if !is_off_terrain[i] {
                let p: PointData = input[i];
                let ret = kdtree.nearest(&[ p.x, p.y ], num_neighbouring_points, &squared_euclidean).unwrap();
                let mut lowest_neighbour_z = f64::INFINITY;
                let mut lowest_neighbour_id = 0;
                for j in 0..ret.len() {
                    index_n = *ret[j].1;
                    if segment_id[index_n] != seg_val { // && !is_off_terrain[index_n] {
                        let p2: PointData = input[index_n];
                        if p2.z < lowest_neighbour_z {
                            lowest_neighbour_z = p2.z;
                            lowest_neighbour_id = segment_id[index_n];
                        }
                    }
                }
                if lowest_neighbour_z < p.z {
                    num_lower[lowest_neighbour_id] += p.z - lowest_neighbour_z;
                    num_higher[seg_val] += p.z - lowest_neighbour_z;
                }
            //}
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Estimating segment prominance: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    // for i in 0..num_segments {
    //     println!("{}", max_seg_elev_diff[i]);
    // }

    // any segment that has a maximum elevation difference of greater than the threshold is an off-terrain object
    // let mut m = 0;
    for i in 0..num_segments {
        //let seg_val = segment_id[i];
        if num_lower[i] > num_higher[i] {
            if segment_mean_norm[i].mean.z == get_max_component(segment_mean_norm[i].mean) { // it's low and flat
                is_off_terrain_segment[i] = false;
            } else { // it's relatively low but more vertical
                is_off_terrain_segment[i] = true;
            }
        } else {
            is_off_terrain_segment[i] = true;
        }

        // if max_seg_elev_diff[i] > oto_threshold {
        //     is_off_terrain_segment[i] = true;
        //     m += 1;
        // }
    }

    println!("Num lower: {}; Num higher: {}", num_lower[4], num_higher[4]);

    //let mut num_oto_points = 0;
    for i in 0..n_points {
        if is_off_terrain_segment[segment_id[i]] {
            is_off_terrain[i] = true;
            //num_oto_points += 1;
        }
    }

    //println!("Num OTO segments: {}", k);
    //println!("Num OTO segments: {}; Num segments: {}; k: {}; Num actual segments: {}; Num OTO points: {}", m, num_segments, k, num_actual_segments, num_oto_points);

    // now output the data
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);

    let mut num_points_filtered: i64 = 0;
    if filter {
        for i in 0..input.header.number_of_points as usize {
            if !is_off_terrain[i] {
                output.add_point_record(input.get_record(i));
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
    } else { // classify
        for i in 0..input.header.number_of_points as usize {
            let mut class_val = ground_class_value; // ground point
            let seg_val = segment_id[i];
            if is_off_terrain_segment[seg_val] {
                class_val = oto_class_value;
            }
            //if is_off_terrain[i] { class_val = oto_class_value; } // off terrain point
            let pr = input.get_record(i);
            let pr2: las::LidarPointRecord;
            match pr {
                las::LidarPointRecord::PointRecord0 { mut point_data }  => {
                    point_data.set_classification(class_val);
                    pr2 = las::LidarPointRecord::PointRecord0 { point_data: point_data };

                },
                las::LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                    point_data.set_classification(class_val);
                    pr2 = las::LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data };
                },
                las::LidarPointRecord::PointRecord2 { mut point_data, rgb_data } => {
                    point_data.set_classification(class_val);
                    pr2 = las::LidarPointRecord::PointRecord2 { point_data: point_data, rgb_data: rgb_data };
                },
                las::LidarPointRecord::PointRecord3 { mut point_data, gps_data, rgb_data } => {
                    point_data.set_classification(class_val);
                    pr2 = las::LidarPointRecord::PointRecord3 { point_data: point_data,
                        gps_data: gps_data, rgb_data: rgb_data};
                },
            }
            output.add_point_record(pr2);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        num_points_filtered = 1; // so it passes the saving
    }

    if num_points_filtered > 0 {
        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };
    } else {
        println!("No points were contained in the elevation slice.");
    }
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

#[inline]
fn get_max_component(v: Vector3<f64>) -> f64 {
    if v.x*v.x > v.y*v.y {
        if v.x*v.x > v.z*v.z {
            return v.x;
        }
    } else {
        if v.y*v.y > v.z*v.z {
            return v.x;
        }
    }
    v.z
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
