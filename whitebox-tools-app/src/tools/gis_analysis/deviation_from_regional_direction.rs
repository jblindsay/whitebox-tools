/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/05/2023
Last Modified: 24/05/2023
License: MIT
*/

use crate::tools::*;
use whitebox_common::algorithms::{minimum_bounding_box, MinimizationCriterion};
use whitebox_common::structures::Point2D;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool calculates the orientation of polygon features based on the slope of a reduced major
/// axis (RMA) regression line. The regression analysis use the vertices of the exterior hull nodes
/// of a vector polygon. The only required input is the name of the vector polygon file. The
/// orientation values, measured in degrees from north, will be placed in the accompanying attribute
/// table as a new field (ORIENT). The value of the orientation measure for any polygon will
/// depend on how elongated the feature is.
///
/// Note that the output values are polygon orientations and not true directions. While directions
/// may take values ranging from 0-360, orientation is expressed as an angle between 0 and 180 degrees
/// clockwise from north. Lastly, the orientation measure may become unstable when polygons are
/// oriented nearly vertical or horizontal.
///
/// # See Also
/// `LinearityIndex`, `ElongationRatio`
pub struct DeviationFromRegionalDirection {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DeviationFromRegionalDirection {
    pub fn new() -> DeviationFromRegionalDirection {
        // public constructor
        let name = "DeviationFromRegionalDirection".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description = "Calculates the orientation of vector polygons.".to_string();

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

        parameters.push(ToolParameter {
            name: "Elongation Threshold (0.05-0.95)".to_owned(),
            flags: vec!["--elong_threshold".to_owned()],
            description: "Elongation threshold used in determining which polygons are used to estimate the regional direction (0.05-0.95).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.75".to_string()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
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

        DeviationFromRegionalDirection {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DeviationFromRegionalDirection {
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
        let mut elongation_threshold = 0.75;

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
            } else if flag_val == "-elong_threshold" {
                elongation_threshold = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if elongation_threshold < 0.05 { elongation_threshold = 0.05; }
        if elongation_threshold > 0.95 { elongation_threshold = 0.95; }

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

        // First, calculate the mean direction of the polygon set. This is weighted by 
        // the polygon length and it's elongation, such that w = L * (1 - S/L) where 
        // L is the long-axis length and S is the short-axis length. Thus, more weight
        // is assigned to larger, more elongated polygons.

        // create output file
        let mut output = Shapefile::initialize_using_file(&input_file, &input, input.header.shape_type, true)?;

        // add the attributes
        output.attributes.add_field(&AttributeField::new(
            "DEV_DIR",
            FieldDataType::Real,
            7u8,
            4u8,
        ));

        let mut part_start: usize;
        let mut part_end: usize;
        let mut midpoint_x: f64;
        let mut midpoint_y: f64;
        let mut n: f64;
        // let mut r_squared: f64;
        let mut slope_deg_rma: f64;
        let mut slope_rma: f64;
        let (mut x, mut y): (f64, f64);
        let mut sigma_x: f64;
        let mut sigma_y: f64;
        let mut sigma_xy: f64;
        let mut sigma_xsqr: f64;
        let mut sigma_ysqr: f64;
        let mut mean: f64;
        let mut sxx: f64;
        let mut syy: f64;
        // let mut sxy: f64;
        let mut dist1: f64;
        let mut dist2: f64;
        let mut short_axis: f64;
        let mut long_axis: f64;
        let mut elongation: f64;
        let mut sum_sin = 0.0;
        let mut sum_cos = 0.0;
        let mut weight: f64;
        let mut num_polys_used = 0;
        // let mut r_squared: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            midpoint_x = (record.x_max - record.x_min) / 2f64;
            midpoint_y = (record.y_max - record.y_min) / 2f64;
            sigma_x = 0f64;
            sigma_y = 0f64;
            sigma_xy = 0f64;
            sigma_xsqr = 0f64;
            sigma_ysqr = 0f64;
            // r_squared = 0f64;
            part_start = record.parts[0] as usize;
            part_end = if record.num_parts > 1 {
                record.parts[1] as usize - 1
            } else {
                record.num_points as usize - 1
            };
            n = (part_end - part_start + 1) as f64;
            let mut points: Vec<Point2D> = Vec::with_capacity(record.num_points as usize);
            for i in part_start..=part_end {
                x = record.points[i].x - midpoint_x;
                y = record.points[i].y - midpoint_y;
                sigma_x += x;
                sigma_y += y;
                sigma_xy += x * y;
                sigma_xsqr += x * x;
                sigma_ysqr += y * y;

                points.push(Point2D::new(record.points[i].x, record.points[i].y));
            }

            mean = sigma_x / n;

            sxx = sigma_xsqr / n - mean * mean;
            syy = sigma_ysqr / n - (sigma_y / n) * (sigma_y / n);
            // sxy = sigma_xy / n - (sigma_x * sigma_y) / (n * n);
            // if (sxx * syy).sqrt() != 0f64 {
            //     r_squared = (sxy / (sxx * syy).sqrt()) * (sxy / (sxx * syy).sqrt());
            // }

            // Calculate the slope of the Reduced Major Axis (RMA)
            slope_rma = (syy / sxx).sqrt();
            if (sigma_xy - mean * sigma_y) / (sigma_xsqr - mean * sigma_x) < 0f64 {
                slope_rma = -slope_rma;
            }

            let mbb_points = minimum_bounding_box(&mut points, MinimizationCriterion::Area);

            // now calculate the distance between the first and second points and the second and third points
            dist1 = mbb_points[0].distance(&mbb_points[1]);
            dist2 = mbb_points[1].distance(&mbb_points[2]);

            // get the short and long axes
            short_axis = dist1.min(dist2);
            long_axis = dist1.max(dist2);

            // calculate the elongation and the weight
            elongation = 1f64 - short_axis / long_axis;
            weight = if elongation >= elongation_threshold {
                num_polys_used += 1;
                long_axis * elongation
            } else {
                 0.0 // If the poly isn't elongated enough, don't use it in caluclating the mean regional poly direction
            }; 
            
            // towards calculating the mean regional poly direction
            sum_sin += (slope_rma.atan()*2.0).sin() * weight; // multiply by 2 because these are axial data, not true directions
            sum_cos += (slope_rma.atan()*2.0).cos() * weight;
            // sum_sin += slope_rma.atan().sin() * weight;
            // sum_cos += slope_rma.atan().cos() * weight;

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if num_polys_used == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "No polygons in the dataset have an elongation ratio greater than the threshold. Lower the elongation threshold value and try again.",
            ));
        }

        // Calculate the weighted regional weighted mean direciton of the polygons
        let mut regional_angle = -(sum_sin.atan2(sum_cos) / 2.0).to_degrees() + 90.0; // divided by 2 because these are axial data not true directions
        // let mut regional_angle = -(sum_sin.atan2(sum_cos)).to_degrees() + 90.0; 
        if regional_angle < 0.0 { regional_angle = 180.0 + regional_angle; }

        if verbose {
            println!("Regional weighted mean polygon direction: {:.3} degrees", regional_angle);
        }


        let mut deviation_angle: f64;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            midpoint_x = (record.x_max - record.x_min) / 2f64;
            midpoint_y = (record.y_max - record.y_min) / 2f64;
            sigma_x = 0f64;
            sigma_y = 0f64;
            sigma_xy = 0f64;
            sigma_xsqr = 0f64;
            sigma_ysqr = 0f64;
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

            // Calculate the slope of the Reduced Major Axis (RMA)
            slope_rma = (syy / sxx).sqrt();
            if (sigma_xy - mean * sigma_y) / (sigma_xsqr - mean * sigma_x) < 0f64 {
                slope_rma = -slope_rma;
            }
            slope_deg_rma = slope_rma.atan().to_degrees();
            slope_deg_rma = if slope_deg_rma < 0f64 {
                90f64 + -1f64 * slope_deg_rma
            } else {
                90f64 - slope_deg_rma
            };

            deviation_angle = slope_deg_rma - regional_angle;
            if deviation_angle < 0.0 {
                deviation_angle += 180.0;
            }
            if deviation_angle > 90.0 {
                deviation_angle = 180.0 - deviation_angle;
            }

            let record_out = record.clone();
            output.add_record(record_out);

            let mut atts = input.attributes.get_record(record_num);
            atts.push(FieldData::Real(deviation_angle));
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
