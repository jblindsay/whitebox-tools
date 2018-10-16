/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 20/09/2018
Last Modified: 13/10/2018
License: MIT
*/

use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use tools::*;
use vector::*;

/// This tool can be used to extend vector lines by a specified distance. The user must
/// input the names of the input and output shapefiles, the distance to extend features
/// by, and whether to extend both ends, line starts, or line ends. The input shapefile
/// must be of a POLYLINE base shape type and should be in a projected coordinate system.
pub struct ExtendVectorLines {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ExtendVectorLines {
    pub fn new() -> ExtendVectorLines {
        // public constructor
        let name = "ExtendVectorLines".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Extends vector lines by a specified distance.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector polyline file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polyline file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Extend Distance".to_owned(),
            flags: vec!["--dist".to_owned()],
            description: "The distance to extend.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Extend Direction".to_owned(),
            flags: vec!["--extend".to_owned()],
            description: "Extend direction, 'both ends' (default), 'line start', 'line end'."
                .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "both ends".to_owned(),
                "line start".to_owned(),
                "line end".to_owned(),
            ]),
            default_value: Some("both ends".to_owned()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=in_file.shp -o=out_file.shp --dist=10.0 --extend='both ends'",
            short_exe, name
        ).replace("*", &sep);

        ExtendVectorLines {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ExtendVectorLines {
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
        let mut dist: f64 = 0.0;
        let mut extend = 0;

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
            } else if flag_val == "-dist" {
                dist = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val.contains("extend") {
                let extend_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                extend = if extend_str.to_lowercase().contains("bo") {
                    // both
                    0
                } else if extend_str.to_lowercase().contains("st") {
                    // line start
                    1
                } else if extend_str.to_lowercase().contains("end") {
                    // line end
                    2
                } else {
                    // in the event that the flag is not recognized, default to both ends
                    0
                };
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        // make sure the input vector file is of polyline type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE base shape type.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;

        let (mut x1, mut x2, mut y1, mut y2): (f64, f64, f64, f64);
        let (mut x_st, mut x_end, mut y_st, mut y_end): (f64, f64, f64, f64);
        let (mut start_point_in_part, mut end_point_in_part): (usize, usize);
        let mut slope: f64;

        for record_num in 0..input.num_records {
            let mut record = input.get_record(record_num).clone();

            for part in 0..record.num_parts as usize {
                start_point_in_part = record.parts[part] as usize;
                end_point_in_part = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };

                if extend == 0 || extend == 1 {
                    // new starting point
                    x1 = record.points[start_point_in_part].x;
                    y1 = record.points[start_point_in_part].y;

                    x2 = record.points[start_point_in_part + 1].x;
                    y2 = record.points[start_point_in_part + 1].y;

                    if (x1 - x2) != 0f64 {
                        slope = (y1 - y2).atan2(x1 - x2);
                        x_st = x1 + dist * slope.cos();
                        y_st = y1 + dist * slope.sin();
                    } else {
                        x_st = x1;
                        y_st = if y2 > y1 { y1 - dist } else { y1 + dist };
                    }

                    record.points[start_point_in_part].x = x_st;
                    record.points[start_point_in_part].y = y_st;
                }

                if extend == 0 || extend == 2 {
                    // new ending point
                    x1 = record.points[end_point_in_part].x;
                    y1 = record.points[end_point_in_part].y;

                    x2 = record.points[end_point_in_part - 1].x;
                    y2 = record.points[end_point_in_part - 1].y;

                    if (x1 - x2) != 0f64 {
                        slope = (y1 - y2).atan2(x1 - x2);
                        x_end = x1 + dist * slope.cos();
                        y_end = y1 + dist * slope.sin();
                    } else {
                        x_end = x1;
                        y_end = if y2 < y1 { y1 - dist } else { y1 + dist };
                    }
                    record.points[end_point_in_part].x = x_end;
                    record.points[end_point_in_part].y = y_end;
                }
            }

            output.add_record(record);

            let atts = input.attributes.get_record(record_num);
            output.attributes.add_record(atts.clone(), false);

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
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
