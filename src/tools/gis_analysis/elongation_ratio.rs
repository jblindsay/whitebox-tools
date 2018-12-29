/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/09/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::algorithms::{minimum_bounding_box, MinimizationCriterion};
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to calculate the elongation ratio for vector polygons. The
/// elongation ratio values calculated for each vector polygon feature will be placed
/// in the accompanying database file (.dbf) as an elongation field (ELONGATION).
///
/// The elongation ratio (`E`) is:
///
/// `E = 1 - S / L`
///
/// Where `S` is the short-axis length, and `L` is the long-axis length. Axes
/// lengths are determined by estimating the minimum bounding box.
///
/// The elongation ratio provides similar information as the Linearity Index. The
/// ratio is not an adequate measure of overall polygon narrowness, because a highly
/// sinuous but narrow polygon will have a low linearity (elongation) owing to the
/// compact nature of these polygon.
pub struct ElongationRatio {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ElongationRatio {
    pub fn new() -> ElongationRatio {
        // public constructor
        let name = "ElongationRatio".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description = "Calculates the elongation ratio for vector polygons.".to_string();

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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=polygons.shp",
            short_exe, name
        )
        .replace("*", &sep);

        ElongationRatio {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ElongationRatio {
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

        let start = Instant::now();

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
            "ELONGATION",
            FieldDataType::Real,
            12u8,
            4u8,
        ));

        let mut elongation: f64;
        let mut short_axis: f64;
        let mut long_axis: f64;
        let mut dist1: f64;
        let mut dist2: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);

            let mut points: Vec<Point2D> = Vec::with_capacity(record.num_points as usize);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
            }
            let mbb_points = minimum_bounding_box(&mut points, MinimizationCriterion::Area);

            // now calculate the distance between the first and second points and the second and third points
            dist1 = mbb_points[0].distance(&mbb_points[1]);
            dist2 = mbb_points[1].distance(&mbb_points[2]);

            short_axis = dist1.min(dist2);
            long_axis = dist1.max(dist2);

            elongation = 1f64 - short_axis / long_axis;

            let record_out = record.clone();
            output.add_record(record_out);

            let mut atts = input.attributes.get_record(record_num);
            atts.push(FieldData::Real(elongation));
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
