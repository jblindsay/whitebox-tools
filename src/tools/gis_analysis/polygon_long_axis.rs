/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 14/09/2018
Last Modified: 03/03/2020
License: MIT
*/

use crate::algorithms::{minimum_bounding_box, MinimizationCriterion};
use crate::structures::Point2D;
use crate::tools::*;
use crate::vector::ShapefileGeometry;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to map the long axis of polygon features. The long axis is the
/// longer of the two primary axes of the minimum bounding box (MBB), i.e. the smallest box
/// to completely enclose a feature. The long axis is drawn for each polygon in the input
/// vector file such that it passes through the centre point of the MBB. The output file is
/// therefore a vector of simple two-point polylines forming a vector field.
pub struct PolygonLongAxis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PolygonLongAxis {
    pub fn new() -> PolygonLongAxis {
        // public constructor
        let name = "PolygonLongAxis".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "This tool can be used to map the long axis of polygon features.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polyline file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.shp",
            short_exe, name
        )
        .replace("*", &sep);

        PolygonLongAxis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PolygonLongAxis {
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

        if input.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: This tool is intended to operate on Polygon type vector files only.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, true)?;

        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            let mut points: Vec<Point2D> = Vec::with_capacity(record.num_points as usize);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
            }
            let mbb_points = minimum_bounding_box(&mut points, MinimizationCriterion::Area);

            // first, find the centre point of the mbb
            let centre = Point2D::centre_point(&mbb_points);

            // now calculate the distance between the first and second points and the second and third points
            let dist1 = mbb_points[0].distance(&mbb_points[1]);
            let dist2 = mbb_points[1].distance(&mbb_points[2]);

            let (p1, p2) = if dist1 > dist2 {
                let midpoint = Point2D::midpoint(&mbb_points[0], &mbb_points[1]);
                (
                    mbb_points[0].translate(centre.x - midpoint.x, centre.y - midpoint.y),
                    mbb_points[1].translate(centre.x - midpoint.x, centre.y - midpoint.y),
                )
            } else {
                let midpoint = Point2D::midpoint(&mbb_points[1], &mbb_points[2]);
                (
                    mbb_points[1].translate(centre.x - midpoint.x, centre.y - midpoint.y),
                    mbb_points[2].translate(centre.x - midpoint.x, centre.y - midpoint.y),
                )
            };

            points.clear();
            points.push(p1);
            points.push(p2);

            let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
            sfg.add_part(&points);
            output.add_record(sfg);

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
