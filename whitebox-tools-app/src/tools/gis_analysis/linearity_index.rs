/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/10/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool calculates the linearity index of polygon features based on a regression analysis.
/// The index is simply the coefficient of determination (r-squared) calculated from a regression
/// analysis of the x and y coordinates of the exterior hull nodes of a vector polygon. Linearity
/// index is a measure of how well a polygon can be described by a straight line. It is a related
/// index to the `ElongationRatio`, but is more efficient to calculate as it does not require
/// finding the minimum bounding box. The Pearson correlation coefficient between linearity index
/// and the elongation ratio for a large data set of lake polygons in northern Canada was found
/// to be 0.656, suggesting a moderate level of association between the two measures of polygon
/// linearity. Note that this index is not useful for identifying narrow yet sinuous polygons, such
/// as meandering rivers.
///
/// The only required input is the name of the file. The linearity values calculated for each vector
/// polygon feature will be placed in the accompanying attribute table as a new field (LINEARITY).
///
/// # See Also
/// `ElongationRatio`, `PatchOrientation`
pub struct LinearityIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LinearityIndex {
    pub fn new() -> LinearityIndex {
        // public constructor
        let name = "LinearityIndex".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description = "Calculates the linearity index for vector polygons.".to_string();

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

        LinearityIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LinearityIndex {
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
            "LINEARITY",
            FieldDataType::Real,
            7u8,
            5u8,
        ));

        let mut part_start: usize;
        let mut part_end: usize;
        let mut midpoint_x: f64;
        let mut midpoint_y: f64;
        let mut n: f64;
        let mut r_squared: f64;
        let (mut x, mut y): (f64, f64);
        let mut sigma_x: f64;
        let mut sigma_y: f64;
        let mut sigma_xy: f64;
        let mut sigma_xsqr: f64;
        let mut sigma_ysqr: f64;
        let mut mean: f64;
        let mut sxx: f64;
        let mut syy: f64;
        let mut sxy: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            midpoint_x = (record.x_max - record.x_min) / 2f64;
            midpoint_y = (record.y_max - record.y_min) / 2f64;
            // regression_data = vec[0f64; 5];
            sigma_x = 0f64;
            sigma_y = 0f64;
            sigma_xy = 0f64;
            sigma_xsqr = 0f64;
            sigma_ysqr = 0f64;
            r_squared = 0f64;
            part_start = record.parts[0] as usize;
            part_end = if record.num_parts > 1 {
                record.parts[1] as usize - 1
            } else {
                record.num_points as usize - 1
            };
            n = (part_end - part_start + 1) as f64;
            for i in part_start..=part_end {
                x = record.points[i].x - midpoint_x;
                y = record.points[i].y - midpoint_y;
                sigma_x += x;
                sigma_y += y;
                sigma_xy += x * y;
                sigma_xsqr += x * x;
                sigma_ysqr += y * y;
            }

            mean = sigma_x / n;

            sxx = sigma_xsqr / n - mean * mean;
            syy = sigma_ysqr / n - (sigma_y / n) * (sigma_y / n);
            sxy = sigma_xy / n - (sigma_x * sigma_y) / (n * n);
            if (sxx * syy).sqrt() != 0f64 {
                r_squared = (sxy / (sxx * syy).sqrt()) * (sxy / (sxx * syy).sqrt());
            }

            let record_out = record.clone();
            output.add_record(record_out);

            let mut atts = input.attributes.get_record(record_num);
            atts.push(FieldData::Real(r_squared));
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
