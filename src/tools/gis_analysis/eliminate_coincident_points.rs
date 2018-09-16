/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/09/2018
Last Modified: 16/09/2018
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::{DistanceMetric, FixedRadiusSearch2D};
use time;
use tools::*;
use vector::*;

/// This tool can be used to remove any coincident, or nearly coincident, points
/// from a vector points file. The user must specify the name of the input file,
/// which must be of a POINTS ShapeType, the output file name, and the tolerance
/// distance. All points that are within the specified tolerance distance will be
/// eliminated from the output file. A tolerance distance of 0.0 indicates that
/// points must be exactly coincident to be removed.
pub struct EliminateCoincidentPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EliminateCoincidentPoints {
    pub fn new() -> EliminateCoincidentPoints {
        // public constructor
        let name = "EliminateCoincidentPoints".to_string();
        let toolbox = "GIS Tools".to_string();
        let description =
            "Removes any coincident, or nearly coincident, points from a vector points file."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Distance Tolerance".to_owned(),
            flags: vec!["--tolerance".to_owned()],
            description: "The distance tolerance for points.".to_owned(),
            parameter_type: ParameterType::Float,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input_file.shp -o=out_file.shp --tolerance=0.01",
            short_exe, name
        ).replace("*", &sep);

        EliminateCoincidentPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EliminateCoincidentPoints {
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
        let mut tolerance = 0f64;

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
            } else if flag_val.contains("tol") {
                tolerance = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
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

        if tolerance <= 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: The tolerance must be greater than zero.",
            ));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // Get the spatial extent
        let input = Shapefile::read(&input_file)?;
        let num_points = input.num_records;

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POINT base shape type.",
            ));
        }

        let (mut x, mut y): (f64, f64);

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, ShapeType::Point, true)?;

        let mut frs: FixedRadiusSearch2D<usize> =
            FixedRadiusSearch2D::new(tolerance * 10f64, DistanceMetric::SquaredEuclidean);

        tolerance *= tolerance; // square distance threshold.

        // first fill the FRS with the hex centre points
        for record_num in 0..num_points as usize {
            let record = input.get_record(record_num);
            x = record.points[0].x;
            y = record.points[0].y;
            frs.insert(x, y, record_num);

            if verbose {
                progress = (100.0_f64 * record_num as f64 / (num_points - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Building fixed-radius search: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut excluded = vec![false; num_points];
        for record_num in 0..num_points as usize {
            let record = input.get_record(record_num);
            x = record.points[0].x;
            y = record.points[0].y;
            let ret = frs.search(x, y);
            if ret.len() > 0 {
                for p in ret {
                    if p.1 < tolerance && record_num > p.0 && !excluded[p.0] {
                        excluded[record_num] = true;
                    }
                }
            }
            if !excluded[record_num] {
                output.add_point_record(x, y);
                let atts = input.attributes.get_record(record_num);
                output.attributes.add_record(atts.clone(), false);
            }
            if verbose {
                progress = (100.0_f64 * record_num as f64 / (num_points - 1) as f64) as usize;
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
