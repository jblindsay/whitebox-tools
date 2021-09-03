/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/06/2019
Last Modified: 27/05/2020
License: MIT
*/

use crate::tools::*;
use whitebox_common::structures::Point2D;
use whitebox_vector::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to remove all features in Shapefiles that are of the `null` ShapeType. It also
/// removes line features with fewer than two vertices and polygon features with fewer than three vertices.
pub struct CleanVector {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CleanVector {
    pub fn new() -> CleanVector {
        // public constructor
        let name = "CleanVector".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Removes null features and lines/polygons with fewer than the required number of vertices.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.shp -o=output.shp",
            short_exe, name
        )
        .replace("*", &sep);

        CleanVector {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CleanVector {
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
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // read the input file
        let input = Shapefile::read(&input_file)?;

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;

        let mut num_vertices: usize;
        let (mut part_start, mut part_end): (usize, usize);
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);

            if record.shape_type != ShapeType::Null {
                num_vertices = record.points.len();
                match record.shape_type.base_shape_type() {
                    // At the moment, this is pretty crude. It would be better to do this for each
                    // part in a geometry.
                    ShapeType::PolyLine => {
                        if num_vertices > 1 {
                            output.add_record(record.clone());
                            output
                                .attributes
                                .add_record(input.attributes.get_record(record_num), false);
                        }
                    }
                    ShapeType::Polygon => {
                        let mut geometry = ShapefileGeometry::new(ShapeType::Polygon);
                        let mut something_to_add = false;
                        for part in 0..record.num_parts as usize {
                            part_start = record.parts[part] as usize;
                            part_end = if part < record.num_parts as usize - 1 {
                                record.parts[part + 1] as usize - 1
                            } else {
                                record.num_points as usize - 1
                            };

                            num_vertices = part_end - part_start + 1;

                            if num_vertices > 2 {
                                let mut points: Vec<Point2D> = Vec::with_capacity(num_vertices + 1);
                                for i in part_start..=part_end {
                                    points.push(record.points[i].clone());
                                }
                                if !record.points[part_start]
                                    .nearly_equals(&record.points[part_end])
                                {
                                    points.push(record.points[part_start].clone());
                                }

                                geometry.add_part(&points);
                                something_to_add = true;
                            }
                        }
                        if something_to_add {
                            output.add_record(geometry);
                            output
                                .attributes
                                .add_record(input.attributes.get_record(record_num), false);
                        }
                    }
                    _ => {
                        output.add_record(record.clone());
                        output
                            .attributes
                            .add_record(input.attributes.get_record(record_num), false);
                    }
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
