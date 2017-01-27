#![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;
extern crate nalgebra as na;
extern crate kdtree;

use std::env;
use std::f64;
use std::path;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;
use kdtree::KdTree;
use kdtree::distance::squared_euclidean;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut threshold_density = 1.0;
    let mut num_neighbours = 10;
    let mut verbose: bool = false;

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
        } else if vec[0].to_lowercase() == "-threshold_density" || vec[0].to_lowercase() == "--threshold_density" {
            if keyval {
                threshold_density = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                threshold_density = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-num_neighbours" || vec[0].to_lowercase() == "--num_neighbours" {
            if keyval {
                num_neighbours = vec[1].to_string().parse::<usize>().unwrap();
            } else {
                num_neighbours = args[i+1].to_string().parse::<usize>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
             s.push_str("-i                 Input LAS file.\n");
             s.push_str("-o                 Output LAS file.\n");
             s.push_str("-wd                Optional working directory. If specified, input and output filenames need not include a full path.\n");
             s.push_str("-threshold_density Threshold in point density (pts / m^3) below which points are filtered from the cloud.\n");
             s.push_str("-num_neighbours    Number of neighbouring points used to determine point density in the region surrounding each point.\n");
             s.push_str("-v                 Optional verbose mode. Tool will report progress if this flag is provided.\n");
             s.push_str("-version           Prints the tool version number.\n");
             s.push_str("-h                 Prints help information.\n\n");
             s.push_str("Example usage:\n\n");
             s.push_str(&">> .*lidar_normal_vec -v -wd *path*to*data* -i input.las -o output.las -num_points 15\n".replace("*", &sep));
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

    lidar_remove_outliers(input_file, output_file, threshold_density, num_neighbours, verbose);
}

fn lidar_remove_outliers(input_file: String, output_file: String, threshold_density: f64, num_neighbours: usize, verbose: bool) {
    if verbose {
        println!("************************************");
        println!("* Welcome to lidar_remove_outliers *");
        println!("************************************");
    }

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

    let mut progress: i32;
    let mut old_progress: i32 = -1;
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

    let num_neighbours_less_one = num_neighbours - 1;
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    let mut p: PointData;
    let mut area: f64;
    let mut r: f64;
    let four_thirds_pi = 4.0 / 3.0 * f64::consts::PI;
    let mut density: f64;
    let mut num_points_in_filtered: i64 = 0;
    for i in 0..n_points {
        p = input[i];
        let ret = kdtree.nearest(&[ p.x, p.y, p.z ], num_neighbours, &squared_euclidean).unwrap();
        r = ret[num_neighbours_less_one].0.sqrt();
        area = four_thirds_pi * r * r * r;
        density = num_neighbours as f64 / area;
        if density >= threshold_density {
            // output the point
            output.add_point_record(input.get_record(i));
            num_points_in_filtered += 1;
        }
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if num_points_in_filtered > 0 {
        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };
    } else {
        println!("No points were contained in the elevation slice.");
    }
}
