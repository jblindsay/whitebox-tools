extern crate whitebox_tools;
extern crate nalgebra as na;

use std::env;
use std::f64;
use std::fs::DirBuilder;
use std::path;
use std::path::Path;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut width_x = 1000.0;
    let mut width_y = 1000.0;
    let mut origin_x = 0.0;
    let mut origin_y = 0.0;
    let mut min_points = 0;
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
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-width_x" || vec[0].to_lowercase() == "--width_x" {
            if keyval {
                width_x = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                width_x = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-width_y" || vec[0].to_lowercase() == "--width_y" {
            if keyval {
                width_y = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                width_y = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-origin_x" || vec[0].to_lowercase() == "--origin_x" {
            if keyval {
                origin_x = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                origin_x = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-origin_y" || vec[0].to_lowercase() == "--origin_y" {
            if keyval {
                origin_y = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                origin_y = args[i+1].to_string().parse::<f64>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-min_points" || vec[0].to_lowercase() == "--min_points" {
            if keyval {
                min_points = vec[1].to_string().parse::<usize>().unwrap();
            } else {
                min_points = args[i+1].to_string().parse::<usize>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
             s.push_str("-i           Input LAS file.\n");
             s.push_str("-wd          Optional working directory. If specified, input filename need not include a full path.\n");
             s.push_str("-width_x     Width of tiles in the x dimension; default 1000.0.\n");
             s.push_str("-width_y     Width of tiles in the y dimension; default 1000.0.\n");
             s.push_str("-origin_x    Origin point for tile grid, x dimension; default 0.0.\n");
             s.push_str("-origin_y    Origin point for tile grid, y dimension; default 0.0.\n");
             s.push_str("-min_points  Minimum number of points contained in a tile for it to be output; default 0.\n");
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

    let sep = std::path::MAIN_SEPARATOR;
    if !working_directory.ends_with(sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }

    lidar_tile(input_file, width_x, width_y, origin_x, origin_y, min_points, verbose);
}

fn lidar_tile(input_file: String, width_x: f64, width_y: f64, origin_x: f64, origin_y: f64, min_points: usize, verbose: bool) {
    if verbose {
        println!("*************************");
        println!("* Welcome to lidar_tile *");
        println!("*************************");
    }

    if verbose { println!("Reading input LAS file..."); }
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };

    if verbose { println!("Performing analysis..."); }

    let min_x = input.header.min_x;
    let max_x = input.header.max_x;
    let min_y = input.header.min_y;
    let max_y = input.header.max_y;
    // let min_z = input.header.min_z;
    // let max_z = input.header.max_z;

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let start_x_grid = ((min_x - origin_x) / width_x).floor();
	let end_x_grid = ((max_x - origin_x) / width_x).ceil();
	let start_y_grid = ((min_y - origin_y) / width_y).floor();
	let end_y_grid = ((max_y - origin_y) / width_y).ceil();
	let cols = (end_x_grid - start_x_grid).abs() as usize;
	let rows = (end_y_grid - start_y_grid).abs() as usize;
	let num_tiles = rows * cols;

    if num_tiles > 32767usize {
        println!("There are too many output tiles.\nChoose a larger grid width.");
        return;
    }

    let mut tile_data = vec![0usize; n_points];

    let (mut col, mut row): (usize, usize);
    let mut progress: i32;
    let mut old_progress: i32 = -1;
    for i in 0..n_points {
        let p: PointData = input[i];
        col = (((p.x - origin_x) / width_x) - start_x_grid).floor() as usize; // relative to the grid edge
        row = (((p.y - origin_y) / width_y) - start_y_grid).floor() as usize; // relative to the grid edge
        tile_data[i] = row * cols + col;

        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Creating tree: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // figure out the last point for each tile, so that the shapefile can be closed afterwards
    let n_points_plus_one = n_points + 1;
    let mut first_point_num = vec![n_points_plus_one; num_tiles];
    let mut last_point_num = vec![0usize; num_tiles];
    let mut num_points_in_tile = vec![0usize; num_tiles];

    for i in 0..n_points {
        last_point_num[tile_data[i]] = i;
        num_points_in_tile[tile_data[i]] += 1;
        if first_point_num[tile_data[i]] == n_points_plus_one {
            first_point_num[tile_data[i]] = i;
        }
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Creating tree: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let mut output_tile = vec![false; num_tiles];
    for tile_num in 0..num_tiles {
        if num_points_in_tile[tile_num] > min_points {
            output_tile[tile_num] = true;
        }
    }

    let mut min_row = 999999;
    let mut min_col = 999999;
    for tile_num in 0..num_tiles {
        if output_tile[tile_num] {
            row = (tile_num as f64 / cols as f64).floor() as usize;
            col = tile_num % cols;
            if row < min_row { min_row = row; }
            if col < min_col { min_col = col; }
        }
    }

    let sep: String = path::MAIN_SEPARATOR.to_string();
    let name: String = match Path::new(&input_file).file_stem().unwrap().to_str() {
        Some(n) => n.to_string(),
        None => "".to_string(),
    };
    let dir: String = match Path::new(&input_file).parent().unwrap().to_str() {
        Some(n) => n.to_string(),
        None => "".to_string(),
    };
    let output_dir: String = format!("{}{}{}{}", dir.to_string(), sep, name, sep);
    DirBuilder::new().recursive(true).create(output_dir.clone()).unwrap();
    let mut num_tiles_created = 0;
    for tile_num in 0..num_tiles {
        if output_tile[tile_num] {
            row = (tile_num as f64 / cols as f64).floor() as usize;
            col = tile_num % cols;
            let output_file = format!("{}{}_row{}_col{}.las", output_dir, name, row - min_row + 1, col - min_col + 1);
            let mut output = las::LasFile::initialize_using_file(&output_file, &input);
            output.header.system_id = "EXTRACTION".to_string();

            for i in first_point_num[tile_num]..last_point_num[tile_num] {
                if tile_data[i] == tile_num {
                    output.add_point_record(input.get_record(i));
                }
            }
            let _ = match output.write() {
                Ok(_) => (), // do nothing
                Err(e) => println!("Error while writing: {:?}", e),
            };
            num_tiles_created += 1;
        }

        if verbose {
            progress = (100.0_f64 * tile_num as f64 / (num_tiles - 1) as f64) as i32;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if num_tiles_created > 0 {
        if verbose {
            println!("Successfully created {} tiles.", num_tiles_created);
        }
    } else if num_tiles_created == 0 {
        panic!("Error: No tiles were created.");
    }

}
