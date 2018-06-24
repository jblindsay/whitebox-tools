/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/05/2018
Last Modified: 06/06/2018
License: MIT

NOTES: This tool thins a LiDAR point cloud such that no more than one point exists within each grid cell of a
superimposed grid of a user-specified resolution. When a cell contains more than one point in the input 
data set, the remaining point can be selected as the lowest, highest, first, last, or nearest the centre.
This tools provides similar functionality to the ESRI Thin LAS (2D) and LasTools lasthin tools. If there is
high variability in point density, consider using the LidarThinHighDesnity tool instead.
*/

use lidar::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::Array2D;
use time;
use tools::*;

/// Thins a LiDAR point cloud, reducing point density.
pub struct LidarThin {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarThin {
    pub fn new() -> LidarThin {
        // public constructor
        let name = "LidarThin".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Thins a LiDAR point cloud, reducing point density.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Sample Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description:
                "The size of the square area used to evaluate nearby points in the LiDAR data."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Point Selection Method".to_string(), 
            flags: vec!["--method".to_string()], 
            description: "Point selection method; options are 'first', 'last', 'lowest' (default), 'highest', 'nearest'.".to_string(),
            parameter_type: ParameterType::OptionList(vec!["first".to_string(), "last".to_string(), "lowest".to_string(), "highest".to_string(), "nearest".to_string()]),
            default_value: Some("lowest".to_string()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Save filtered points to seperate file?".to_owned(),
            flags: vec!["--save_filtered".to_owned()],
            description: "Save filtered points to seperate file?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.las --resolution=2.0, --method=first --save_filtered", short_exe, name).replace("*", &sep);

        LidarThin {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarThin {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut method: String = "first".to_string();
        let mut save_filtered = false;

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-method" {
                method = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                method = method.to_lowercase();
            } else if flag_val == "-save_filtered" {
                save_filtered = true;
            }
        }

        let start = time::now();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        println!("Reading input LAS file...");
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        println!("Performing analysis...");

        // Make sure that the input LAS file have GPS time data?
        if input.header.point_format == 0u8 || input.header.point_format == 2u8 {
            panic!("The input file has a Point Format that does not include GPS time, which is required for the operation of this tool.");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let west: f64 = input.header.min_x;
        let north: f64 = input.header.max_y;
        let rows: isize = (((north - input.header.min_y) / grid_res).ceil()) as isize;
        let columns: isize = (((input.header.max_x - west) / grid_res).ceil()) as isize;
        let south: f64 = north - rows as f64 * grid_res;
        let east = west + columns as f64 * grid_res;
        let half_grid_res = grid_res / 2.0;
        let ns_range = north - south;
        let ew_range = east - west;
        let mut col: isize;
        let mut row: isize;

        let mut pt_id: Array2D<usize> = Array2D::new(rows, columns, n_points, n_points)?;
        let mut prev_id: usize;
        let mut filtered = vec![false; n_points];
        let mut p: PointData;
        match &method as &str {
            "first" => {
                filtered = vec![true; n_points];
                for i in 0..n_points {
                    p = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round())
                        as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round())
                        as isize;
                    if pt_id.get_value(row, col) == n_points {
                        pt_id.set_value(row, col, i);
                        filtered[i] = false;
                    }
                    if verbose {
                        progress = (100.0_f64 * i as f64 / num_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "last" => {
                for i in 0..n_points {
                    p = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round())
                        as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round())
                        as isize;
                    prev_id = pt_id.get_value(row, col);
                    if prev_id == n_points {
                        pt_id.set_value(row, col, i);
                    } else {
                        pt_id.set_value(row, col, i);
                        filtered[prev_id] = true;
                    }
                    if verbose {
                        progress = (100.0_f64 * i as f64 / num_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "lowest" => {
                for i in 0..n_points {
                    p = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round())
                        as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round())
                        as isize;
                    prev_id = pt_id.get_value(row, col);
                    if prev_id == n_points {
                        pt_id.set_value(row, col, i);
                    } else if p.z < input.get_point_info(prev_id).z {
                        pt_id.set_value(row, col, i);
                        filtered[prev_id] = true;
                    } else {
                        filtered[i] = true;
                    }
                    if verbose {
                        progress = (100.0_f64 * i as f64 / num_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "highest" => {
                for i in 0..n_points {
                    p = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round())
                        as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round())
                        as isize;
                    prev_id = pt_id.get_value(row, col);
                    if prev_id == n_points {
                        pt_id.set_value(row, col, i);
                    } else if p.z > input.get_point_info(prev_id).z {
                        pt_id.set_value(row, col, i);
                        filtered[prev_id] = true;
                    } else {
                        filtered[i] = true;
                    }
                    if verbose {
                        progress = (100.0_f64 * i as f64 / num_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "nearest" => {
                let mut min_dist: Array2D<f64> =
                    Array2D::new(rows, columns, f64::INFINITY, -32768f64)?;
                let mut center_x: f64;
                let mut center_y: f64;
                let mut sqrd_dist: f64;
                for i in 0..n_points {
                    p = input.get_point_info(i);
                    col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range).round())
                        as isize;
                    row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range).round())
                        as isize;
                    center_x = west + half_grid_res + col as f64 * grid_res;
                    center_y = north - half_grid_res - row as f64 * grid_res;
                    sqrd_dist =
                        (p.x - center_x) * (p.x - center_x) + (p.y - center_y) * (p.y - center_y);
                    prev_id = pt_id.get_value(row, col);
                    if prev_id == n_points {
                        pt_id.set_value(row, col, i);
                        min_dist.set_value(row, col, sqrd_dist);
                    } else if sqrd_dist < min_dist.get_value(row, col) {
                        pt_id.set_value(row, col, i);
                        min_dist.set_value(row, col, sqrd_dist);
                        filtered[prev_id] = true;
                    } else {
                        filtered[i] = true;
                    }
                    if verbose {
                        progress = (100.0_f64 * i as f64 / num_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Specified 'method' parameter ({}) is not recognized.",
                        method
                    ),
                ));
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        let end;

        if !save_filtered {
            for i in 0..n_points {
                if !filtered[i] {
                    output.add_point_record(input.get_record(i));
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as usize;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            end = time::now();
        } else {
            let p = path::Path::new(&output_file);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            let filtered_output_file = output_file.replace(&extension, "_filtered_points.las");
            let mut filtered_output = LasFile::initialize_using_file(&filtered_output_file, &input);
            filtered_output.header.system_id = "EXTRACTION".to_string();

            for i in 0..n_points {
                if !filtered[i] {
                    output.add_point_record(input.get_record(i));
                } else {
                    filtered_output.add_point_record(input.get_record(i));
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as usize;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            end = time::now();

            let _ = match filtered_output.write() {
                Ok(_) => println!("Filtered points LAS file saved"),
                Err(e) => println!("error while writing: {:?}", e),
            };
        }

        let elapsed_time = end - start;

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
