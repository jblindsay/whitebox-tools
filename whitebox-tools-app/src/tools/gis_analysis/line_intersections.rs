/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/10/2018
Last Modified: 16/10/2018
License: MIT
*/

use whitebox_common::algorithms::find_line_intersections;
use whitebox_common::structures::{BoundingBox, Point2D};
use crate::tools::*;
use whitebox_vector::*;
use num_cpus;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool identifies points where the features of two vector line/polygon layers
/// intersect. The user must specify the names of two input vector line files and the
/// output file. The output file will be a vector of POINT ShapeType. If the input
/// vectors intersect at a line segment, the beginning and end vertices of the segment
/// will be present in the output file. A warning is issued if intersection line segments
/// are identified during analysis. If no intersections are found between the input line
/// files, the output file will not be saved and a warning will be issued.
///
/// Each intersection point will contain `PARENT1` and `PARENT2` attribute fields,
/// identifying the instersecting features in the first and second input line files
/// respectively. Additionally, the output attribute table will contain all of the
/// attributes (excluding `FID`s) of the two parent line features.
pub struct LineIntersections {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LineIntersections {
    pub fn new() -> LineIntersections {
        // public constructor
        let name = "LineIntersections".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Identifies points where the features of two vector line layers intersect.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input vector polyline file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::LineOrPolygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input vector polyline file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::LineOrPolygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector Point File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector point file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=lines1.shp --i2=lines2.shp -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        LineIntersections {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LineIntersections {
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
        let mut input1_file: String = "".to_string();
        let mut input2_file: String = "".to_string();
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
            if flag_val == "-i1" || flag_val == "-input1" {
                input1_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-i2" || flag_val == "-input2" {
                input2_file = if keyval {
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

        if !input1_file.contains(path::MAIN_SEPARATOR) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }

        if !input2_file.contains(path::MAIN_SEPARATOR) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input1 = Arc::new(Shapefile::read(&input1_file)?);

        // make sure the input vector file is of polyline type
        if input1.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && input1.header.shape_type.base_shape_type() != ShapeType::Polygon
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE or POLYGON base shape type.",
            ));
        }

        let input2 = Arc::new(Shapefile::read(&input2_file)?);

        // make sure the input vector file is of polyline type
        if input2.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && input2.header.shape_type.base_shape_type() != ShapeType::Polygon
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE or POLYGON base shape type.",
            ));
        }

        // Create lists of imported attributes from each input.
        // Append all fields except the ubiquitous FID field
        let mut input1_attributes = vec![];
        let mut input1_att_nums = vec![];
        for a in 0..input1.attributes.get_num_fields() {
            let f = input1.attributes.get_field(a);
            if f.name.to_lowercase() != "fid" {
                input1_attributes.push(f.clone());
                input1_att_nums.push(a);
            }
        }

        let mut input2_attributes = vec![];
        let mut input2_att_nums = vec![];
        for a in 0..input2.attributes.get_num_fields() {
            let f = input2.attributes.get_field(a);
            if f.name.to_lowercase() != "fid" {
                input2_attributes.push(f.clone());
                input2_att_nums.push(a);
            }
        }

        // Get the bounding boxes of each of the features in input1 and input 2
        let mut bb1: Vec<BoundingBox> = Vec::with_capacity(input1.num_records);
        for record_num in 0..input1.num_records {
            let record = input1.get_record(record_num);
            bb1.push(record.get_bounding_box());
        }

        let mut bb2: Vec<BoundingBox> = Vec::with_capacity(input2.num_records);
        for record_num in 0..input2.num_records {
            let record = input2.get_record(record_num);
            bb2.push(record.get_bounding_box());
        }

        // multithreading setup
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let feature_list = Arc::new(Mutex::new(0..input1.num_records));
        let bb1 = Arc::new(bb1);
        let bb2 = Arc::new(bb2);

        // hunt for intersections in the overlapping bounding boxes
        for _ in 0..num_procs {
            let input1 = input1.clone();
            let input2 = input2.clone();
            let bb1 = bb1.clone();
            let bb2 = bb2.clone();
            let feature_list = feature_list.clone();
            let tx1 = tx1.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let mut start_point_in_part1: usize;
                let mut start_point_in_part2: usize;
                let mut end_point_in_part1: usize;
                let mut end_point_in_part2: usize;
                let mut print_warning = false;
                let mut record_num1 = 0;
                let mut intersection_points: Vec<(Point2D, usize, usize)> = vec![];
                while record_num1 < input1.num_records {
                    let mut num_intersections = 0;
                    // Get the next tile up for interpolation
                    record_num1 = match feature_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to interpolate
                    };

                    let record1 = input1.get_record(record_num1);
                    for record_num2 in 0..input2.num_records {
                        if bb1[record_num1].overlaps(bb2[record_num2]) {
                            // find any intersections between the contained geometries
                            let record2 = input2.get_record(record_num2);
                            for part1 in 0..record1.num_parts as usize {
                                start_point_in_part1 = record1.parts[part1] as usize;
                                end_point_in_part1 = if part1 < record1.num_parts as usize - 1 {
                                    record1.parts[part1 + 1] as usize - 1
                                } else {
                                    record1.num_points as usize - 1
                                };

                                for part2 in 0..record2.num_parts as usize {
                                    start_point_in_part2 = record2.parts[part2] as usize;
                                    end_point_in_part2 = if part2 < record2.num_parts as usize - 1 {
                                        record2.parts[part2 + 1] as usize - 1
                                    } else {
                                        record2.num_points as usize - 1
                                    };

                                    let intersections = find_line_intersections(
                                        &(record1.points
                                            [start_point_in_part1..=end_point_in_part1]),
                                        &(record2.points
                                            [start_point_in_part2..=end_point_in_part2]),
                                    );
                                    for ls in intersections {
                                        // the intersection is a point
                                        intersection_points.push((ls.p1, record_num1, record_num2));
                                        num_intersections += 1;
                                        if ls.p1 != ls.p2 {
                                            // the intersection is a line segment
                                            intersection_points.push((
                                                ls.p2,
                                                record_num1,
                                                record_num2,
                                            ));
                                            print_warning = true;
                                            num_intersections += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tx2.send(num_intersections).unwrap();
                }
                tx1.send((intersection_points.clone(), print_warning))
                    .unwrap();
            });
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input1, ShapeType::Point, false)?;

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        output.attributes.add_field(&AttributeField::new(
            "PARENT1",
            FieldDataType::Int,
            7u8,
            0u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "PARENT2",
            FieldDataType::Int,
            7u8,
            0u8,
        ));

        for a in 0..input1_attributes.len() {
            output.attributes.add_field(&input1_attributes[a].clone());
        }
        for a in 0..input2_attributes.len() {
            output.attributes.add_field(&input2_attributes[a].clone());
        }

        // receive the points
        let mut print_warning = false;
        let mut record_num1: usize;
        let mut record_num2: usize;
        let mut fid = 1i32;
        for _ in 0..num_procs {
            let (data, pw) = rx1.recv().unwrap();
            if pw {
                print_warning = true;
            }
            for i in 0..data.len() {
                output.add_point_record(data[i].0.x, data[i].0.y);

                // Add the attributes
                record_num1 = data[i].1;
                record_num2 = data[i].2;
                let mut atts = vec![
                    FieldData::Int(fid),
                    FieldData::Int(record_num1 as i32 + 1i32),
                    FieldData::Int(record_num2 as i32 + 1i32),
                ];

                let atts1 = input1.attributes.get_record(record_num1);
                for a in 0..input1_att_nums.len() {
                    atts.push(atts1[input1_att_nums[a]].clone());
                }
                let atts2 = input2.attributes.get_record(record_num2);
                for a in 0..input2_att_nums.len() {
                    atts.push(atts2[input2_att_nums[a]].clone());
                }
                output.attributes.add_record(atts, false);
                fid += 1;
            }
        }

        let mut num_intersections = 0;
        for i in 0..input1.num_records {
            let found_intersections = rx2.recv().unwrap();
            num_intersections += found_intersections;
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / input1.num_records as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Progress ({} intersections found): {}%",
                        num_intersections, progress
                    );
                    old_progress = progress;
                }
            }
        }

        if print_warning {
            println!("Warning: Some of the input line features intersect at line segments rather than points.")
        }

        if fid == 1 {
            println!("Warning: No intersections were found between the input features.")
        } else {
            // Some features were found. Save the output file.
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
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
