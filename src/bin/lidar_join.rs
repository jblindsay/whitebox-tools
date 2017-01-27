extern crate whitebox_tools;
extern crate nalgebra as na;

use std::env;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::path;
use whitebox_tools::lidar::las;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_files: String = String::new();
    let mut output_file = String::new();
    let mut working_directory: String = String::new();
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
        if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--inputs" {
            if keyval {
                input_files = vec[1].to_string();
            } else {
                input_files = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
             s.push_str("-i           Input LAS files, separated by commas.\n");
             s.push_str("-o           Output LAS file.\n");
             s.push_str("-wd          Optional working directory. If specified, input filename need not include a full path.\n");
             s.push_str("-v           Optional verbose mode. Tool will report progress if this flag is provided.\n");
             s.push_str("-version     Prints the tool version number.\n");
             s.push_str("-h           Prints help information.\n\n");
             s.push_str("Example usage:\n\n");
             s.push_str(&">> .*lidar_tile -v -i *path*to*data*input.las -width_x 100.0 -width_y 250.0 -min_points 100\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    match run(input_files, output_file, working_directory, verbose) {
        Ok(()) => println!("Complete!"),
        Err(err) => panic!("{}", err),
    }
}

fn run(input_files: String, mut output_file: String, mut working_directory: String, verbose: bool)  -> Result<(), io::Error>{
    if verbose {
        println!("*************************");
        println!("* Welcome to lidar_join *");
        println!("*************************");
    }

    let sep = std::path::MAIN_SEPARATOR;
    if !working_directory.ends_with(sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !output_file.contains(sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }

    let mut output: las::LasFile = las::LasFile::new(&output_file, "w")?;

    let cmd = input_files.split(",");
    let vec = cmd.collect::<Vec<&str>>();
    let mut i = 0;
    let num_files = vec.len();
    let mut file_format = -1i32;
    // let mut progress: i32;
    // let mut old_progress: i32 = -1;
    for value in vec {
        if !value.trim().is_empty() {
            let mut input_file = value.trim().to_owned();
            if !input_file.contains(sep) {
                input_file = format!("{}{}", working_directory, input_file);
            }

            let input = las::LasFile::new(&input_file, "r")?;

            if file_format == -1 {
                file_format = input.header.point_format as i32;
            } else {
                if input.header.point_format as i32 != file_format {
                    return Err(Error::new(ErrorKind::InvalidData, "All input files must be of the same LAS Point Format."));
                }
            }

            if i == 0 {
                output = las::LasFile::initialize_using_file(&output_file, &input);
            }

            let n_points = input.header.number_of_points as usize;

            let mut pr: las::LidarPointRecord;
            for i in 0..n_points {
                pr = input.get_record(i);
                output.add_point_record(pr);

                // if verbose {
                //     progress = (100.0_f64 * i as f64 / num_points) as i32;
                //     if progress != old_progress {
                //         println!("Progress: {}%", progress);
                //         old_progress = progress;
                //     }
                // }
            }
        }
        i += 1;
        if verbose { println!("Adding file: {} of {}", i, num_files); }
    }

    if verbose { println!("Writing output LAS file..."); }
    output.write()?;

    Ok(())
}
