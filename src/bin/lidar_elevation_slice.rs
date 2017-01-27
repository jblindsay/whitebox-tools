#![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;

use std::env;
use std::path;
use std::f64;
use whitebox_tools::lidar::las;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut minz = -f64::INFINITY;
    let mut maxz = f64::INFINITY;
    let mut verbose: bool = false;
    let mut filter = true;
    let mut in_class_value = 2u8;
    let mut out_class_value = 1u8;

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
        } else if vec[0].to_lowercase() == "-maxz" || vec[0].to_lowercase() == "--maxz" {
            if keyval {
                maxz = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                maxz = args[i+1].to_string().parse::<f64>().unwrap();
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
        } else if vec[0].to_lowercase() == "-inclassval" || vec[0].to_lowercase() == "--inclassval" {
            filter = false;
            if keyval {
                in_class_value = vec[1].to_string().parse::<u8>().unwrap();
            } else {
                in_class_value = args[i+1].to_string().parse::<u8>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-outclassval" || vec[0].to_lowercase() == "--outclassval" {
            filter = false;
            if keyval {
                out_class_value = vec[1].to_string().parse::<u8>().unwrap();
            } else {
                out_class_value = args[i+1].to_string().parse::<u8>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
                     s.push_str("-i           Input LAS file.\n");
                     s.push_str("-o           Output LAS file.\n");
                     s.push_str("-wd          Optional working directory. If specified, input and output filenames need not include a full path.\n");
                     s.push_str("-minz        Optional minimum elevation (inclusive) for the slice (default is -Inf).\n");
                     s.push_str("-maxz        Optional maximum elevation (inclusive) for the slice (default is Inf).\n");
                     s.push_str("-class       If this flag is used, the output LAS file will contain all the points of the input, but classified to indicate whether a point belongs to the slice.\n");
                     s.push_str("-inclassval  Class value (integer between 0-31) to be assigned to points within the slice elevation range. Default is 2.\n");
                     s.push_str("-outclassval Class value (integer between 0-31) to be assigned to points outside of the slice elevation range. Default is 1.\n");
                     s.push_str("-v           Verbose mode; if this flag is present, the tool will report progress if this flag is provided.\n");
                     s.push_str("-version     Prints the tool version number.\n");
                     s.push_str("-h           Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*lidar_elevation_slice -v -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -minz 100.0 -maxz 250.0\n".replace("*", &sep));
                     s.push_str(&">> .*lidar_elevation_slice -v -i \"*path*to*data*input.las\" -o \"*path*to*data*output.las\" -minz 100.0 -maxz 250.0 -class\n".replace("*", &sep));
                     s.push_str(&">> .*lidar_elevation_slice -v -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -minz 100.0 -maxz 250.0 -class -inclassval 1 -outclassval 0\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    if verbose {
        println!("************************************");
        println!("* Welcome to lidar_elevation_slice *");
        println!("************************************");
    }

    if in_class_value > 31 || out_class_value > 31 {
        panic!("Error: Either the in-slice or out-of-slice class values are larger than 31.");
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

    if verbose { println!("Reading input LAS file..."); }
    //let input = las::LasFile::new(&input_file, "r");
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    output.header.system_id = "EXTRACTION".to_string();

    // let mut output = las::LasFile::new(&output_file, "w");
    //
    // output.add_header(input.header.clone());
    //
    // for i in 0..(input.header.number_of_vlrs as usize) {
    //     output.add_vlr(input.vlr_data[i].clone());
    // }

    if verbose { println!("Performing analysis..."); }
    let mut z: f64;
    let mut progress: i32;
    let mut old_progress: i32 = -1;
    let mut num_points_filtered: i64 = 0;
    let num_points: f64 = (input.header.number_of_points - 1) as f64;

    if filter {
        for i in 0..input.header.number_of_points as usize {
            z = input.get_point_info(i).z;
            if z >= minz && z <= maxz {
                output.add_point_record(input.get_record(i));
                num_points_filtered += 1;
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    } else {
        for i in 0..input.header.number_of_points as usize {
            let mut class_val = out_class_value; // outside elevation slice
            z = input.get_point_info(i).z;
            if z >= minz && z <= maxz {
                class_val = in_class_value; // inside elevation slice
            }
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
        num_points_filtered = 1;
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
