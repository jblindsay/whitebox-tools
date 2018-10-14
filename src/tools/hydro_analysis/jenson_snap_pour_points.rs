/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: 12/10/2018
License: MIT
*/

use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::isize;
use std::path;
use tools::*;
use vector::*;

pub struct JensonSnapPourPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl JensonSnapPourPoints {
    pub fn new() -> JensonSnapPourPoints {
        // public constructor
        let name = "JensonSnapPourPoints".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Moves outlet points used to specify points of interest in a watershedding operation to the nearest stream cell.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Pour Points (Outlet) File".to_owned(),
            flags: vec!["--pour_pts".to_owned()],
            description: "Input vector pour points (outlet) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Snap Distance (map units)".to_owned(),
            flags: vec!["--snap_dist".to_owned()],
            description: "Maximum snap distance in map units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --pour_pts='pour_pts.shp' --streams='streams.tif' -o='output.shp' --snap_dist=15.0", short_exe, name).replace("*", &sep);

        JensonSnapPourPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for JensonSnapPourPoints {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut pourpts_file = String::new();
        let mut streams_file = String::new();
        let mut output_file = String::new();
        let mut snap_dist = 0.0;

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
            if flag_val == "-pour_pts" {
                pourpts_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
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
            } else if flag_val == "-snap_dist" {
                snap_dist = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !pourpts_file.contains(&sep) && !pourpts_file.contains("/") {
            pourpts_file = format!("{}{}", working_directory, pourpts_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        // let pourpts = Raster::new(&pourpts_file, "r")?;
        let pourpts = Shapefile::read(&pourpts_file)?;

        // make sure the input vector file is of points type
        if pourpts.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let nodata = streams.configs.nodata;

        let mut output =
            Shapefile::initialize_using_file(&output_file, &pourpts, ShapeType::Point, true)?;

        let snap_dist_int: isize =
            ((snap_dist / streams.configs.resolution_x) / 2.0).floor() as isize;

        let mut dist: f64;
        let mut min_dist: f64;
        let mut zn: f64;
        let (mut row, mut col): (isize, isize);
        let (mut xn, mut yn): (f64, f64);
        let (mut x, mut y): (f64, f64);
        for record_num in 0..pourpts.num_records {
            let record = pourpts.get_record(record_num);
            let attr_rec = pourpts.attributes.get_record(record_num);
            output
                .attributes
                .add_record(attr_rec, pourpts.attributes.is_deleted[record_num]);
            row = streams.get_row_from_y(record.points[0].y);
            col = streams.get_column_from_x(record.points[0].x);
            min_dist = f64::INFINITY;
            xn = record.points[0].x;
            yn = record.points[0].y;
            for c in (col - snap_dist_int)..(col + snap_dist_int + 1) {
                for r in (row - snap_dist_int)..(row + snap_dist_int + 1) {
                    zn = streams.get_value(r, c);
                    if zn > 0f64 && zn != nodata {
                        // it's a stream
                        x = streams.get_x_from_column(c);
                        y = streams.get_y_from_row(r);
                        dist = (x - record.points[0].x) * (x - record.points[0].x)
                            + (y - record.points[0].y) * (y - record.points[0].y); // actually squared-dist
                        if dist < min_dist {
                            min_dist = dist;
                            xn = x;
                            yn = y;
                        }
                    }
                }
            }
            output.add_point_record(xn, yn);
            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (pourpts.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let rows = pourpts.configs.rows as isize;
        // let columns = pourpts.configs.columns as isize;
        // let nodata = pourpts.configs.nodata;
        // let streams_nodata = streams.configs.nodata;

        // // make sure the input files have the same size
        // if pourpts.configs.rows != streams.configs.rows
        //     || pourpts.configs.columns != streams.configs.columns
        // {
        //     return Err(Error::new(
        //         ErrorKind::InvalidInput,
        //         "The input files must have the same number of rows and columns and spatial extent.",
        //     ));
        // }

        // let snap_dist_int: isize =
        //     ((snap_dist / pourpts.configs.resolution_x) / 2.0).floor() as isize;

        // let mut output = Raster::initialize_using_file(&output_file, &pourpts);

        // let mut outlet_id: f64;
        // let mut min_dist: isize;
        // let mut dist: isize;
        // let mut zn: f64;
        // let mut xn: isize;
        // let mut yn: isize;
        // for row in 0..rows {
        //     for col in 0..columns {
        //         outlet_id = pourpts[(row, col)];
        //         if outlet_id > 0.0 && outlet_id != nodata {
        //             min_dist = isize::MAX;
        //             xn = col;
        //             yn = row;
        //             for x in (col - snap_dist_int)..(col + snap_dist_int + 1) {
        //                 for y in (row - snap_dist_int)..(row + snap_dist_int + 1) {
        //                     zn = streams[(y, x)];
        //                     if zn > 0.0 && zn != streams_nodata {
        //                         // it's a stream
        //                         dist = (x - col) * (x - col) + (y - row) * (y - row); // actually squared-dist
        //                         if dist < min_dist {
        //                             min_dist = dist;
        //                             xn = x;
        //                             yn = y;
        //                         }
        //                     }
        //                 }
        //             }
        //             output[(yn, xn)] = outlet_id;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Initializing: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let elapsed_time = get_formatted_elapsed_time(start);
        // output.add_metadata_entry(format!(
        //     "Created by whitebox_tools\' {} tool",
        //     self.get_tool_name()
        // ));
        // output.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
        // output.add_metadata_entry(format!("Streams file: {}", streams_file));
        // output.add_metadata_entry(format!("Snap distance: {}", snap_dist));
        // output.add_metadata_entry(
        //     format!("Elapsed Time (excluding I/O): {}", elapsed_time)
        // );

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
