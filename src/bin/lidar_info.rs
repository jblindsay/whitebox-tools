#![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;

use std::env;
use std::u16;
use std::path;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut show_vlrs = false;
    let mut keyval = false;
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        keyval = false;
        if vec.len() > 1 { keyval = true; }
        if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--i" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-vlr" || vec[0].to_lowercase() == "--vlr" ||
                vec[0].to_lowercase() == "-vlrs" || vec[0].to_lowercase() == "--vlrs" {
            show_vlrs = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
            vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
                     s.push_str("-i       Input LAS file.\n");
                     s.push_str("-vlr     Flag indicates whether to print variable length records (VLRs).\n");
                     s.push_str("-wd      Optional working directory. If specified, filenames parameters need not include a full path.\n");
                     s.push_str("-version Prints the tool version number.\n");
                     s.push_str("-h       Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*lidar_info -wd *path*to*data* -i input.las -vlr\n".replace("*", &sep));
                     s.push_str(&">> .*lidar_info -i *path*to*data*input.las -vlr\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    println!("*************************");
    println!("* Welcome to lidar_info *");
    println!("*************************");

    let sep = std::path::MAIN_SEPARATOR;
    if !working_directory.ends_with(sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }

    //let input = las::LasFile::new(&input_file, "r");
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };
    println!("{}", input);

    let num_points = input.header.number_of_points;
    let mut min_i = u16::MAX;
    let mut max_i = u16::MIN;
    let mut intensity: u16;
    let mut num_first: i64 = 0;
    let mut num_last: i64 = 0;
    let mut num_only: i64 = 0;
    let mut num_intermediate: i64 = 0;
    let mut ret: u8;
    let mut nrets: u8;
    let mut p: PointData;
    let mut ret_array: [i32; 5] = [0; 5];
    let mut class_array: [i32; 256] = [0; 256];
    for i in 0..input.header.number_of_points as usize {
        p = input[i];
        ret = p.return_number();
        if ret > 5 {
            // Return is too high
            ret = 5;
        }
        ret_array[(ret - 1) as usize] += 1;
        nrets = p.number_of_returns();
        class_array[p.classification() as usize] += 1;
        if nrets == 1 {
            num_only += 1;
        } else if ret == 1 && nrets > 1 {
            num_first += 1;
        } else if ret == nrets {
            num_last += 1;
        } else {
            num_intermediate += 1;
        }
        intensity = p.intensity;
        if intensity > max_i { max_i = intensity; }
        if intensity < min_i { min_i = intensity; }
    }

    println!("\n\nMin I: {}\nMax I: {}", min_i, max_i);

    println!("\nPoint Return Table");
    for i in 0..5 {
        println!("Return {}:           {}", i + 1, ret_array[i]);
    }

    println!("\nPoint Position Table");
    println!("Only returns:         {}", num_only);
    println!("First returns:        {}", num_first);
    println!("Intermediate returns: {}", num_intermediate);
    println!("Last returns:         {}", num_last);

    println!("\nPoint Classification Table");
    for i in 0..256 {
        if class_array[i] > 0 {
            let percent: f64 = class_array[i] as f64 / num_points as f64 * 100.0;
            let percent_str = format!("{:.*}", 2, percent);
            let class_string = convert_class_val_to_class_string(i as u8);
            println!("{} ({}): {} ({}%)", class_string, i, class_array[i], percent_str);
        }

    }

    println!("\n\n{}", input.geokeys.interpret_geokeys());

    if show_vlrs {
        for i in 0..(input.header.number_of_vlrs as usize) {
            println!("\nVLR {}:\n{}", i, input.vlr_data[i].clone());
        }
    }
}
