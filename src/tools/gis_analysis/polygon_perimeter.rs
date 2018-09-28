/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/09/2018
Last Modified: 25/09/2018
License: MIT
*/

use algorithms::polygon_perimeter;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use time;
use tools::*;
use vector::*;

/// This tool calculates the perimeter of vector polygons, adding the result
/// to the vector's attribute table (PERIMETER field). The area calculation will
/// account for any holes contained within polygons. The vector should be in a
/// a projected coordinate system.
pub struct PolygonPerimeter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PolygonPerimeter {
    pub fn new() -> PolygonPerimeter {
        // public constructor
        let name = "PolygonPerimeter".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Calculates the perimeter of vector polygons.".to_string();

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
        ).replace("*", &sep);

        PolygonPerimeter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PolygonPerimeter {
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
            "PERIMETER",
            FieldDataType::Real,
            12u8,
            4u8,
        ));

        let mut part_start: usize;
        let mut part_end: usize;
        let mut perimeter: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            perimeter = 0f64;
            for part in 0..record.num_parts as usize {
                part_start = record.parts[part] as usize;
                part_end = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };
                perimeter += polygon_perimeter(&record.points[part_start..part_end]);
            }
            let record_out = record.clone();
            output.add_record(record_out);

            let mut atts = input.attributes.get_record(record_num);
            atts.push(FieldData::Real(perimeter));
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
