/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/09/2018
Last Modified: 05/12/2018
License: MIT
*/

use self::na::Vector3;
use algorithms::triangulate;
use na;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::Point2D;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// This tool creates a vector triangular irregular network (TIN) for a set of vector points.
pub struct ConstructVectorTIN {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ConstructVectorTIN {
    pub fn new() -> ConstructVectorTIN {
        // public constructor
        let name = "ConstructVectorTIN".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a vector triangular irregular network (TIN) for a set of vector points."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Input field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use Shapefile 'z' values?".to_owned(),
            flags: vec!["--use_z".to_owned()],
            description:
                "Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?"
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=HEIGHT -o=tin.shp
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --use_z -o=tin.shp",
            short_exe, name
        ).replace("*", &sep);

        ConstructVectorTIN {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ConstructVectorTIN {
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
        let mut field_name = String::new();
        let mut use_z = false;
        let mut use_field = false;
        let mut output_file: String = "".to_string();

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
            } else if flag_val == "-field" {
                field_name = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                use_field = true;
            } else if flag_val.contains("use_z") {
                use_z = true;
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

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point
            && input.header.shape_type.base_shape_type() != ShapeType::MultiPoint
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POINT base shape type.",
            ));
        }

        if use_z && input.header.shape_type.dimension() != ShapeTypeDimension::Z {
            return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input vector data must be of 'POINTZ' or 'MULTIPOINTZ' ShapeType to use the --use_z flag.",
                ));
        } else if use_field {
            // What is the index of the field to be analyzed?
            let field_index = match input.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The specified field name does not exist in input shapefile.",
                    ))
                }
            };

            // Is the field numeric?
            if !input.attributes.is_field_numeric(field_index) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified attribute field is non-numeric.",
                ));
            }
        }

        let azimuth = (315f64 - 90f64).to_radians();
        let altitude = 30f64.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        output.projection = input.projection.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 5u8, 0u8));

        if use_field || use_z {
            output.attributes.add_field(&AttributeField::new(
                "CENTROID_Z",
                FieldDataType::Real,
                10u8,
                4u8,
            ));

            output.attributes.add_field(&AttributeField::new(
                "HILLSHADE",
                FieldDataType::Int,
                4u8,
                0u8,
            ));
        }

        let mut points: Vec<Point2D> = vec![];
        let mut z_values: Vec<f64> = vec![];

        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
                if use_z {
                    z_values.push(record.z_array[i]);
                } else if use_field {
                    match input.attributes.get_value(record_num, &field_name) {
                        FieldData::Int(val) => {
                            z_values.push(val as f64);
                        }
                        FieldData::Real(val) => {
                            z_values.push(val);
                        }
                        _ => {
                            // likely a null field
                            z_values.push(0f64);
                        }
                    }
                }
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
            println!("Performing triangulation...");
        }
        // this is where the heavy-lifting is
        let result = triangulate(&points).expect("No triangulation exists.");

        let (mut p1, mut p2, mut p3): (usize, usize, usize);
        let (mut fx, mut fy): (f64, f64);
        let (mut tan_slope, mut aspect): (f64, f64);
        let (mut term1, mut term2, mut term3): (f64, f64, f64);
        let mut hillshade: f64;
        let mut rec_num = 1i32;
        for i in (0..result.triangles.len()).step_by(3) {
            // the points in triangles are counter clockwise ordered and we need clockwise
            p1 = result.triangles[i + 2];
            p2 = result.triangles[i + 1];
            p3 = result.triangles[i];

            let mut tri_points: Vec<Point2D> = Vec::with_capacity(4);
            tri_points.push(points[p1].clone());
            tri_points.push(points[p2].clone());
            tri_points.push(points[p3].clone());
            tri_points.push(points[p1].clone());

            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
            sfg.add_part(&tri_points);
            output.add_record(sfg);

            if use_field || use_z {
                // calculate the hillshade value
                let a = Vector3::new(tri_points[0].x, tri_points[0].y, z_values[p1]);
                let b = Vector3::new(tri_points[1].x, tri_points[1].y, z_values[p2]);
                let c = Vector3::new(tri_points[2].x, tri_points[2].y, z_values[p3]);
                let norm = (b - a).cross(&(c - a)); //).normalize();
                let centroid = (a + b + c) / 3f64;
                // k = -(tri_points[0].x * norm.x + tri_points[0].y * norm.y + norm.z * z_values[p1]);
                // centroid_z = -(norm.x * centroid.x + norm.y * centroid.y + k) / norm.z;

                hillshade = 0f64;
                if norm.z != 0f64 {
                    fx = -norm.x / norm.z;
                    fy = -norm.y / norm.z;
                    if fx != 0f64 {
                        tan_slope = (fx * fx + fy * fy).sqrt();
                        aspect = (180f64 - ((fy / fx).atan()).to_degrees()
                            + 90f64 * (fx / (fx).abs())).to_radians();
                        term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                        term2 = sin_theta / tan_slope;
                        term3 = cos_theta * (azimuth - aspect).sin();
                        hillshade = term1 * (term2 - term3);
                    } else {
                        hillshade = 0.5;
                    }
                    hillshade = hillshade * 1024f64;
                    if hillshade < 0f64 {
                        hillshade = 0f64;
                    }
                }

                output.attributes.add_record(
                    vec![
                        FieldData::Int(rec_num),
                        FieldData::Real(centroid.z),
                        FieldData::Int(hillshade as i32),
                    ],
                    false,
                );
            } else {
                output
                    .attributes
                    .add_record(vec![FieldData::Int(rec_num)], false);
            }

            rec_num += 1i32;

            if verbose {
                progress = (100.0_f64 * i as f64 / (result.triangles.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating polygons: {}%", progress);
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

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
