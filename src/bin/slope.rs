// #![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;

use std::env;
use std::path;
// use std::f64;
use whitebox_tools::raster::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut working_directory: String = "".to_string();
    let mut verbose: bool = false;
    let mut keyval: bool;
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        keyval = false;
        if vec.len() > 1 { keyval = true; }
        if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
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
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
            vec[0].to_lowercase() == "--h"{
            let mut s: String = "Help:\n".to_owned();
                     s.push_str("-i      Input LAS file (classification).\n");
                     s.push_str("-o       Output HTML file.\n");
                     s.push_str("-wd      Optional working directory. If specified, filenames parameters need not include a full path.\n");
                     s.push_str("-version Prints the tool version number.\n");
                     s.push_str("-h       Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*lidar_kappa -wd *path*to*data* -i1 class.las -i2 ref.las -o kappa.html\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("slope v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    if !working_directory.ends_with(&sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(&sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }

    println!("{}", output_file);
    println!("{:?}", verbose);

    // let mut z_conv_factor = 1.0;

    // let file_name = "/Users/johnlindsay/Documents/Data/AlbertaLidar/DEM_final_no_gaps.dep";
    //let file_name = "/Users/johnlindsay/Documents/Data/AlbertaLidar/DEM_10m_no_gaps.dep";
    //let file_name = "/Users/johnlindsay/Documents/Data/AlbertaLidar/DEM denoised5.dep";
    // let file_name = "/Users/johnlindsay/Documents/Data/AlbertaLidar/Alberta/OldLidar.dep";
    //let file_name = "/Users/johnlindsay/Documents/Data/AlbertaLidar/tmp2.dep";
    //let file_name = "/Users/johnlindsay/Documents/Data/Indiana LiDAR/DEM breached.rst";
    //let file_name = "/Users/johnlindsay/Documents/Data/Indiana LiDAR/DEM breached.dep";
    //let file_name = "/Users/johnlindsay/Documents/Data/Indiana LiDAR/out2.grd";

    println!("Reading data...");
    let input = match Raster::new(&input_file, "r") {
        Ok(f) => f,
        Err(err) => panic!("Error: {}", err),
    };
    // let nodata = input.configs.nodata;
    // let eight_res_x = input.configs.resolution_x * 8.0;
    // let eight_res_y = input.configs.resolution_y * 8.0;

    if input.is_in_geographic_coordinates() {
        // calculate a new z-conversion factor
        // let mid_lat = (input.configs.north - input.configs.south) / 2.0;
        // if mid_lat <= 90.0 && mid_lat >= -90.0 {
        //     z_conv_factor = 1.0 / (113200.0 * (f64::consts::PI/180.0*mid_lat).cos());
		// }
    }

    println!("Input file format: {:?}", input.raster_type);
    let r = 50;
    let c = 45;
    println!("Value ({}, {}): {}", r, c, input.get_value(r, c));

    println!("North: {}", input.configs.north);
    println!("South: {}", input.configs.south);
    println!("East: {}", input.configs.east);
    println!("West: {}", input.configs.west);
    println!("Rows: {}", input.configs.rows);
    println!("Columns: {}", input.configs.columns);


    // let out_file = "/Users/johnlindsay/Documents/Data/AlbertaLidar/out.flt";
    //let out_file = "/Users/johnlindsay/Documents/Data/Indiana LiDAR/out.grd";

    // let mut output = Raster::initialize_using_file(&output_file, &input);
    // let out_nodata = output.configs.nodata;
    // output.configs.palette = "spectrum.plt".to_string();
    // output.add_metadata_entry("Created by the libgeospatial slope tool".to_string());

    // println!("Output file format: {:?}", output.raster_type);

    // let columns = input.configs.columns;
    // let rows = input.configs.rows;
    //
    // // let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
	// // let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
    // // let mut z_n = vec![nodata; 8];
    // let mut z: f64;
    // // let mut fy: f64;
    // // let mut fx: f64;
    // // let mut slope: f64;
    // let mut progress: usize;
    // let mut old_progress: usize = 1;
    // for row in 0..rows as isize {
    //     for col in 0..columns as isize {
    //         z = input.get_value(row, col);
    //         if z != nodata {
    //             // for i in 0..8 {
    //             //     z_n[i] = input.get_value(row + d_y[i], col + d_x[i]);
    //             //     if z_n[i] != nodata {
    //             //         z_n[i] = z_n[i] * z_conv_factor;
    //             //     } else {
    //             //         z_n[i] = z * z_conv_factor;
    //             //     }
    //             // }
    //             // fy = (z_n[6] - z_n[4] + 2.0 * (z_n[7] - z_n[3]) + z_n[0] - z_n[2]) / eight_res_y;
    //             // fx = (z_n[2] - z_n[4] + 2.0 * (z_n[1] - z_n[5]) + z_n[0] - z_n[6]) / eight_res_x;
    //             //slope = ((fx * fx + fy * fy).sqrt()).atan().to_degrees();
    //             output.set_value(row, col, z);
    //         } else {
    //             output.set_value(row, col, out_nodata);
    //         }
    //     }
    //     if verbose {
    //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
    //         if progress != old_progress {
    //             println!("Progress: {}%", progress);
    //             old_progress = progress;
    //         }
    //     }
    // }
    //
    // println!("Saving data...");
    // let _ = match output.write() {
    //     Ok(_) => println!("Complete!"),
    //     Err(e) => println!("error while writing: {:?}", e),
    // };
}
