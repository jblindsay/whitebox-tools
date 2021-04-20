/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/09/2018
Last Modified: 18/10/2019
License: MIT
*/

use self::na::Vector3;
use whitebox_common::algorithms::{point_in_poly, triangulate};
use crate::na;
use whitebox_raster::*;
use whitebox_common::structures::Point2D;
use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// Creates a raster grid based on a triangular irregular network (TIN) fitted to vector points
/// and linear interpolation within each triangular-shaped plane. The TIN creation algorithm is based on
/// [Delaunay triangulation](https://en.wikipedia.org/wiki/Delaunay_triangulation).
///
/// The user must specify the attribute field containing point values (`--field`). Alternatively, if the input Shapefile
/// contains z-values, the interpolation may be based on these values (`--use_z`). Either an output grid resolution
/// (`--cell_size`) must be specified or alternatively an existing base file (`--base`) can be used to determine the
/// output raster's (`--output`) resolution and spatial extent. Natural neighbour interpolation generally produces a
/// satisfactorily smooth surface within the region of data points but can produce spurious breaks in the surface
/// outside of this region. Thus, it is recommended that the output surface be clipped to the convex hull of the input
/// points (`--clip`).
///
/// # See Also
/// `LidarTINGridding`, `ConstructVectorTIN`, `NaturalNeighbourInterpolation`
pub struct TINGridding {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TINGridding {
    pub fn new() -> TINGridding {
        // public constructor
        let name = "TINGridding".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a raster grid based on a triangular irregular network (TIN) fitted to vector points."
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
            name: "Output Raster File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Base Raster File (optional)".to_owned(), 
            flags: vec!["--base".to_owned()], 
            description: "Optionally specified input base raster file. Not used when a cell size is specified.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Maximum Triangle Edge Length (optional)".to_owned(),
            flags: vec!["--max_triangle_edge_length".to_owned()],
            description: "Optional maximum triangle edge length; triangles larger than this size will not be gridded.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=HEIGHT -o=tin.shp --resolution=10.0
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --use_z -o=tin.shp --resolution=5.0",
            short_exe, name
        ).replace("*", &sep);

