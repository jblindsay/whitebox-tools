/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/09/2018
Last Modified: 16/06/2020
License: MIT
*/

use whitebox_common::algorithms::{is_clockwise_order, point_in_poly};
use whitebox_common::structures::Point2D;
use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to convert a vector file containing multi-part features into a vector
/// containing only single-part features. Any multi-part polygons or lines within the input
/// vector file will be split into separate features in the output file, each possessing their
/// own entry in the associated attribute file. For polygon-type vectors, the user may optionally
/// choose to exclude hole-parts from being separated from their containing polygons. That is,
/// with the `--exclude_holes` flag, hole parts in the input vector will continue to belong to
/// their enclosing polygon in the output vector. The tool will also convert MultiPoint Shapefiles
/// into single Point vectors.
///
/// # See Also
/// `SinglePartToMultiPart`
pub struct MultiPartToSinglePart {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultiPartToSinglePart {
    pub fn new() -> MultiPartToSinglePart {
        // public constructor
        let name = "MultiPartToSinglePart".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a vector file containing multi-part features into a vector containing only single-part features.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Line or Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector line or polygon file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Line or Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector line or polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Exclude hole parts?".to_owned(),
            flags: vec!["--exclude_holes".to_owned()],
            description: "Exclude hole parts from the feature splitting? (holes will continue to belong to their features in output.)".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some(String::from("true")),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.shp -o=output.shp --exclude_holes",
            short_exe, name
        )
        .replace("*", &sep);

        MultiPartToSinglePart {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultiPartToSinglePart {
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
        let mut exclude_holes = false;

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
            } else if flag_val.contains("-exc") || flag_val.contains("hole") {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    exclude_holes = true;
                }
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

        let input = Shapefile::read(&input_file)?;

        // make sure the input vector file is of polyline, polygon, or multipoint type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && input.header.shape_type.base_shape_type() != ShapeType::Polygon
            && input.header.shape_type.base_shape_type() != ShapeType::MultiPoint
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of either a POLYLINE or POLYGON base shape type.",
            ));
        }

        // create output file
        let mut output = if input.header.shape_type.base_shape_type() != ShapeType::MultiPoint {
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)
                .expect("Error while creating output file.")
        } else {
            Shapefile::initialize_using_file(&output_file, &input, ShapeType::Point, true)
                .expect("Error while creating output file.")
        };

        // add the attributes
        // output
        //     .attributes
        //     .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));
        // for att in input.attributes.get_fields() {
        //     let mut att_clone = att.clone();
        //     if att_clone.name == "FID" {
        //         att_clone.name = String::from("SRC_FID");
        //     }
        //     output.attributes.add_field(&att_clone);
        // }

        let (mut part_start, mut part_end): (usize, usize);
        // let mut fid = 1i32;

        if input.header.shape_type.base_shape_type() == ShapeType::MultiPoint {
            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                if record.shape_type != ShapeType::Null {
                    let atts = input.attributes.get_record(record_num);

                    // each point becomes a record in the output
                    for i in 0..record.points.len() {
                        output.add_point_record(record.points[i].x, record.points[i].y);
                        output.attributes.add_record(atts.clone(), false);
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
        } else if !exclude_holes || input.header.shape_type.base_shape_type() == ShapeType::PolyLine
        {
            let mut points_in_part: usize;

            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                if record.shape_type != ShapeType::Null {
                    let atts = input.attributes.get_record(record_num);
                    // atts.insert(0, FieldData::Int(fid));
                    // fid += 1;
                    for part in 0..record.num_parts as usize {
                        let mut sfg = ShapefileGeometry::new(input.header.shape_type);

                        part_start = record.parts[part] as usize;
                        part_end = if part < record.num_parts as usize - 1 {
                            record.parts[part + 1] as usize - 1
                        } else {
                            record.num_points as usize - 1
                        };

                        points_in_part = part_end - part_start + 1;

                        let mut points: Vec<Point2D> = Vec::with_capacity(points_in_part + 1);
                        for i in part_start..=part_end {
                            points.push(record.points[i].clone());
                        }

                        if input.header.shape_type.base_shape_type() == ShapeType::Polygon {
                            // make sure the points are in clockwise order
                            if !is_clockwise_order(&points) {
                                // the first part is assumed to be the hull and must be in clockwise order.
                                points.reverse();
                            }
                        }

                        sfg.add_part(&points);

                        output.add_record(sfg);

                        output.attributes.add_record(atts.clone(), false);
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
        } else {
            // polygon with holes
            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                if record.shape_type != ShapeType::Null {
                    let atts = input.attributes.get_record(record_num);
                    // atts.insert(0, FieldData::Int(fid));
                    // fid += 1;

                    let mut num_composite_features = 0;
                    for part in 0..record.num_parts {
                        if !record.is_hole(part) {
                            num_composite_features += 1;
                        }
                    }

                    if num_composite_features > 0 {
                        let mut geometries = vec![
                            ShapefileGeometry::new(input.header.shape_type);
                            num_composite_features
                        ];

                        let mut hull_vertices = vec![];

                        // Add the hulls first
                        let mut feature_num = 0usize;
                        // let mut p: Point2D;
                        for part in 0..record.num_parts as usize {
                            if !record.is_hole(part as i32) {
                                part_start = record.parts[part] as usize;
                                part_end = if part < record.num_parts as usize - 1 {
                                    record.parts[part + 1] as usize - 1
                                } else {
                                    record.num_points as usize - 1
                                };

                                geometries[feature_num]
                                    .add_part(&record.points[part_start..=part_end].to_vec());

                                hull_vertices.push(record.points[part_start..=part_end].to_vec());
                                // p = hull_vertices[feature_num][0].clone();
                                // hull_vertices[feature_num].push(p); // close the loop.

                                feature_num += 1;
                            }
                        }

                        // now add the holes to their containing hulls
                        for part in 0..record.num_parts as usize {
                            if record.is_hole(part as i32) {
                                part_start = record.parts[part] as usize;
                                part_end = if part < record.num_parts as usize - 1 {
                                    record.parts[part + 1] as usize - 1
                                } else {
                                    record.num_points as usize - 1
                                };

                                // which hull is this hole contained within?
                                for a in 0..num_composite_features {
                                    if point_in_poly(&record.points[part_start], &hull_vertices[a])
                                    {
                                        geometries[a].add_part(
                                            &record.points[part_start..=part_end].to_vec(),
                                        );
                                        break;
                                    }
                                }
                            }
                        }

                        for f in 0..num_composite_features {
                            output.add_record(geometries[f].clone());
                            output.attributes.add_record(atts.clone(), false);
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
