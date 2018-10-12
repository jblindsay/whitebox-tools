/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/09/2018
Last Modified: 16/09/2018
License: MIT
*/

use lidar::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::{BoundingBox, DistanceMetric, FixedRadiusSearch2D, Point2D};
use time;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// The practice of binning point data to form a type of 2D histogram, density plot,
/// or what is sometimes called a heatmap, is quite useful as an alternative for the
/// cartographic display of of very dense points sets. This is particularly the case
/// when the points experience significant overlap at the displayed scale. The
/// `LidarPointDensity` tool can be used to perform binning based on a regular grid
/// (raster output). This tool, by comparison, bases the binning on a hexagonal grid.
///
/// The tool is similar to the `CreateHexagonalVectorGrid` tool, however instead will
/// create an output hexagonal grid in which each hexagonal cell possesses a `COUNT`
/// attribute which specifies the number of points from an input points file (LAS file)
/// that are contained within the hexagonal cell. The tool will also calculate the
/// minimum and maximum elevations and intensity values and outputs these data to the
/// attribute table.
///
/// In addition to the names of the input points file and the output Shapefile, the user
/// must also specify the desired hexagon width (w), which is the distance between opposing
/// sides of each hexagon. The size (s) each side of the hexagon can then be calculated as,
/// s = w / [2 x cos(PI / 6)]. The area of each hexagon (A) is, A = 3s(w / 2). The user must
/// also specify the orientation of the grid with options of horizontal (pointy side up) and
/// vertical (flat side up).
///
/// # See Also
/// `VectorHexBinning`, `LidarPointDensity`, `CreateHexagonalVectorGrid`
pub struct LidarHexBinning {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarHexBinning {
    pub fn new() -> LidarHexBinning {
        // public constructor
        let name = "LidarHexBinning".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Hex-bins a set of LiDAR points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input base file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
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
            name: "Hexagon Width".to_owned(),
            flags: vec!["--width".to_owned()],
            description: "The grid cell width.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid Orientation".to_owned(),
            flags: vec!["--orientation".to_owned()],
            description: "Grid Orientation, 'horizontal' or 'vertical'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "horizontal".to_owned(),
                "vertical".to_owned(),
            ]),
            default_value: Some("horizontal".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.shp --width=10.0 --orientation=vertical",
            short_exe, name
        ).replace("*", &sep);

