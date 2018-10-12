/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 14/09/2018
Last Modified: 31/09/2018
License: MIT
*/

use algorithms::{minimum_bounding_box, MinimizationCriterion};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use structures::Point2D;
use time;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// This tool delineates the minimum bounding box (MBB) for a group of vectors. The MBB is the smallest box to
/// completely enclose a feature. The algorithm works by rotating the feature, calculating the axis-aligned
/// bounding box for each rotation, and finding the box with the smallest area, length, width, or perimeter. The
/// MBB is needed to compute several shape indices, such as the Elongation Ratio. The `MinimumBoundingEnvelop`
/// tool can be used to calculate the axis-aligned bounding rectangle around each feature in a vector file.
///
/// # See Also
/// `MinimumBoundingCircle`, `MinimumBoundingEnvelope`, `MinimumConvexHull`
pub struct MinimumBoundingBox {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MinimumBoundingBox {
    pub fn new() -> MinimumBoundingBox {
        // public constructor
        let name = "MinimumBoundingBox".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a vector minimum bounding rectangle around vector features.".to_string();

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
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimization Criterion".to_owned(),
            flags: vec!["--criterion".to_owned()],
            description: "Minimization criterion; options include 'area' (default), 'length', 'width', and 'perimeter'."
                .to_owned(),
            parameter_type: ParameterType::OptionList(vec!["area".to_owned(), "length".to_owned(), "width".to_owned(), "perimeter".to_owned()]),
            default_value: Some("area".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Find bounding rectangles around each individual feature.".to_owned(),
            flags: vec!["--features".to_owned()],
            description:
                "Find the minimum bounding rectangles around each individual vector feature"
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.shp --criterion=length --features",
            short_exe, name
        ).replace("*", &sep);

        MinimumBoundingBox {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MinimumBoundingBox {
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
        let mut individual_feature_hulls = false;
        let mut min_criterion = MinimizationCriterion::Area;

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
            } else if flag_val == "-criterion" {
                let criteria_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                min_criterion = if criteria_str.contains("len") {
                    MinimizationCriterion::Length
                } else if criteria_str.contains("wi") {
                    MinimizationCriterion::Width
                } else if criteria_str.contains("per") {
                    MinimizationCriterion::Perimeter
                } else {
                    MinimizationCriterion::Area
                };
            } else if flag_val == "-features" || flag_val == "-feature" {
                individual_feature_hulls = true;
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = time::now();

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

        if input.header.shape_type.base_shape_type() == ShapeType::Point {
            // Finding hulls around individual points makes no sense. Likely
            // the user didn't intend to supply the --hull flag.
            individual_feature_hulls = false;
        }

        if individual_feature_hulls {
            // create output file
            let mut output =
                Shapefile::initialize_using_file(&output_file, &input, ShapeType::Polygon, true)?;

            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                let mut points: Vec<Point2D> = Vec::with_capacity(record.num_points as usize);
                for i in 0..record.num_points as usize {
                    points.push(Point2D::new(record.points[i].x, record.points[i].y));
                }
                let mut mbb_points = minimum_bounding_box(&mut points, min_criterion);
                // now add a last point same as the first.
                let p = mbb_points[0];
                mbb_points.push(p);

                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                sfg.add_part(&mbb_points);
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
                Ok(_) => if verbose {
                    println!("Output file written")
                },
                Err(e) => return Err(e),
            };
        } else {
            // create output file
            let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;
            output.projection = input.projection.clone();

            // add the attributes
            let fid = AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8);
            output.attributes.add_field(&fid);

            let mut points: Vec<Point2D> = vec![];
            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                for i in 0..record.num_points as usize {
                    points.push(Point2D::new(record.points[i].x, record.points[i].y));
                }

                if verbose {
                    progress =
                        (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                    if progress != old_progress {
                        println!("Reading points: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            if verbose {
                println!("Finding convex hull...");
            }
            let mut mbb_points = minimum_bounding_box(&mut points, min_criterion);
            // now add a last point same as the first.
            let p = mbb_points[0];
            mbb_points.push(p);

            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
            sfg.add_part(&mbb_points);
            output.add_record(sfg);
            output
                .attributes
                .add_record(vec![FieldData::Int(1i32)], false);

            if verbose {
                println!("Saving data...")
            };
            let _ = match output.write() {
                Ok(_) => if verbose {
                    println!("Output file written")
                },
                Err(e) => return Err(e),
            };
        }

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
