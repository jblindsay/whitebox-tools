/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 20/09/2018
Last Modified: 24/07/2020
License: MIT
*/

use crate::tools::*;
use crate::vector::*;
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool calculates the medoid for a series of vector features contained in a shapefile. The medoid
/// of a two-dimensional feature is conceptually similar its centroid, or mean position, but the medoid
/// is always a members of the input feature data set. Thus, the medoid is a measure of central tendency
/// that is robust in the presence of outliers. If the input vector is of a POLYLINE or POLYGON ShapeType,
/// the nodes of each feature will be used to estimate the feature medoid. If the input vector is of a
/// POINT base ShapeType, the medoid will be calculated for the collection of points. While there are
/// more than one competing method of calculating the medoid, this tool uses an algorithm that works as follows:
///
/// 1. The x-coordinate and y-coordinate of each point/node are placed into two arrays.
/// 2. The x- and y-coordinate arrays are then sorted and the median x-coordinate (Med X) and median
/// y-coordinate (Med Y) are calculated.
/// 3. The point/node in the dataset that is nearest the point (Med X, Med Y) is identified as the medoid.
///
/// # See Also
/// `CentroidVector`
pub struct Medoid {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Medoid {
    pub fn new() -> Medoid {
        // public constructor
        let name = "Medoid".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Calculates the medoid for a series of vector features contained in a shapefile."
                .to_string();

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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=in_file.shp -o=out_file.shp",
            short_exe, name
        )
        .replace("*", &sep);

        Medoid {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Medoid {
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

        let (mut x, mut y): (f64, f64);
        let (mut medx, mut medy): (f64, f64);
        let mut medoid: usize;
        let mut med: usize;
        let (mut dist, mut min_dist): (f64, f64);

        if input.header.shape_type.base_shape_type() == ShapeType::Point {
            // create output file
            let mut output = Shapefile::initialize_using_file(
                &output_file,
                &input,
                input.header.shape_type,
                true,
            )?;

            // read in the coordinates and find the median x and y coordinates
            let mut x_coordinates: Vec<f64> = Vec::with_capacity(input.num_records);
            let mut y_coordinates: Vec<f64> = Vec::with_capacity(input.num_records);

            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                x_coordinates.push(record.points[0].x);
                y_coordinates.push(record.points[0].y);

                if verbose {
                    progress =
                        (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            x_coordinates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
            y_coordinates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));

            med = (input.num_records as f64 / 2f64).floor() as usize;
            if input.num_records % 2 == 1 {
                // odd number; med is middle element
                medx = x_coordinates[med];
                medy = y_coordinates[med];
            } else {
                // even number; average the two middle elements.
                medx = (x_coordinates[med - 1] + x_coordinates[med]) / 2f64;
                medy = (y_coordinates[med - 1] + y_coordinates[med]) / 2f64;
            }

            // find the nearest point to the median coordinates
            min_dist = f64::INFINITY;
            medoid = 0;
            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                x = record.points[0].x;
                y = record.points[0].y;

                dist = (x - medx) * (x - medx) + (y - medy) * (y - medy);
                if dist < min_dist {
                    min_dist = dist;
                    medoid = record_num;
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

            // output the medoid point
            let record = input.get_record(medoid);
            output.add_point_record(record.points[0].x, record.points[0].y);
            let atts = input.attributes.get_record(medoid);
            output.attributes.add_record(atts.clone(), false);

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
        } else {
            // create output file
            let mut output =
                Shapefile::initialize_using_file(&output_file, &input, ShapeType::Point, true)?;

            // add the attributes
            // output
            //     .attributes
            //     .add_field(&AttributeField::new("FID", FieldDataType::Int, 2u8, 0u8));

            let mut num_points: usize;
            // output a medoid for each feature in the input file
            for record_num in 0..input.num_records {
                let record = input.get_record(record_num);
                num_points = record.points.len();
                let mut x_coordinates: Vec<f64> = Vec::with_capacity(num_points);
                let mut y_coordinates: Vec<f64> = Vec::with_capacity(num_points);
                for p in &record.points {
                    x_coordinates.push(p.x);
                    y_coordinates.push(p.y);
                }
                x_coordinates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
                y_coordinates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));

                med = (num_points as f64 / 2f64).floor() as usize;
                if input.num_records % 2 == 1 {
                    // odd number; med is middle element
                    medx = x_coordinates[med];
                    medy = y_coordinates[med];
                } else {
                    // even number; average the two middle elements.
                    medx = (x_coordinates[med - 1] + x_coordinates[med]) / 2f64;
                    medy = (y_coordinates[med - 1] + y_coordinates[med]) / 2f64;
                }

                // find the nearest point to the median coordinates
                min_dist = f64::INFINITY;
                medoid = 0;
                for i in 0..record.points.len() {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    dist = (x - medx) * (x - medx) + (y - medy) * (y - medy);
                    if dist < min_dist {
                        min_dist = dist;
                        medoid = i;
                    }
                }

                output.add_point_record(record.points[medoid].x, record.points[medoid].y);
                // output
                //     .attributes
                //     .add_record(vec![FieldData::Int(record_num as i32 + 1i32)], false);
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
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
