/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 18/04/2018
Last Modified: 22/10/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::BoundingBox;
use crate::tools::*;
use crate::vector::{FieldData, ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to convert a vector lines or polygon file into a raster grid of lines. If a vector of one
/// of the polygon ShapeTypes is selected, the resulting raster will outline the polygons without filling these
/// features. Use the `VectorPolygonToRaster` tool if you need to fill the polygon features.
///
/// The user must specify the name of the input vector (`--input`) and the output raster file (`--output`). The Field
/// Name (`--field`) is
/// the field from the attributes table, from which the tool will retrieve the information to assign to
/// grid cells in the output raster. Note that if this field contains numerical data with no decimals, the output raster
/// data type will be INTEGER; if it contains decimals it will be of a FLOAT data type. The field must contain numerical
/// data. If the user does not supply a Field Name parameter, each feature in the raster will be assigned the record
/// number of the feature. The assignment operation determines how the situation of multiple points contained within the
/// same grid cell is handled. The background value is the value that is assigned to grid cells in the output raster that
/// do not correspond to the location of any points in the input vector. This value can be any numerical value (e.g. 0)
/// or the string 'NoData', which is the default.
///
/// If the user optionally specifies the `--cell_size` parameter then the coordinates will be determined by the input
/// vector (i.e. the bounding box) and the specified Cell Size. This will also determine the number of rows and columns
/// in the output raster. If the user instead specifies the optional base raster file parameter (`--base`), the output raster's
/// coordinates (i.e. north, south, east, west) and row and column count will be the same as the base file. If the user
/// does not specify either of these two optional parameters, the tool will determine the cell size automatically as the
/// maximum of the north-south extent (determined from the shapefile's bounding box) or the east-west extent divided by 500.
///
/// # See Also
/// `VectorPointsToRaster`, `VectorPolygonsToRaster`
pub struct VectorLinesToRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VectorLinesToRaster {
    /// public constructor
    pub fn new() -> VectorLinesToRaster {
        let name = "VectorLinesToRaster".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a vector containing polylines into a raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector lines file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
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
            default_value: Some("FID".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Background value is NoData?".to_owned(),
            flags: vec!["--nodata".to_owned()],
            description:
                "Background value to set to NoData. Without this flag, it will be set to 0.0."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Base Raster File (optional)".to_owned(), 
            flags: vec!["--base".to_owned()], 
            description: "Optionally specified input base raster file. Not used when a cell size is specified.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lines.shp --field=ELEV -o=output.tif --nodata --cell_size=10.0
        >>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lines.shp --field=FID -o=output.tif --base=existing_raster.tif", short_exe, name).replace("*", &sep);

        VectorLinesToRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VectorLinesToRaster {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut field_name = String::from("FID");
        let mut output_file = String::new();
        let mut cell_size = 0f64;
        let mut base_file = String::new();
        let nodata = -32768.0f64;
        let mut background_val = 0f64;

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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-cell_size" {
                cell_size = if keyval {
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
            } else if flag_val == "-base" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-nodata" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    background_val = nodata;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let vector_data = Shapefile::read(&input_file).expect("Error reading input Shapefile.");

        let start = Instant::now();

        // make sure the input vector file is of polyline or polygon type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && vector_data.header.shape_type.base_shape_type() != ShapeType::Polygon
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must either be of polyline or polygon base shape type.",
            ));
        }

        // What is the index of the field to be analyzed?
        let field_index = match vector_data.attributes.get_field_num(&field_name) {
            Some(i) => i,
            None => {
                // Field not found use FID
                if verbose {
                    println!("Warning: Attribute not found in table. FID will be used instead.");
                }
                field_name = "FID".to_string();
                0
            }
        };

        // Is the field numeric?
        if !vector_data.attributes.is_field_numeric(field_index) {
            // Warn user of non-numeric
            if verbose {
                println!("Warning: Non-numeric attributes cannot be rasterized. FID will be used instead.");
            }
            field_name = "FID".to_string(); // Can't use non-numeric field; use FID instead.
        }

        // Create the output raster. The process of doing this will
        // depend on whether a cell size or a base raster were specified.
        // If both are specified, the base raster takes priority.

        let mut output = if !base_file.trim().is_empty() || cell_size == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let base = Raster::new(&base_file, "r")?;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            // base the output raster on the cell_size and the
            // extent of the input vector.
            let west: f64 = vector_data.header.x_min;
            let north: f64 = vector_data.header.y_max;
            let rows: isize = (((north - vector_data.header.y_min) / cell_size).ceil()) as isize;
            let columns: isize = (((vector_data.header.x_max - west) / cell_size).ceil()) as isize;
            let south: f64 = north - rows as f64 * cell_size;
            let east = west + columns as f64 * cell_size;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows as usize;
            configs.columns = columns as usize;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = cell_size;
            configs.resolution_y = cell_size;
            configs.nodata = nodata;
            configs.data_type = DataType::F32;
            configs.photometric_interp = PhotometricInterpretation::Continuous;
            configs.projection = vector_data.projection.clone();

            Raster::initialize_using_config(&output_file, &configs)
        };

        if background_val != nodata {
            output.reinitialize_values(background_val);
        }

        if field_name == "FID" {
            output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        }

        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;

        let mut attribute_data = vec![background_val; vector_data.num_records];
        // get the attribute data
        for record_num in 0..vector_data.num_records {
            if field_name != "FID" {
                match vector_data.attributes.get_value(record_num, &field_name) {
                    FieldData::Int(val) => {
                        attribute_data[record_num] = val as f64;
                    }
                    // FieldData::Int64(val) => {
                    //     attribute_data[record_num] = val as f64;
                    // },
                    FieldData::Real(val) => {
                        attribute_data[record_num] = val;
                    }
                    _ => {
                        // do nothing; likely due to null value for record.
                    }
                }
            } else {
                attribute_data[record_num] = (record_num + 1) as f64;
            }

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Reading attributes: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let raster_bb = BoundingBox::new(
            output.configs.west,
            output.configs.east,
            output.configs.south,
            output.configs.north,
        );
        let mut bb = BoundingBox {
            ..Default::default()
        };
        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let mut row_y_coord: f64;
        let mut col_x_coord: f64;
        let (mut x1, mut x2, mut y1, mut y2): (f64, f64, f64, f64);
        let (mut x_prime, mut y_prime): (f64, f64);
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        let mut output_something = false;
        let num_records = vector_data.num_records;
        for record_num in 0..vector_data.num_records {
            let record = vector_data.get_record(record_num);
            let rec_bb = BoundingBox::new(record.x_min, record.x_max, record.y_min, record.y_max);
            if rec_bb.overlaps(raster_bb) {
                for part in 0..record.num_parts as usize {
                    start_point_in_part = record.parts[part] as usize;
                    if part < record.num_parts as usize - 1 {
                        end_point_in_part = record.parts[part + 1] as usize - 1;
                    } else {
                        end_point_in_part = record.num_points as usize - 1;
                    }

                    bb.initialize_to_inf();
                    for i in start_point_in_part..end_point_in_part + 1 {
                        if record.points[i].x < bb.min_x {
                            bb.min_x = record.points[i].x;
                        }
                        if record.points[i].x > bb.max_x {
                            bb.max_x = record.points[i].x;
                        }
                        if record.points[i].y < bb.min_y {
                            bb.min_y = record.points[i].y;
                        }
                        if record.points[i].y > bb.max_y {
                            bb.max_y = record.points[i].y;
                        }
                    }
                    top_row = output.get_row_from_y(bb.max_y);
                    bottom_row = output.get_row_from_y(bb.min_y);
                    left_col = output.get_column_from_x(bb.min_x);
                    right_col = output.get_column_from_x(bb.max_x);

                    if top_row < 0 {
                        top_row = 0;
                    }
                    if bottom_row < 0 {
                        bottom_row = 0;
                    }
                    if top_row >= rows {
                        top_row = rows - 1;
                    }
                    if bottom_row >= rows {
                        bottom_row = rows - 1;
                    }

                    if left_col < 0 {
                        left_col = 0;
                    }
                    if right_col < 0 {
                        right_col = 0;
                    }
                    if left_col >= columns {
                        left_col = columns - 1;
                    }
                    if right_col >= columns {
                        right_col = columns - 1;
                    }

                    // find each intersection with a row.
                    for row in top_row..bottom_row + 1 {
                        row_y_coord = output.get_y_from_row(row);
                        // find the x-coordinates of each of the line segments
                        // that intersect this row's y coordinate
                        for i in start_point_in_part..end_point_in_part {
                            if is_between(row_y_coord, record.points[i].y, record.points[i + 1].y) {
                                y1 = record.points[i].y;
                                y2 = record.points[i + 1].y;
                                if y2 != y1 {
                                    x1 = record.points[i].x;
                                    x2 = record.points[i + 1].x;

                                    // calculate the intersection point
                                    x_prime = x1 + (row_y_coord - y1) / (y2 - y1) * (x2 - x1);
                                    let col = output.get_column_from_x(x_prime);

                                    output.set_value(row, col, attribute_data[record_num]);
                                    output_something = true;
                                }
                            }
                        }
                    }

                    // find each intersection with a column.
                    for col in left_col..right_col + 1 {
                        col_x_coord = output.get_x_from_column(col);
                        for i in start_point_in_part..end_point_in_part {
                            if is_between(col_x_coord, record.points[i].x, record.points[i + 1].x) {
                                x1 = record.points[i].x;
                                x2 = record.points[i + 1].x;
                                if x1 != x2 {
                                    y1 = record.points[i].y;
                                    y2 = record.points[i + 1].y;

                                    // calculate the intersection point
                                    y_prime = y1 + (col_x_coord - x1) / (x2 - x1) * (y2 - y1);

                                    let row = output.get_row_from_y(y_prime);

                                    output.set_value(row, col, attribute_data[record_num]);
                                    output_something = true;
                                }
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (record_num + 1) as f64 / num_records as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Rasterizing {} of {}: {}%",
                        record_num + 1,
                        num_records,
                        progress
                    );
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
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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

        if !output_something && verbose {
            println!("Warning: No polylines were output to the raster.");
        }

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

fn is_between(val: f64, threshold1: f64, threshold2: f64) -> bool {
    if val == threshold1 || val == threshold2 {
        return true;
    }
    if threshold2 > threshold1 {
        return val > threshold1 && val < threshold2;
    }
    val > threshold2 && val < threshold1
}
