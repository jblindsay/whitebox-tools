/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/09/2018
Last Modified: 25/09/2018
License: MIT
*/

use algorithms::{convex_hull, polygon_area};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use time;
use tools::*;
use vector::*;

/// This tool provides a measure of overall polygon shape complexity, or irregularity,
/// for vector polygons. Several shape indices have been created to compare a polygon's
/// shape to simple Euclidean shapes (e.g. circles, squares, etc.). One of the problems
/// with this approach is that it inherently convolves the characteristics of polygon
/// complexity and elongation. The Shape Complexity Index (SCI) was developed as a
/// parameter for assessing the complexity of a polygon that is independent of its
/// elongation.
///
/// SCI relates a polygon's shape to that of an encompassing convex hull. It is
/// defined as:  
///
/// `SCI = 1 - A / Ah`
///
/// Where `A` is the polygon's area and `Ah` is the area of the convex hull containing
/// the polygon. Convex polygons, i.e. those that do not contain concavities or holes,
/// have a value of 0. As the shape of the polygon becomes more complex, the SCI
/// approaches 1. Note that polygon shape complexity also increases with the greater
/// number of holes (i.e. islands), since holes have the effect of reducing the lake
/// area.
///
/// The SCI values calculated for each vector polygon feature will be placed in the
/// accompanying database file (.dbf) as a complexity field (COMPLEXITY).
pub struct ShapeComplexityIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ShapeComplexityIndex {
    pub fn new() -> ShapeComplexityIndex {
        // public constructor
        let name = "ShapeComplexityIndex".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Calculates overall polygon shape complexity or irregularity.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector polygon file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=points.shp",
            short_exe, name
        ).replace("*", &sep);

        ShapeComplexityIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ShapeComplexityIndex {
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
        let mut input_file = String::new();

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
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Shapefile::read(&input_file)?;

        let start = time::now();

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYGON base shape type.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&input_file, &input, input.header.shape_type, true)?;

        // add the attributes
        output.attributes.add_field(&AttributeField::new(
            "COMPLEXITY",
            FieldDataType::Real,
            7u8,
            5u8,
        ));

        let mut part_start: usize;
        let mut part_end: usize;
        let mut area: f64;
        let mut hull_area: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            area = 0f64;
            hull_area = 0f64;
            for part in 0..record.num_parts as usize {
                part_start = record.parts[part] as usize;
                part_end = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };
                if !record.is_hole(part as i32) {
                    area += polygon_area(&record.points[part_start..part_end]);

                    // it's also a hull
                    let mut points = record.points[part_start..part_end].to_vec();
                    let hull_points = convex_hull(&mut points);
                    hull_area += polygon_area(&hull_points);
                } else {
                    area -= polygon_area(&record.points[part_start..part_end]);
                }
            }
            let record_out = record.clone();
            output.add_record(record_out);

            let mut atts = input.attributes.get_record(record_num);
            atts.push(FieldData::Real(1f64 - area / hull_area));
            output.attributes.add_record(atts, false);

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

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time: {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
