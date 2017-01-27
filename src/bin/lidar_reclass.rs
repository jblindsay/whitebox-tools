extern crate whitebox_tools;

use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::env;
use std::path;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::collections::HashMap;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::RgbData;

//use libgeospatial::lidar::point_data::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file = String::new();
    let mut reclass_file = String::new();
    let mut output_file = String::new();
    let mut working_directory = String::new();
    let mut verbose: bool = false;
    let mut byte_bit_mode: bool = true;
    let mut unclassed_value = 1u8;

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
        } else if vec[0].to_lowercase() == "-reclass_file" || vec[0].to_lowercase() == "--reclass_file" {
            if keyval {
                reclass_file = vec[1].to_string();
            } else {
                reclass_file = args[i+1].to_string();
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
        } else if vec[0].to_lowercase() == "-unclassed_value" || vec[0].to_lowercase() == "--unclassed_value" {
            if keyval {
                unclassed_value = vec[1].to_string().parse::<u8>().unwrap();
            } else {
                unclassed_value = args[i+1].to_string().parse::<u8>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-16bitmode" || vec[0].to_lowercase() == "--16bitmode" {
            byte_bit_mode = false;
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
            s.push_str("-i               Input LAS file.\n");
            s.push_str("-reclass_file    Input reclassification file. This is a CSV file of the format 'red,green,blue,class'.\n");
            s.push_str("-o               Output LAS file.\n");
            s.push_str("-wd              Optional working directory. If specified, input and output filenames need not include a full path.\n");
            s.push_str("-unclassed_value Value (0-255) assigned to points that are not specified in reclass_file. Default is 1 (unclassified)\n");
            s.push_str("-16bitmode       Use this optional flag only when 16-bit RGB values are used in the reclass_file. If unspecified, 8-bit colour is assumed.");
            s.push_str("-v               Optional verbose mode. Tool will report progress if this flag is provided.\n");
            s.push_str("-version         Prints the tool version number.\n");
            s.push_str("-h               Prints help information.\n\n");
            s.push_str("Example usage:\n\n");
            s.push_str(&">> .*lidar_reclass -wd *path*to*data* -i input.las -reclass_file reclass.txt -o output.las -unclassed_value 1 -v\n".replace("*", &sep));
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
    if !reclass_file.contains(sep) {
        reclass_file = format!("{}{}", working_directory, reclass_file);
    }

    match run(input_file, reclass_file, output_file, verbose, byte_bit_mode, unclassed_value) {
        Ok(()) => println!("Complete!"),
        Err(err) => panic!("Error: {}", err),
    }
}

fn run(input_file: String, reclass_file: String, output_file: String, verbose: bool,
    byte_bit_mode: bool, unclassed_value: u8) -> Result<(), io::Error> {
    if verbose {
        println!("****************************");
        println!("* Welcome to lidar_reclass *");
        println!("****************************");
    }

    if verbose { println!("Reading input LAS file..."); }
    // read the input LAS file
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };
    let point_format = input.header.point_format;

    // read the reclass file
    let f = File::open(reclass_file)?;
    let f = BufReader::new(f);

    let mut reclass_data = HashMap::new();
    if byte_bit_mode {
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            if !line_unwrapped.contains("r") && !line_unwrapped.contains("g")
               && !line_unwrapped.contains("b") && !line_unwrapped.contains("class") {
                let line_split = line_unwrapped.split(",");
                let vec = line_split.collect::<Vec<&str>>();
                let r = vec[0].trim().to_string().parse::<u8>().unwrap() as u16 * 256u16;
                let g = vec[1].trim().to_string().parse::<u8>().unwrap() as u16 * 256u16;
                let b = vec[2].trim().to_string().parse::<u8>().unwrap() as u16 * 256u16;
                let cls = vec[3].trim().to_string().parse::<u8>().unwrap();
                reclass_data.insert(RgbData{ red: r, green: g, blue: b }, cls);
            }
        }
    } else {
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            if !line_unwrapped.contains("r") && !line_unwrapped.contains("g")
               && !line_unwrapped.contains("b") && !line_unwrapped.contains("class") {
                let line_split = line_unwrapped.split(",");
                let vec = line_split.collect::<Vec<&str>>();
                let r = vec[0].trim().to_string().parse::<u16>().unwrap();
                let g = vec[1].trim().to_string().parse::<u16>().unwrap();
                let b = vec[2].trim().to_string().parse::<u16>().unwrap();
                let cls = vec[3].trim().to_string().parse::<u8>().unwrap();
                reclass_data.insert(RgbData{ red: r, green: g, blue: b }, cls);
            }
        }
    }

    // create the output LAS file
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);

    if point_format == 2 || point_format == 3 {
        let n_points = input.header.number_of_points as usize;
        let num_points = (n_points - 1) as f64;
        let mut progress: i32;
        let mut old_progress: i32 = -1;

        //let mut p: PointData;
        let mut pr: las::LidarPointRecord;
        let mut pr2: las::LidarPointRecord;
        let mut rgb: RgbData;
        let mut class_val: u8;
        for i in 0..n_points {
            rgb = input.get_rgb(i).unwrap();

            match reclass_data.get(&rgb) {
                Some(&cls) => class_val = cls,
                _ => class_val = unclassed_value,
            }

            pr = input.get_record(i);
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
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose { println!("Writing output LAS file..."); }
        output.write()?;
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "The input LAS file does not contain RGB colour data."));
    }
    Ok(())
}
