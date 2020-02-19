/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/10/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::tools::*;
use crate::vector::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool smooths a vector coverage of either a POLYLINE or POLYGON base ShapeType. The algorithm
/// uses a simple moving average method for smoothing, where the size of the averaging window is specified
/// by the user. The default filter size is 3 and can be any odd integer larger than or equal to 3. The
/// larger the averaging window, the greater the degree of line smoothing.
pub struct SmoothVectors {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SmoothVectors {
    pub fn new() -> SmoothVectors {
        // public constructor
        let name = "SmoothVectors".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Smooths a vector coverage of either a POLYLINE or POLYGON base ShapeType.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector POLYLINE or POLYGON file.".to_owned(),
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

        parameters.push(ToolParameter {
            name: "Filter Size".to_owned(),
            flags: vec!["--filter".to_owned()],
            description:
                "The filter size, any odd integer greater than or equal to 3; default is 3."
                    .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some(String::from("3")),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=in_file.shp -o=out_file.shp --filter=9",
            short_exe, name
        )
        .replace("*", &sep);

        SmoothVectors {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SmoothVectors {
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
        let mut filter: usize = 3;

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
            } else if flag_val == "-filter" {
                filter = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
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

        if filter < 3 {
            filter = 3;
        }

        if filter % 2 == 0 {
            filter += 1;
        }

        let half_filter = (filter / 2) as i32;

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        // make sure the input vector file is of PolyLine or Polygon type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && input.header.shape_type.base_shape_type() != ShapeType::Polygon
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE or POLYGON base shape type.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;

        let (mut x, mut y): (f64, f64);
        let mut n: f64;
        let (mut start_point_in_part, mut end_point_in_part): (i32, i32);

        if input.header.shape_type.base_shape_type() == ShapeType::PolyLine {
            for record_num in 0..input.num_records {
                let in_record = input.get_record(record_num);
                let mut out_record = in_record.clone();

                for part in 0..in_record.num_parts as usize {
                    start_point_in_part = in_record.parts[part];
                    end_point_in_part = if (part as i32) < in_record.num_parts - 1 {
                        in_record.parts[part + 1] - 1
                    } else {
                        in_record.num_points - 1
                    };
                    if end_point_in_part - start_point_in_part > 2 {
                        // Notice, we are pinning the starting and ending points positions.
                        // This is because the edge effects of the filter will have the effect
                        // of shortening the polyline otherwise.
                        for i in start_point_in_part + 1..end_point_in_part {
                            n = 0f64;
                            x = 0f64;
                            y = 0f64;
                            for j in (i - half_filter)..=(i + half_filter) {
                                if j >= start_point_in_part && j <= end_point_in_part {
                                    n += 1f64;
                                    x += in_record.points[j as usize].x;
                                    y += in_record.points[j as usize].y;
                                }
                            }
                            if n > 0f64 {
                                out_record.points[i as usize].x = x / n;
                                out_record.points[i as usize].y = y / n;
                            }
                        }
                    }
                }

                output.add_record(out_record);

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
        } else {
            // Polygon
            let mut k: i32;
            for record_num in 0..input.num_records {
                let in_record = input.get_record(record_num);
                let mut out_record = in_record.clone();

                for part in 0..in_record.num_parts as usize {
                    start_point_in_part = in_record.parts[part];
                    end_point_in_part = if (part as i32) < in_record.num_parts - 1 {
                        in_record.parts[part + 1] - 1
                    } else {
                        in_record.num_points - 1
                    };
                    if end_point_in_part - start_point_in_part > 4 {
                        // We won't smooth the last point in the part. This
                        // will be set to the same position as the first
                        // after the smoothing operation.
                        for i in start_point_in_part..end_point_in_part {
                            n = 0f64;
                            x = 0f64;
                            y = 0f64;
                            for j in (i - half_filter)..=(i + half_filter) {
                                k = j;
                                if j < start_point_in_part {
                                    k = end_point_in_part - (start_point_in_part - j);
                                }
                                if j > end_point_in_part {
                                    k = start_point_in_part + (j - end_point_in_part);
                                }
                                if k >= start_point_in_part && k <= end_point_in_part {
                                    n += 1f64;
                                    x += in_record.points[k as usize].x;
                                    y += in_record.points[k as usize].y;
                                }
                            }
                            if n > 0f64 {
                                out_record.points[i as usize].x = x / n;
                                out_record.points[i as usize].y = y / n;
                            }
                        }
                        out_record.points[end_point_in_part as usize].x =
                            out_record.points[start_point_in_part as usize].x;
                        out_record.points[end_point_in_part as usize].y =
                            out_record.points[start_point_in_part as usize].y;
                    }
                }

                output.add_record(out_record);

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
