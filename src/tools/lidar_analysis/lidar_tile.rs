/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/06/2017
Last Modified: 05/02/2019
License: MIT
*/
use crate::lidar::*;
use crate::tools::*;
use std;
use std::env;
use std::fs::DirBuilder;
use std::io::{Error, ErrorKind};
use std::path;
use std::path::Path;

/// This tool can be used to break a LiDAR LAS file into multiple, non-overlapping tiles, each saved as a
/// single LAS file. The user must specify the parameter of the tile grid, including its origin (`--origin_x` and
/// `--origin_y`) and the tile width and height (`--width` and `--height`). Tiles containing fewer points than
/// specified in the `--min_points` parameter will not be output. This can be useful when tiling terrestrial LiDAR
/// datasets because the low point density at the edges of the point cloud (i.e. most distant from the scan
/// station) can result in poorly populated tiles containing relatively few points.
///
/// # See Also
/// `LidarJoin`, `LidarTileFootprint`
pub struct LidarTile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarTile {
    pub fn new() -> LidarTile {
        // public constructor
        let name = "LidarTile".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Tiles a LiDAR LAS file into multiple LAS files.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Tile Width".to_owned(),
            flags: vec!["--width".to_owned()],
            description: "Width of tiles in the X dimension; default 1000.0.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1000.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Tile Height".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "Height of tiles in the Y dimension.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1000.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Origin Point X-Coordinate".to_owned(),
            flags: vec!["--origin_x".to_owned()],
            description: "Origin point X coordinate for tile grid.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Origin Point Y-Coordinate".to_owned(),
            flags: vec!["--origin_y".to_owned()],
            description: "Origin point Y coordinate for tile grid.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Minimum Number of Tile Points".to_owned(),
            flags: vec!["--min_points".to_owned()],
            description: "Minimum number of points contained in a tile for it to be saved."
                .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("2".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v -i=*path*to*data*input.las --width=1000.0 --height=2500.0 -=min_points=100", short_exe, name).replace("*", &sep);

        LidarTile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarTile {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file: String = String::new();
        let mut width_x = 1000.0;
        let mut width_y = 1000.0;
        let mut origin_x = 0.0;
        let mut origin_y = 0.0;
        let mut min_points = 2;

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-width_x" || flag_val == "-width" {
                if keyval {
                    width_x = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    width_x = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-width_y" || flag_val == "-height" {
                if keyval {
                    width_y = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    width_y = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-origin_x" {
                if keyval {
                    origin_x = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    origin_x = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-origin_y" {
                if keyval {
                    origin_y = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    origin_y = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-min_points" {
                if keyval {
                    min_points = vec[1].to_string().parse::<f32>().unwrap() as usize;
                } else {
                    min_points = args[i + 1].to_string().parse::<f32>().unwrap() as usize;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;

        if min_points < 2 {
            min_points = 2;
        }

        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if verbose {
            println!("Performing analysis...");
        }

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
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "There are too many output tiles. Try choosing a larger grid width.",
            ));
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
                if row < min_row {
                    min_row = row;
                }
                if col < min_col {
                    min_col = col;
                }
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
        DirBuilder::new()
            .recursive(true)
            .create(output_dir.clone())
            .unwrap();
        let mut num_tiles_created = 0;
        for tile_num in 0..num_tiles {
            if output_tile[tile_num] {
                row = (tile_num as f64 / cols as f64).floor() as usize;
                col = tile_num % cols;
                let output_file = format!(
                    "{}{}_row{}_col{}.las",
                    output_dir,
                    name,
                    row - min_row + 1,
                    col - min_col + 1
                );
                let mut output = LasFile::initialize_using_file(&output_file, &input);
                output.header.system_id = "EXTRACTION".to_string();

                for i in first_point_num[tile_num]..last_point_num[tile_num] {
                    if tile_data[i] == tile_num {
                        output.add_point_record(input.get_record(i));
                    }
                }
                let _ = match output.write() {
                    Ok(_) => (), // do nothing
                    Err(e) => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("Error while writing: {:?}", e),
                        ))
                    }
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
            return Err(Error::new(
                ErrorKind::Other,
                "Error: No tiles were created.",
            ));
        }

        Ok(())
    }
}