        LidarHexBinning {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarHexBinning {
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
        let mut width = 0f64;
        let mut orientation = String::from("h");

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
            } else if flag_val == "-width" {
                width = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val.contains("ori") {
                orientation = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                if orientation.to_lowercase().contains("v") {
                    // vertical orientation
                    orientation = String::from("v");
                } else {
                    // horizontal orientation
                    orientation = String::from("h");
                }
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

        if width <= 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: The grid cell width must be greater than zero.",
            ));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // Get the spatial extent
        let mut input = LasFile::new(&input_file, "r")?;
        let num_points = input.header.number_of_points;
        let extent = BoundingBox::new(
            input.header.min_x,
            input.header.max_x,
            input.header.min_y,
            input.header.max_y,
        );
        let proj_info = input.get_wkt();

        let sixty_degrees = f64::consts::PI / 6f64;
        let half_width = 0.5 * width;
        let size = half_width / sixty_degrees.cos();
        let height = size * 2f64;
        let three_quarter_height = 0.75 * height;
        let mut angle: f64;
        let (mut x, mut y): (f64, f64);
        let mut rec_num = 1i32;
        let (mut center_x, mut center_y): (f64, f64);

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        if !proj_info.is_empty() && proj_info.to_lowercase() != "not specified" {
            output.projection = proj_info;
        }

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 5u8, 0u8));
        output
            .attributes
            .add_field(&AttributeField::new("ROW", FieldDataType::Int, 5u8, 0u8));
        output
            .attributes
            .add_field(&AttributeField::new("COLUMN", FieldDataType::Int, 5u8, 0u8));
        output
            .attributes
            .add_field(&AttributeField::new("COUNT", FieldDataType::Int, 7u8, 0u8));
        output
            .attributes
            .add_field(&AttributeField::new("MIN_Z", FieldDataType::Real, 9u8, 4u8));
        output
            .attributes
            .add_field(&AttributeField::new("MAX_Z", FieldDataType::Real, 9u8, 4u8));
        output
            .attributes
            .add_field(&AttributeField::new("MIN_I", FieldDataType::Int, 6u8, 0u8));
        output
            .attributes
            .add_field(&AttributeField::new("MAX_I", FieldDataType::Int, 6u8, 0u8));

        let mut frs: FixedRadiusSearch2D<usize> =
            FixedRadiusSearch2D::new(width * 2f64, DistanceMetric::SquaredEuclidean);

        if orientation == "h" {
            // horizontal orientation
            let center_x_0 = extent.min_x + half_width;
            let center_y_0 = extent.max_y - 0.25 * height;
            let rows = (extent.get_height() / three_quarter_height).ceil() as usize;
            let mut columns = (extent.get_width() / width).ceil() as usize;
            if rows * columns > 100000 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "ERROR: This operation would produce a vector file with too many polygons. Perhaps choose a higher hexagon width",
                ));
            }

            // first fill the FRS with the hex centre points
            let mut hex_index = 0usize;
            for row in 0..rows {
                center_y = center_y_0 - row as f64 * three_quarter_height;
                columns = ((extent.get_width() + half_width * (row as f64 % 2f64)) / width).ceil()
                    as usize;
                for col in 0..columns {
                    center_x = (center_x_0 - half_width * (row as f64 % 2f64)) + col as f64 * width;
                    frs.insert(center_x, center_y, hex_index);
                    hex_index += 1usize;
                }
            }

            // now find which bin each point belongs to and update the stats
            let num_hexes = hex_index;
            let mut count = vec![0i32; num_hexes];
            let mut min_z = vec![f64::INFINITY; num_hexes];
            let mut max_z = vec![f64::NEG_INFINITY; num_hexes];
            let mut min_i = vec![32767i32; num_hexes];
            let mut max_i = vec![0i32; num_hexes];

            let mut p: PointData;
            for i in 0..num_points as usize {
                p = input.get_point_info(i);
                let ret = frs.knn_search(p.x, p.y, 1);
                if ret.len() > 0 {
                    hex_index = ret[0].0;
                    count[hex_index] += 1;
                    if p.z < min_z[hex_index] {
                        min_z[hex_index] = p.z;
                    }
                    if p.z > max_z[hex_index] {
                        max_z[hex_index] = p.z;
                    }
                    if (p.intensity as i32) < min_i[hex_index] {
                        min_i[hex_index] = p.intensity as i32;
                    }
                    if (p.intensity as i32) > max_i[hex_index] {
                        max_i[hex_index] = p.intensity as i32;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points as f64) as usize;
                    if progress != old_progress {
                        println!("Binning points: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            hex_index = 0usize;
            for row in 0..rows {
                center_y = center_y_0 - row as f64 * three_quarter_height;
                columns = ((extent.get_width() + half_width * (row as f64 % 2f64)) / width).ceil()
                    as usize;
                for col in 0..columns {
                    let mut points: Vec<Point2D> = Vec::with_capacity(7);

                    center_x = (center_x_0 - half_width * (row as f64 % 2f64)) + col as f64 * width;
                    for i in (0..=6).rev() {
                        angle = 2f64 * sixty_degrees * (i as f64 + 0.5);
                        x = center_x + size * angle.cos();
                        y = center_y + size * angle.sin();
                        points.push(Point2D::new(x, y));
                    }

                    let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                    sfg.add_part(&points);
                    output.add_record(sfg);

                    output.attributes.add_record(
                        vec![
                            FieldData::Int(rec_num),
                            FieldData::Int(row as i32),
                            FieldData::Int(col as i32),
                            FieldData::Int(count[hex_index]),
                            FieldData::Real(min_z[hex_index]),
                            FieldData::Real(max_z[hex_index]),
                            FieldData::Int(min_i[hex_index]),
                            FieldData::Int(max_i[hex_index]),
                        ],
                        false,
                    );

                    hex_index += 1usize;
                    rec_num += 1i32;
                }

                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            let center_x_0 = extent.min_x + 0.5 * size;
            let center_y_0 = extent.max_y - half_width;
            let mut rows = (extent.get_height() / width).ceil() as usize;
            let columns = (extent.get_width() / three_quarter_height).ceil() as usize;
            if rows * columns > 100000 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "ERROR: This operation would produce a vector file with too many polygons. Perhaps choose a higher hexagon width",
                ));
            }

            // first fill the FRS with the hex centre points
            let mut hex_index = 0usize;
            for col in 0..columns {
                rows = ((extent.get_height() + ((col as f64 % 2f64) * half_width)) / width).ceil()
                    as usize;
                for row in 0..rows {
                    center_x = center_x_0 + col as f64 * three_quarter_height;
                    center_y = center_y_0 - row as f64 * width + ((col as f64 % 2f64) * half_width);
                    frs.insert(center_x, center_y, hex_index);
                    hex_index += 1usize;
                }
            }

            // now find which bin each point belongs to and update the stats
            let num_hexes = hex_index;
            let mut count = vec![0i32; num_hexes];
            let mut min_z = vec![f64::INFINITY; num_hexes];
            let mut max_z = vec![f64::NEG_INFINITY; num_hexes];
            let mut min_i = vec![32767i32; num_hexes];
            let mut max_i = vec![0i32; num_hexes];

            let mut p: PointData;
            for i in 0..num_points as usize {
                p = input.get_point_info(i);
                let ret = frs.knn_search(p.x, p.y, 1);
                if ret.len() > 0 {
                    hex_index = ret[0].0;
                    count[hex_index] += 1;
                    if p.z < min_z[hex_index] {
                        min_z[hex_index] = p.z;
                    }
                    if p.z > max_z[hex_index] {
                        max_z[hex_index] = p.z;
                    }
                    if (p.intensity as i32) < min_i[hex_index] {
                        min_i[hex_index] = p.intensity as i32;
                    }
                    if (p.intensity as i32) > max_i[hex_index] {
                        max_i[hex_index] = p.intensity as i32;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points as f64) as usize;
                    if progress != old_progress {
                        println!("Binning points: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            hex_index = 0usize;
            for col in 0..columns {
                rows = ((extent.get_height() + ((col as f64 % 2f64) * half_width)) / width).ceil()
                    as usize;
                for row in 0..rows {
                    center_x = center_x_0 + col as f64 * three_quarter_height;
                    center_y = center_y_0 - row as f64 * width + ((col as f64 % 2f64) * half_width);
                    let mut points: Vec<Point2D> = Vec::with_capacity(7);
                    for i in (0..=6).rev() {
                        angle = 2f64 * sixty_degrees * (i as f64 + 0.5) - sixty_degrees;
                        x = center_x + size * angle.cos();
                        y = center_y + size * angle.sin();
                        points.push(Point2D::new(x, y));
                    }

                    let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                    sfg.add_part(&points);
                    output.add_record(sfg);

                    output.attributes.add_record(
                        vec![
                            FieldData::Int(rec_num),
                            FieldData::Int(row as i32),
                            FieldData::Int(col as i32),
                            FieldData::Int(count[hex_index]),
                            FieldData::Real(min_z[hex_index]),
                            FieldData::Real(max_z[hex_index]),
                            FieldData::Int(min_i[hex_index]),
                            FieldData::Int(max_i[hex_index]),
                        ],
                        false,
                    );

                    hex_index += 1usize;
                    rec_num += 1i32;
                }

                if verbose {
                    progress = (100.0_f64 * col as f64 / (columns - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
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
