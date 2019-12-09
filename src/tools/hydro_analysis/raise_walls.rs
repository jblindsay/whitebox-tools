/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/04/2018
Last Modified: 22/10/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::{Array2D, BoundingBox};
use crate::tools::*;
use crate::vector::{ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool is used to increment the elevations in a digital elevation model (DEM) along 
/// the boundaries of a vector lines or polygon layer. The user must specify the name of the 
/// raster DEM (`--dem`), the vector file (`--input`), the output file name (`--output`), the 
/// increment height (`--height`), and an optional breach lines vector layer (`--breach`). 
/// The breach lines layer can be used to breach a whole in the raised walls at intersections 
/// with the wall layer.
pub struct RaiseWalls {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RaiseWalls {
    /// public constructor
    pub fn new() -> RaiseWalls {
        let name = "RaiseWalls".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Raises walls in a DEM along a line or around a polygon, e.g. a watershed.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Line or Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "walls".to_owned(), "--input".to_owned()],
            description: "Input vector lines or polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Breach Lines (optional)".to_owned(),
            flags: vec!["--breach".to_owned()],
            description: "Optional input vector breach lines.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
            name: "Wall Height".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "Wall height.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("100.0".to_string()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=watershed.shp --dem=dem.tif -o=output.tif --height=25.0
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=watershed.shp --breach=outlet.shp --dem=dem.tif -o=output.tif --height=25.0", short_exe, name).replace("*", &sep);

        RaiseWalls {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RaiseWalls {
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
        let mut breach_file = String::new();
        let mut dem_file = String::new();
        let mut output_file = String::new();
        let mut wall_height = 100f64;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-walls" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-breach" {
                breach_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-dem" {
                dem_file = if keyval {
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
            } else if flag_val == "-height" {
                wall_height = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
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
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let vector_data = Shapefile::read(&input_file)?;

        // read the DEM into memory
        let dem = Raster::new(&dem_file, "r")?;

        let start = Instant::now();

        // make sure the input vector file is of polygon or polyline type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Polygon
            && vector_data.header.shape_type.base_shape_type() != ShapeType::PolyLine
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of a polyline or polygon base shape type.",
            ));
        }

        let mut output = Raster::initialize_using_file(&output_file, &dem);
        output.set_data_from_raster(&dem)?;
        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;
        let nodata = output.configs.nodata;

        let mut walled: Array2D<u8> = Array2D::new(rows, columns, 0, 0)?;
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
        let mut z: f64;
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
                                    z = output.get_value(row, col);
                                    if z != nodata && walled.get_value(row, col) == 0u8 {
                                        output.set_value(row, col, z + wall_height);
                                        output_something = true;
                                        walled.set_value(row, col, 1u8);
                                    }
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

                                    z = output.get_value(row, col);
                                    if z != nodata && walled.get_value(row, col) == 0u8 {
                                        output.set_value(row, col, z + wall_height);
                                        output_something = true;
                                        walled.set_value(row, col, 1u8);
                                    }
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

        // thicken the wall such that one can't pass through diagonals
        for row in 0..rows {
            for col in 0..columns {
                if walled.get_value(row, col) == 1u8 {
                    // - x o
                    // - o x
                    // - - -
                    if walled.get_value(row - 1, col + 1) == 1u8 {
                        if walled.get_value(row - 1, col) == 0u8
                            && walled.get_value(row, col + 1) == 0u8
                        {
                            output.increment(row - 1, col, wall_height);
                            walled.set_value(row - 1, col, 1u8);
                        }
                    }

                    // - - -
                    // - o x
                    // - x o
                    if walled.get_value(row + 1, col + 1) == 1u8 {
                        if walled.get_value(row, col + 1) == 0u8
                            && walled.get_value(row + 1, col) == 0u8
                        {
                            output.increment(row, col + 1, wall_height);
                            walled.set_value(row, col + 1, 1u8);
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // If breach lines are provided, breach the wall along the lines
        if !breach_file.trim().is_empty() {
            if !breach_file.contains(&sep) && !breach_file.contains("/") {
                breach_file = format!("{}{}", working_directory, breach_file);
            }

            let breach_data = Shapefile::read(&breach_file)?;

            // make sure the input vector file is of polygon or polyline type
            if breach_data.header.shape_type.base_shape_type() != ShapeType::PolyLine {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input breach vector data must be of polyline base shape type.",
                ));
            }

            let num_records = breach_data.num_records;
            for record_num in 0..breach_data.num_records {
                let record = breach_data.get_record(record_num);
                let rec_bb =
                    BoundingBox::new(record.x_min, record.x_max, record.y_min, record.y_max);
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
                                if is_between(
                                    row_y_coord,
                                    record.points[i].y,
                                    record.points[i + 1].y,
                                ) {
                                    y1 = record.points[i].y;
                                    y2 = record.points[i + 1].y;
                                    if y2 != y1 {
                                        x1 = record.points[i].x;
                                        x2 = record.points[i + 1].x;

                                        // calculate the intersection point
                                        x_prime = x1 + (row_y_coord - y1) / (y2 - y1) * (x2 - x1);
                                        let col = output.get_column_from_x(x_prime);
                                        z = dem.get_value(row, col);
                                        if output.get_value(row, col) != z {
                                            output.set_value(row, col, z);
                                            output_something = true;
                                        }
                                    }
                                }
                            }
                        }

                        // find each intersection with a column.
                        for col in left_col..right_col + 1 {
                            col_x_coord = output.get_x_from_column(col);
                            for i in start_point_in_part..end_point_in_part {
                                if is_between(
                                    col_x_coord,
                                    record.points[i].x,
                                    record.points[i + 1].x,
                                ) {
                                    x1 = record.points[i].x;
                                    x2 = record.points[i + 1].x;
                                    if x1 != x2 {
                                        y1 = record.points[i].y;
                                        y2 = record.points[i + 1].y;

                                        // calculate the intersection point
                                        y_prime = y1 + (col_x_coord - x1) / (x2 - x1) * (y2 - y1);

                                        let row = output.get_row_from_y(y_prime);

                                        z = dem.get_value(row, col);
                                        if output.get_value(row, col) != z {
                                            output.set_value(row, col, z);
                                            output_something = true;
                                        }
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

fn is_between(val: f64, threshold1: f64, threshold2: f64) -> bool {
    if val == threshold1 || val == threshold2 {
        return true;
    }
    if threshold2 > threshold1 {
        return val > threshold1 && val < threshold2;
    }
    val > threshold2 && val < threshold1
}
