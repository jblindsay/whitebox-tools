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

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut search_dist = 2.0f64;
    let mut minz = f64::NEG_INFINITY;
    let mut verbose = false;
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
            if keyval {
                search_dist = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                search_dist = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-minz" || vec[0].to_lowercase() == "--minz" {
            if keyval {
                minz = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                minz = args[i+1].to_string().parse::<f64>().unwrap();
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
             s.push_str("-minz        Minimum elevation used in the analysis (optional).\n");
             s.push_str("-v           Verbose mode; if this flag is present, the tool will report progress if this flag is provided.\n");
             s.push_str("-version     Prints the tool version number.\n");
             s.push_str("-h           Prints help information.\n\n");
             s.push_str("Example usage:\n\n");
             s.push_str(&">> .*lidar_slope_based_filter -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -v\n".replace("*", &sep));
             s.push_str(&">> .*lidar_slope_based_filter -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -minz 0.0 -class -v\n".replace("*", &sep));
             s.push_str(&">> .*lidar_slope_based_filter -wd \"*path*to*data*\" -i \"input.las\" -o \"output.las\" -dist 5.0 -slope 45.0 -class -groundclass 1 -otoclass 0 -v\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_elev_above_ground v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    match run(
        input_file,
        output_file,
        working_directory,
        search_dist,
        minz,
        verbose,
    ) {
        Ok(()) => println!("Complete!"),
        Err(err) => panic!("{}", err),
    }
}

fn run(mut input_file: String, mut output_file: String, mut working_directory: String,
    search_dist: f64, minz: f64, verbose: bool)
    -> Result<(), Error> {

    println!("**************************************");
    println!("* Welcome to lidar_elev_above_ground *");
    println!("**************************************");

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

    let input: las::LasFile = las::LasFile::new(&input_file, "r")?;

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only
    let mut progress: i32;
    let mut old_progress: i32 = -1;
    let mut zn: f64;
    let mut index_n: usize;
    let mut neighbourhood_min = vec![f64::INFINITY; n_points];
    let mut neighbourhood_max_min = vec![f64::NEG_INFINITY; n_points];
    let mut residuals = vec![0.0f64; n_points];

    let mut frs: FixedRadiusSearch<usize> = FixedRadiusSearch::new(search_dist);
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        if p.z > minz && p.classification() != 7u8 {
            frs.insert(p.x, p.y, i);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
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

    // now output the data
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    output.header.system_id = "EXTRACTION".to_string();

    let mut num_points_filtered: i64 = 0;
    for i in 0..input.header.number_of_points as usize {
        let pr = input.get_record(i);
        let pr2: las::LidarPointRecord;
        match pr {
            las::LidarPointRecord::PointRecord0 { mut point_data }  => {
                point_data.z = residuals[i];
                pr2 = las::LidarPointRecord::PointRecord0 { point_data: point_data };

            },
            las::LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                point_data.z = residuals[i];
                pr2 = las::LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data };
            },
            las::LidarPointRecord::PointRecord2 { mut point_data, rgb_data } => {
                point_data.z = residuals[i];
                pr2 = las::LidarPointRecord::PointRecord2 { point_data: point_data, rgb_data: rgb_data };
            },
            las::LidarPointRecord::PointRecord3 { mut point_data, gps_data, rgb_data } => {
                point_data.z = residuals[i];
                pr2 = las::LidarPointRecord::PointRecord3 { point_data: point_data,
                    gps_data: gps_data, rgb_data: rgb_data};
            },
        }
        output.add_point_record(pr2);
        num_points_filtered += 1;
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Saving data: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if num_points_filtered > 0 {
        if verbose { println!("Writing output LAS file..."); }
        output.write()?;
    } else {
        return Err(Error::new(ErrorKind::InvalidData, "No points were contained in the output file."));
    }

    Ok(())
}
