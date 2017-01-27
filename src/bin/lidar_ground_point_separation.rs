//////////////////////////////////////////////////////////////////////////////////////////////
// This tool performs a filter operation on a LAS file. The basis of the filter is
// a mathematical morphology operator known as an 'Opening', which is an erosion (min filter)
// followed by a dialation (max filter).
//////////////////////////////////////////////////////////////////////////////////////////////
extern crate kdtree;
extern crate whitebox_tools;

use std::env;
use std::f64;
use std::io::Error;
use std::io::ErrorKind;
use std::path;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;
use whitebox_tools::structures::fixed_radius_search::FixedRadiusSearch;
use kdtree::KdTree;
use kdtree::distance::squared_euclidean;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut search_dist = 2.0f64;
    let mut max_elev_diff = 1.5;
    let mut min_elev_diff = 0.15;
    let mut slope_threshold = 60f64;
    let mut minz = f64::NEG_INFINITY;
    let mut verbose = false;
    let mut filter = true;
    let mut ground_class_value = 2u8;
    let mut oto_class_value = 1u8;
    let mut variable_dist = true;
    let mut num_neighbouring_points = 25;
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
            variable_dist = true;
            if keyval {
                num_neighbouring_points = vec[1].to_string().parse::<usize>().unwrap();
            } else {
                num_neighbouring_points = args[i+1].to_string().parse::<usize>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
            if keyval {
                slope_threshold = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                slope_threshold = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-minz" || vec[0].to_lowercase() == "--minz" {
            if keyval {
                minz = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                minz = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-minzdiff" || vec[0].to_lowercase() == "--minzdiff" {
            if keyval {
                min_elev_diff = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                min_elev_diff = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-maxzdiff" || vec[0].to_lowercase() == "--maxzdiff" {
            if keyval {
                max_elev_diff = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                max_elev_diff = args[i+1].to_string().parse::<f64>().unwrap();
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
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
            vec[0].to_lowercase() == "--h"{
            let mut s: String = "Help:\n".to_owned();
             s.push_str("-i           Input LAS file.\n");
             s.push_str("-o           Output LAS file.\n");
             s.push_str("-wd          Optional working directory. If specified, filenames parameters need not include a full path.\n");
             s.push_str("-dist        Optional search distance in xy units; default is 2.0.\n");
             s.push_str("-slope       Slope threshold, in degrees; default is 60.0.\n");
             s.push_str("-num_points  Optional number (integer) of nearest-neighbour points in place of constant search distance.\n");
             s.push_str("-minz        Minimum elevation used in the analysis (optional).\n");
             s.push_str("-maxzdiff    Maximum elevaton difference between points and opening surface; default 1.5.\n");
             s.push_str("-minzdiff    Minimum elevaton difference between points to be considered an off-terrain point; default 0.15.\n");
             s.push_str("-class       If this flag is used, the output LAS file will contain all the points of the input, but classified to indicate whether a point belongs to the slice.\n");
             s.push_str("-groundclass Class value (integer between 0-31) to be assigned to ground points; default is 2.\n");
             s.push_str("-otoclass    Class value (integer between 0-31) to be assigned to off-terrain objects (OTOs); default is 1.\n");
             s.push_str("-v           Verbose mode; if this flag is present, the tool will report progress if this flag is provided.\n");
             s.push_str("-version     Prints the tool version number.\n");
             s.push_str("-h           Prints help information.\n\n");
             s.push_str("Example usage:\n\n");
             s.push_str(&">> .*lidar_ground_point_separation -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -v\n".replace("*", &sep));
             s.push_str(&">> .*lidar_ground_point_separation -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -minz 0.0 -class -v\n".replace("*", &sep));
             s.push_str(&">> .*lidar_ground_point_separation -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -class -groundclass 1 -otoclass 0 -v\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_ground_point_separation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    match run(
        input_file, output_file,
        working_directory,
        search_dist,
        min_elev_diff,
        max_elev_diff,
        slope_threshold,
        minz,
        verbose,
        filter,
        ground_class_value,
        oto_class_value,
        variable_dist,
        num_neighbouring_points
    ) {
        Ok(()) => println!("Complete!"),
        Err(err) => panic!("{}", err),
    }
}

fn run(mut input_file: String, mut output_file: String, mut working_directory: String,
    search_dist: f64, min_elev_diff: f64, max_elev_diff: f64, mut slope_threshold: f64, minz: f64,
    verbose: bool, filter: bool, ground_class_value: u8, oto_class_value: u8,
    variable_dist: bool, num_neighbouring_points: usize)
    -> Result<(), Error> {

    println!("**************************************");
    println!("* Welcome to lidar_ground_separation *");
    println!("**************************************");

    if ground_class_value > 31 || oto_class_value > 31 {
        return Err(Error::new(ErrorKind::InvalidInput, "Error: Either the ground or OTO class values are larger than 31."));
    }

    slope_threshold = slope_threshold.to_radians().tan();

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

    // let input: las::LasFile = las::LasFile::new(&input_file, "r")?;
    let input = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error reading file {}: {}", input_file, err),
    };

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only
    let mut is_off_terrain = vec![false; n_points];

    let mut progress: i32;
    let mut old_progress: i32 = -1;
    let mut zn: f64;
    let mut index_n: usize;
    let mut neighbourhood_min = vec![f64::INFINITY; n_points];
    let mut neighbourhood_max_min = vec![f64::NEG_INFINITY; n_points];
    let mut residuals = vec![0.0f64; n_points];
    // let mut elevations = vec![0.0f64; n_points];

    if !variable_dist {
        // use a fixed radius search
        let mut frs: FixedRadiusSearch<usize> = FixedRadiusSearch::new(search_dist);
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            if p.z > minz && p.classification() != 7u8 && p.classification() != 18u8 {
                frs.insert(p.x, p.y, i);
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Binning points: {}%", progress);
                        old_progress = progress;
                    }
                }
            } else {
                is_off_terrain[i] = true;
            }
        }

        for i in 0..n_points {
            // if !is_off_terrain[i] {
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
            // }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Performing erosion: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for i in 0..n_points {
            // if !is_off_terrain[i] {
                let p: PointData = input.get_point_info(i);
                let ret = frs.search(p.x, p.y);
                for j in 0..ret.len() {
                    index_n = ret[j].0;
                    if neighbourhood_min[index_n] > neighbourhood_max_min[i] {
                        neighbourhood_max_min[i] = neighbourhood_min[index_n];
                    }
                }
                // if (p.z - neighbourhood_max_min[i]) > min_elev_diff {
                //     is_off_terrain[i] = true;
                // }
                residuals[i] = p.z - neighbourhood_max_min[i];
            // }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Performing dilation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut elev_diff: f64;
        let mut z: f64;
        let mut zn: f64;
        let (mut higher_z, mut lower_z, mut higher_index): (f64, f64, usize);
        let mut index_n: usize;
        let mut dist: f64;
        let mut slope: f64;

        for i in 0..n_points {
            if !is_off_terrain[i] {
                let p: PointData = input.get_point_info(i);
                if p.is_late_return() && p.classification() != 7u8 && p.classification() != 18u8 { // not a first or intermediate return, not a low or high noise point
                    if residuals[i] > max_elev_diff {
                        is_off_terrain[i] = true;
                    } else {
                        let ret = frs.search(p.x, p.y);
                        z = residuals[i];
                        for j in 0..ret.len() {
                            dist = ret[j].1;
                            if dist > 0.0 {
                                index_n = ret[j].0;
                                zn = residuals[index_n];
                                if zn < z {
                                    higher_z = z;
                                    lower_z = zn;
                                    higher_index = i;
                                } else {
                                    higher_z = zn;
                                    lower_z = z;
                                    higher_index = index_n;
                                }
                                elev_diff = higher_z - lower_z;
                                slope = elev_diff / dist;
                                if slope > slope_threshold && elev_diff > min_elev_diff {
                                    is_off_terrain[higher_index] = true;
                                }
                            }
                        }
                    }
                } else {
                    is_off_terrain[i] = true;
                }
            } else {
                is_off_terrain[i] = true;
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let dist_decay = 1f64;
        // let height_decay = 2f64;
        // let mut z: f64;
        // let mut weight: f64;
        // let max_usable_residual = min_elev_diff * 2.0;
        // for i in 0..n_points {
        //     let p: PointData = input.get_point_info(i);
        //     let ret = frs.search(p.x, p.y);
        //     let mut sum_weights = 0f64;
        //     z = 0f64;
        //     for j in 0..ret.len() {
        //         index_n = ret[j].0;
        //         if residuals[index_n] < max_usable_residual {
        //             weight = (ret[j].1).powf(dist_decay) + (residuals[index_n]).powf(height_decay);
        //             if weight > 0.0 {
        //                 sum_weights += 1.0 / weight;
        //             } else {
        //                 let p_n: PointData = input.get_point_info(index_n);
        //                 z = p_n.z;
        //                 sum_weights = 0f64;
        //                 break
        //             }
        //         }
        //     }
        //     if sum_weights > 0f64 {
        //         for j in 0..ret.len() {
        //             index_n = ret[j].0;
        //             if residuals[index_n] < max_usable_residual {
        //                 let p_n: PointData = input.get_point_info(index_n);
        //                 weight = (ret[j]).1.powf(dist_decay) + (residuals[index_n]).powf(height_decay);
        //                 z += p_n.z * (1.0 / weight) / sum_weights;
        //             }
        //         }
        //     }
        //     let p: PointData = input.get_point_info(i);
        //     elevations[i] = p.z - z;
        //     if verbose {
        //         progress = (100.0_f64 * i as f64 / num_points) as i32;
        //         if progress != old_progress {
        //             println!("Performing Interpolation: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

    } else {
        // use a K-D tree
        let dimensions = 2;
        let capacity_per_node = 128;
        let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);

        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            if p.z > minz {
                let coords: [f64; 2] = [ p.x, p.y ];
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

        for i in 0..n_points {
            if !is_off_terrain[i] {
                let p: PointData = input.get_point_info(i);
                let ret = kdtree.nearest(&[ p.x, p.y ], num_neighbouring_points, &squared_euclidean).unwrap();
                for j in 0..ret.len() {
                    index_n = *ret[j].1;
                    let pn: PointData = input.get_point_info(index_n);
                    zn = pn.z;
                    if zn < neighbourhood_min[i] {
                        neighbourhood_min[i] = zn;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Performing Analysis (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for i in 0..n_points {
            if !is_off_terrain[i] {
                let p: PointData = input.get_point_info(i);
                let ret = kdtree.nearest(&[ p.x, p.y ], num_neighbouring_points, &squared_euclidean).unwrap();
                for j in 0..ret.len() {
                    index_n = *ret[j].1;
                    if neighbourhood_min[index_n] > neighbourhood_max_min[i] {
                        neighbourhood_max_min[i] = neighbourhood_min[index_n];
                    }
                }
                if (p.z - neighbourhood_max_min[i]) > min_elev_diff {
                    is_off_terrain[i] = true;
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Performing Analysis (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    // now output the data
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    output.header.system_id = "EXTRACTION".to_string();

    let mut num_points_filtered: i64 = 0;
    if filter {
        for i in 0..input.header.number_of_points as usize {
            if !is_off_terrain[i] {
                output.add_point_record(input.get_record(i));
                // let pr = input.get_record(i);
                // let pr2: las::LidarPointRecord;
                // match pr {
                //     las::LidarPointRecord::PointRecord0 { mut point_data }  => {
                //         point_data.z = residuals[i];
                //         pr2 = las::LidarPointRecord::PointRecord0 { point_data: point_data };
                //
                //     },
                //     las::LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                //         point_data.z = residuals[i];
                //         pr2 = las::LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data };
                //     },
                //     las::LidarPointRecord::PointRecord2 { mut point_data, rgb_data } => {
                //         point_data.z = residuals[i];
                //         pr2 = las::LidarPointRecord::PointRecord2 { point_data: point_data, rgb_data: rgb_data };
                //     },
                //     las::LidarPointRecord::PointRecord3 { mut point_data, gps_data, rgb_data } => {
                //         point_data.z = residuals[i];
                //         pr2 = las::LidarPointRecord::PointRecord3 { point_data: point_data,
                //             gps_data: gps_data, rgb_data: rgb_data};
                //     },
                // }
                // output.add_point_record(pr2);
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
            if is_off_terrain[i] { class_val = oto_class_value; } // off terrain point
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
        output.write()?;
    } else {
        return Err(Error::new(ErrorKind::InvalidData, "No points were contained in the output file."));
    }

    Ok(())
}
