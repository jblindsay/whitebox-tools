/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/04/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use whitebox_vector::{FieldData, ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to convert a vector points file into a raster grid. The user must
/// specify the name of the input vector and the output raster file. The field name (`--field`)
/// is the field from the attributes table from which the tool will retrieve the information to
/// assign to grid cells in the output raster. The field must contain numerical data. If the user does not
/// supply a field name parameter, each feature in the raster will be assigned the record number
/// of the feature. The assignment operation determines how the situation of multiple points
/// contained within the same grid cell is handled. The background value is zero by default
/// but can be set to `NoData` optionally using the `--nodata` value.
///
/// If the user optionally specifies the grid cell size parameter (`--cell_size`) then the coordinates
/// will be determined by the input vector (i.e. the bounding box) and the specified cell size. This
/// will also determine the number of rows and columns in the output raster. If the user instead
/// specifies the optional base raster file parameter (`--base`), the output raster's coordinates (i.e.
/// north, south, east, west) and row and column count will be the same as the base file.
///
/// In the case that multiple points are contained within a single grid cell, the output can be
/// assigned (`--assign`) the first, last (default), min, max, sum, or number of the contained points.
///
/// # See Also
/// `VectorPolygonsToRaster`, `VectorLinesToRaster`
pub struct VectorPointsToRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VectorPointsToRaster {
    /// public constructor
    pub fn new() -> VectorPointsToRaster {
        let name = "VectorPointsToRaster".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a vector containing points into a raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector Points file.".to_owned(),
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

        parameters.push(ToolParameter{
            name: "Assignment Operation".to_owned(), 
            flags: vec!["--assign".to_owned()], 
            description: "Assignment operation, where multiple points are in the same grid cell; options include 'first', 'last' (default), 'min', 'max', 'sum', 'number'".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["first".to_owned(), "last".to_owned(), "min".to_owned(), "max".to_owned(), "sum".to_owned(), "number".to_owned()]),
            default_value: Some("last".to_owned()),
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=ELEV -o=output.tif --assign=min --nodata --cell_size=10.0
        >>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=FID -o=output.tif --assign=last --base=existing_raster.tif", short_exe, name).replace("*", &sep);

        VectorPointsToRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VectorPointsToRaster {
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
        let mut assign_op = String::from("last");

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
            } else if flag_val == "-assign" {
                assign_op = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i + 1].to_lowercase()
                };
            }
        }

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
        let vector_data = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
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
            // configs.epsg_code = vector_data.projection.clone();
            configs.projection = vector_data.projection.clone();

            Raster::initialize_using_config(&output_file, &configs)
        };

        if background_val != nodata {
            output.reinitialize_values(background_val);
        }

        if field_name == "FID" {
            output.configs.data_type = DataType::I16;
            output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        }

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

        let mut row: isize;
        let mut col: isize;
        let (mut x, mut y, mut z): (f64, f64, f64);
        let num_records = vector_data.num_records;
        if assign_op.contains("last") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    output.set_value(row, col, attribute_data[record_num]);
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
        } else if assign_op.contains("first") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    z = output.get_value(row, col);
                    if z == background_val || z == nodata {
                        output.set_value(row, col, attribute_data[record_num]);
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
        } else if assign_op.contains("min") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    z = output.get_value(row, col);
                    if z == background_val || z == nodata || attribute_data[record_num] < z {
                        output.set_value(row, col, attribute_data[record_num]);
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
        } else if assign_op.contains("max") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    z = output.get_value(row, col);
                    if z == background_val || z == nodata || attribute_data[record_num] > z {
                        output.set_value(row, col, attribute_data[record_num]);
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
        } else if assign_op.contains("sum") || assign_op.contains("total") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    z = output.get_value(row, col);
                    if z == background_val || z == nodata {
                        output.set_value(row, col, attribute_data[record_num]);
                    } else {
                        output.set_value(row, col, z + attribute_data[record_num]);
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
        } else if assign_op.contains("num") {
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.num_points as usize {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    row = output.get_row_from_y(y);
                    col = output.get_column_from_x(x);
                    z = output.get_value(row, col);
                    if z == background_val || z == nodata {
                        output.set_value(row, col, 1f64);
                    } else {
                        output.set_value(row, col, z + 1f64);
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

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
