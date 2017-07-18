/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: July 17, 2017
License: MIT
*/
use std;
use std::env;
use std::io::{Error, ErrorKind};
use std::fs::DirBuilder;
use std::path;
use std::path::Path;
use lidar::*;
// use lidar::point_data::*;
use tools::WhiteboxTool;

pub struct LidarTile {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarTile {
    pub fn new() -> LidarTile { // public constructor
        let name = "LidarTile".to_string();
        
        let description = "Tiles a LiDAR LAS file into multiple LAS files.".to_string();
        
        let mut parameters = "-i, --input    Input LAS file.\n".to_owned();
        parameters.push_str("-width_x      Width of tiles in the x dimension; default 1000.0.\n");
        parameters.push_str("--width_y     Width of tiles in the y dimension; default 1000.0.\n");
        parameters.push_str("--origin_x    Origin point for tile grid, x dimension; default 0.0.\n");
        parameters.push_str("--origin_y    Origin point for tile grid, y dimension; default 0.0.\n");
        parameters.push_str("--min_points  Minimum number of points contained in a tile for it to be output; default 0.\n");
             
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v -i=*path*to*data*input.las --width_x=1000.0 --width_y=2500.0 -=min_points=100", short_exe, name).replace("*", &sep);
    
        LidarTile { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarTile {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file: String = String::new();
        let mut width_x = 1000.0;
        let mut width_y = 1000.0;
        let mut origin_x = 0.0;
        let mut origin_y = 0.0;
        let mut min_points = 0;

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 { keyval = true; }
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
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
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;

        if !input_file.contains(sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if verbose { println!("Performing analysis..."); }

        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let min_x = input.header.min_x;
        let max_x = input.header.max_x;
        let min_y = input.header.min_y;
        let max_y = input.header.max_y;

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
            return Err(Error::new(ErrorKind::InvalidInput, "There are too many output tiles. Try choosing a larger grid width."));
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
                    println!("Progress (Loop 1 of 3): {}%", progress);
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
                    println!("Progress (Loop 2 of 3): {}%", progress);
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
                let mut output = LasFile::initialize_using_file(&output_file, &input);
                output.header.system_id = "EXTRACTION".to_string();

                for i in first_point_num[tile_num]..last_point_num[tile_num] {
                    if tile_data[i] == tile_num {
                        output.add_point_record(input.get_record(i));
                    }
                }
                let _ = match output.write() {
                    Ok(_) => (), // do nothing
                    Err(e) => return Err(Error::new(ErrorKind::Other, format!("Error while writing: {:?}", e))),
                };
                num_tiles_created += 1;
            }

            if verbose {
                progress = (100.0_f64 * tile_num as f64 / (num_tiles - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress (Loop 3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if num_tiles_created > 0 {
            if verbose {
                println!("Successfully created {} tiles.", num_tiles_created);
            }
        } else if num_tiles_created == 0 {
            return Err(Error::new(ErrorKind::Other, "Error: No tiles were created."));
        }

        Ok(())
    }
}