        TINGridding {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TINGridding {
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
        let mut grid_res: f64 = 0.0;
        let mut base_file = String::new();
        let mut max_triangle_edge_length = f64::INFINITY;

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
            } else if flag_val == "-field" {
                field_name = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                use_field = true;
            } else if flag_val.contains("use_z") {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_z = true;
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
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
            } else if flag_val == "-max_triangle_edge_length" {
                max_triangle_edge_length = if keyval {
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

                max_triangle_edge_length *= max_triangle_edge_length; // actually squared distance
            } else if flag_val == "-base" {
                base_file = if keyval {
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

        if !use_z && !use_field {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "If vector data 'Z' data are unavailable (--use_z), an attribute field must be specified (--field=).",
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

        // let west: f64 = input.header.x_min;
        // let north: f64 = input.header.y_max;
        // let rows: isize = (((north - input.header.y_min) / grid_res).ceil()) as isize;
        // let columns: isize = (((input.header.x_max - west) / grid_res).ceil()) as isize;
        // let south: f64 = north - rows as f64 * grid_res;
        // let east = west + columns as f64 * grid_res;
        // let nodata = -32768.0f64;

        // let mut configs = RasterConfigs {
        //     ..Default::default()
        // };
        // configs.rows = rows as usize;
        // configs.columns = columns as usize;
        // configs.north = north;
        // configs.south = south;
        // configs.east = east;
        // configs.west = west;
        // configs.resolution_x = grid_res;
        // configs.resolution_y = grid_res;
        // configs.nodata = nodata;
        // configs.data_type = DataType::F32;
        // configs.photometric_interp = PhotometricInterpretation::Continuous;

        // let mut output = Raster::initialize_using_config(&output_file, &configs);

        let nodata = -32768.0f64;
        let mut output = if !base_file.trim().is_empty() || grid_res == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let mut base = Raster::new(&base_file, "r")?;
            base.configs.nodata = nodata;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            if grid_res == 0f64 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified grid resolution is incorrect. Either a non-zero grid resolution \nor an input existing base file name must be used.",
                ));
            }
            // base the output raster on the grid_res and the
            // extent of the input vector.
            let west: f64 = input.header.x_min;
            let north: f64 = input.header.y_max;
            let rows: isize = (((north - input.header.y_min) / grid_res).ceil()) as isize;
            let columns: isize = (((input.header.x_max - west) / grid_res).ceil()) as isize;
            let south: f64 = north - rows as f64 * grid_res;
            let east = west + columns as f64 * grid_res;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows as usize;
            configs.columns = columns as usize;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = grid_res;
            configs.resolution_y = grid_res;
            configs.nodata = nodata;
            configs.data_type = DataType::F32;
            configs.photometric_interp = PhotometricInterpretation::Continuous;

            Raster::initialize_using_config(&output_file, &configs)
        };

        let west = output.configs.west;
        let north = output.configs.north;
        output.configs.nodata = nodata; // in case a base image is used with a different nodata value.
        output.configs.palette = "spectrum.pal".to_string();
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        let res_x = output.configs.resolution_x;
        let res_y = output.configs.resolution_y;

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
        let delaunay = triangulate(&points).expect("No triangulation exists.");
        let num_triangles = delaunay.triangles.len() / 3;

        let (mut p1, mut p2, mut p3): (usize, usize, usize);
        let (mut top, mut bottom, mut left, mut right): (f64, f64, f64, f64);

        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let mut tri_points: Vec<Point2D> = vec![Point2D::new(0f64, 0f64); 4];
        let mut k: f64;
        let mut norm: Vector3<f64>;
        let (mut a, mut b, mut c): (Vector3<f64>, Vector3<f64>, Vector3<f64>);
        let (mut x, mut y): (f64, f64);
        let mut z: f64;
        let mut i: usize;
        for triangle in 0..num_triangles {
            i = triangle * 3;
            p1 = delaunay.triangles[i];
            p2 = delaunay.triangles[i + 1];
            p3 = delaunay.triangles[i + 2];
            if max_distance_squared(
                points[p1],
                points[p2],
                points[p3],
                z_values[p1],
                z_values[p2],
                z_values[p3],
            ) < max_triangle_edge_length
            {
                tri_points[0] = points[p1].clone();
                tri_points[1] = points[p2].clone();
                tri_points[2] = points[p3].clone();
                tri_points[3] = points[p1].clone();
                // if is_clockwise_order(&tri_points) {
                //     tri_points.reverse();
                // }

                // get the equation of the plane
                a = Vector3::new(tri_points[0].x, tri_points[0].y, z_values[p1]);
                b = Vector3::new(tri_points[1].x, tri_points[1].y, z_values[p2]);
                c = Vector3::new(tri_points[2].x, tri_points[2].y, z_values[p3]);
                norm = (b - a).cross(&(c - a));

                if norm.z != 0f64 {
                    k = -(tri_points[0].x * norm.x
                        + tri_points[0].y * norm.y
                        + norm.z * z_values[p1]);

                    // find grid intersections with this triangle
                    bottom = points[p1].y.min(points[p2].y.min(points[p3].y));
                    top = points[p1].y.max(points[p2].y.max(points[p3].y));
                    left = points[p1].x.min(points[p2].x.min(points[p3].x));
                    right = points[p1].x.max(points[p2].x.max(points[p3].x));

                    bottom_row = ((north - bottom) / res_y).ceil() as isize;
                    top_row = ((north - top) / res_y).floor() as isize;
                    left_col = ((left - west) / res_x).floor() as isize;
                    right_col = ((right - west) / res_x).ceil() as isize;

                    for row in top_row..=bottom_row {
                        for col in left_col..=right_col {
                            x = west + (col as f64 + 0.5) * res_x;
                            y = north - (row as f64 + 0.5) * res_y;
                            if point_in_poly(&Point2D::new(x, y), &tri_points) {
                                // calculate the z values
                                z = -(norm.x * x + norm.y * y + k) / norm.z;
                                output.set_value(row, col, z);
                            }
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * triangle as f64 / (num_triangles - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

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

/// Calculate squared Euclidean distance between the point and another.
pub fn max_distance_squared(
    p1: Point2D,
    p2: Point2D,
    p3: Point2D,
    z1: f64,
    z2: f64,
    z3: f64,
) -> f64 {
    let mut dx = p1.x - p2.x;
    let mut dy = p1.y - p2.y;
    let mut dz = z1 - z2;
    let mut max_dist = dx * dx + dy * dy + dz * dz;

    dx = p1.x - p3.x;
    dy = p1.y - p3.y;
    dz = z1 - z3;
    let mut dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    dx = p2.x - p3.x;
    dy = p2.y - p3.y;
    dz = z2 - z3;
    dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    max_dist
}
