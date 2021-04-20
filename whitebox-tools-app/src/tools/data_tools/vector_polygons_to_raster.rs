/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/04/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_common::algorithms::point_in_poly;
use whitebox_raster::*;
use whitebox_common::structures::{Array2D, BoundingBox, Point2D};
use crate::tools::*;
use whitebox_vector::{FieldData, ShapeType, Shapefile};
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

pub struct VectorPolygonsToRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VectorPolygonsToRaster {
    /// public constructor
    pub fn new() -> VectorPolygonsToRaster {
        let name = "VectorPolygonsToRaster".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a vector containing polygons into a raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lakes.shp --field=ELEV -o=output.tif --nodata --cell_size=10.0
        >>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lakes.shp --field=ELEV -o=output.tif --base=existing_raster.tif", short_exe, name).replace("*", &sep);

        VectorPolygonsToRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VectorPolygonsToRaster {
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
        let vector_data = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of polygon type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type.",
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
        let mut freq_data = HashMap::new();
        let mut key: String;
        if !vector_data.attributes.is_field_numeric(field_index) {
            // Warn user of non-numeric
            // if verbose {
            println!("Warning: Non-numeric attributes cannot be directly assigned to raster data. A key will be established.");
            println!("\nKey, Value");
            // }
            // field_name = "FID".to_string(); // Can't use non-numeric field; use FID instead.
            let mut id = 1f64;
            for record_num in 0..vector_data.num_records {
                key = match vector_data.attributes.get_value(record_num, &field_name) {
                    FieldData::Int(val) => val.to_string(),
                    FieldData::Real(val) => val.to_string(),
                    FieldData::Text(val) => val.to_string(),
                    FieldData::Date(val) => val.to_string(),
                    FieldData::Bool(val) => val.to_string(),
                    FieldData::Null => "null".to_string(),
                };
                if !freq_data.contains_key(&key) {
                    println!("{},{}", key, id);
                    freq_data.insert(key, id);
                    id += 1f64;
                }
            }
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
                    FieldData::Real(val) => {
                        attribute_data[record_num] = val;
                    }
                    FieldData::Text(key) => {
                        attribute_data[record_num] = match freq_data.get(&key) {
                            Some(val) => *val,
                            None => 0f64,
                        }
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
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        let mut col: isize;
        let mut row: isize;
        let mut output_something = false;
        let (mut x, mut y): (f64, f64);
        let (mut starting_row, mut ending_row, mut starting_col, mut ending_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let mut holes: Array2D<i32> = Array2D::new(rows, columns, -1i32, -1i32)?;
        let mut record_i32: i32;
        let num_records = vector_data.num_records;
        for record_num in 0..vector_data.num_records {
            let record = vector_data.get_record(record_num);
            record_i32 = (record_num + 1) as i32;
            let rec_bb = BoundingBox::new(record.x_min, record.x_max, record.y_min, record.y_max);
            if rec_bb.overlaps(raster_bb) {
                // first find the holes
                for part in 0..record.num_parts as usize {
                    if record.is_hole(part as i32) {
                        start_point_in_part = record.parts[part] as usize;
                        end_point_in_part = if part < record.num_parts as usize - 1 {
                            record.parts[part + 1] as usize - 1
                        } else {
                            record.num_points as usize - 1
                        };

                        // First, figure out the minimum and maximum row and column for the polygon part
                        starting_row = rows;
                        ending_row = 0;
                        starting_col = columns;
                        ending_col = 0;
                        for p in start_point_in_part..end_point_in_part + 1 {
                            row = output.get_row_from_y(record.points[p].y);
                            col = output.get_column_from_x(record.points[p].x);
                            if row < starting_row {
                                starting_row = row;
                            }
                            if row > ending_row {
                                ending_row = row;
                            }
                            if col < starting_col {
                                starting_col = col;
                            }
                            if col > ending_col {
                                ending_col = col;
                            }
                        }

                        if starting_row < 0 {
                            starting_row = 0;
                        }
                        if ending_row < 0 {
                            ending_row = 0;
                        }
                        if starting_row >= rows {
                            starting_row = rows - 1;
                        }
                        if ending_row >= rows {
                            ending_row = rows - 1;
                        }

                        if starting_col < 0 {
                            starting_col = 0;
                        }
                        if ending_col < 0 {
                            ending_col = 0;
                        }
                        if starting_col >= columns {
                            starting_col = columns - 1;
                        }
                        if ending_col >= columns {
                            ending_col = columns - 1;
                        }

                        for r in starting_row..=ending_row {
                            y = output.get_y_from_row(r);
                            for c in starting_col..=ending_col {
                                x = output.get_x_from_column(c);
                                if point_in_poly(
                                    &Point2D { x: x, y: y },
                                    &record.points[start_point_in_part..end_point_in_part + 1],
                                ) {
                                    // output.set_value(r, c, background_val);
                                    holes.set_value(r, c, record_i32);
                                }
                            }
                            if verbose {
                                progress = (100.0_f64 * r as f64
                                    / (ending_row - starting_row + 1) as f64)
                                    as usize;
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
                }

                for part in 0..record.num_parts as usize {
                    if !record.is_hole(part as i32) {
                        start_point_in_part = record.parts[part] as usize;
                        end_point_in_part = if part < record.num_parts as usize - 1 {
                            record.parts[part + 1] as usize - 1
                        } else {
                            record.num_points as usize - 1
                        };

                        // First, figure out the minimum and maximum row and column for the polygon part
                        starting_row = rows;
                        ending_row = 0;
                        starting_col = columns;
                        ending_col = 0;
                        for p in start_point_in_part..end_point_in_part + 1 {
                            row = output.get_row_from_y(record.points[p].y);
                            col = output.get_column_from_x(record.points[p].x);
                            if row < starting_row {
                                starting_row = row;
                            }
                            if row > ending_row {
                                ending_row = row;
                            }
                            if col < starting_col {
                                starting_col = col;
                            }
                            if col > ending_col {
                                ending_col = col;
                            }
                        }

                        if starting_row < 0 {
                            starting_row = 0;
                        }
                        if ending_row < 0 {
                            ending_row = 0;
                        }
                        if starting_row >= rows {
                            starting_row = rows - 1;
                        }
                        if ending_row >= rows {
                            ending_row = rows - 1;
                        }

                        if starting_col < 0 {
                            starting_col = 0;
                        }
                        if ending_col < 0 {
                            ending_col = 0;
                        }
                        if starting_col >= columns {
                            starting_col = columns - 1;
                        }
                        if ending_col >= columns {
                            ending_col = columns - 1;
                        }

                        for r in starting_row..=ending_row {
                            y = output.get_y_from_row(r);
                            for c in starting_col..=ending_col {
                                x = output.get_x_from_column(c);
                                if point_in_poly(
                                    &Point2D { x: x, y: y },
                                    &record.points[start_point_in_part..end_point_in_part + 1],
                                ) {
                                    if holes.get_value(r, c) != record_i32 {
                                        output.set_value(r, c, attribute_data[record_num]);
                                        output_something = true;
                                    }
                                }
                            }
                            if verbose {
                                progress = (100.0_f64 * r as f64
                                    / (ending_row - starting_row + 1) as f64)
                                    as usize;
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
            println!("Warning: No polygons were output to the raster.");
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
