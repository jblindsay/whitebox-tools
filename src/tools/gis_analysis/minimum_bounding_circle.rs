/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 14/09/2018
Last Modified: 13/10/2018
License: MIT
*/

use algorithms::smallest_enclosing_circle;
use std::env;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use structures::Point2D;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// This tool delineates the minimum bounding circle (MBC) for a group of vectors. The MBC is the smallest enclosing
/// circle to completely enclose a feature.
///
/// # See Also
/// `MinimumBoundingBox`, `MinimumBoundingEnvelope`, `MinimumConvexHull`
pub struct MinimumBoundingCircle {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MinimumBoundingCircle {
    pub fn new() -> MinimumBoundingCircle {
        // public constructor
        let name = "MinimumBoundingCircle".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Delineates the minimum bounding circle (i.e. smallest enclosing circle) for a group of vectors.".to_string();

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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.shp --features",
            short_exe, name
        ).replace("*", &sep);

        MinimumBoundingCircle {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MinimumBoundingCircle {
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
            } else if flag_val == "-features" || flag_val == "-feature" {
                individual_feature_hulls = true;
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

        if input.header.shape_type.base_shape_type() == ShapeType::Point {
            // Finding hulls around individual points makes no sense. Likely
            // the user didn't intend to supply the --hull flag.
            individual_feature_hulls = false;
        }

        let num_circle_vertices = 128usize;
        let angular_resolution = 2f64 * PI / num_circle_vertices as f64;

        if individual_feature_hulls {
            // create output file
            let mut output =
                Shapefile::initialize_using_file(&output_file, &input, ShapeType::Polygon, true)?;

            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);

                let mbc = smallest_enclosing_circle(&record.points);

                let mut points: Vec<Point2D> = Vec::with_capacity(num_circle_vertices + 1);
                let (mut x, mut y): (f64, f64);
                let mut slope: f64;
                for i in 0..num_circle_vertices {
                    slope = i as f64 * angular_resolution;
                    x = mbc.center.x + mbc.radius * slope.sin();
                    y = mbc.center.y + mbc.radius * slope.cos();
                    points.push(Point2D::new(x, y));
                }

                // now add a last point same as the first.
                slope = 0f64;
                x = mbc.center.x + mbc.radius * slope.sin();
                y = mbc.center.y + mbc.radius * slope.cos();
                points.push(Point2D::new(x, y));

                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
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
                println!("Finding minimum bounding circle...");
            }
            let mbc = smallest_enclosing_circle(&points);

            let mut points: Vec<Point2D> = Vec::with_capacity(num_circle_vertices + 1);
            let (mut x, mut y): (f64, f64);
            let mut slope: f64;
            for i in 0..num_circle_vertices {
                slope = i as f64 * angular_resolution;
                x = mbc.center.x + mbc.radius * slope.sin();
                y = mbc.center.y + mbc.radius * slope.cos();
                points.push(Point2D::new(x, y));
            }

            // now add a last point same as the first.
            slope = 0f64;
            x = mbc.center.x + mbc.radius * slope.sin();
            y = mbc.center.y + mbc.radius * slope.cos();
            points.push(Point2D::new(x, y));

            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
            sfg.add_part(&points);
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

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
