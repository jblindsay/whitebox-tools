/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/03/2018
Last Modified: 29/03/2018
License: MIT

NOTES: When support is provided for reading vector attributes tables, this tool should be modified so
that lake elevations can alternatively be specified as an attribute of the vector. In the event that
a lake elevation attribute is not specified, the tool would then default to checking the minimum
elevation on each lake's coastline.
*/

use time;
use std::f64;
use std::env;
use std::path;
use raster::*;
use vector;
use vector::{Shapefile, ShapeType, Point2D};
use std::io::{Error, ErrorKind};
use tools::*;
use structures::BoundingBox;

pub struct FlattenLakes {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FlattenLakes {
    /// public constructor
    pub fn new() -> FlattenLakes { 
        let name = "FlattenLakes".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Flattens lake polygons in a raster DEM.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Input Lakes Vector Polygon File".to_owned(), 
            flags: vec!["--lakes".to_owned()], 
            description: "Input lakes vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(VectorGeometryType::Polygon)),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem='DEM.tif' --lakes='lakes.shp' -o='output.tif'", short_exe, name).replace("*", &sep);
    
        FlattenLakes { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for FlattenLakes {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut polygons_file = String::new();
        let mut output_file = String::new();
         
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters."));
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
            if flag_val == "-i" || flag_val == "-dem" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-lakes" {
                polygons_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
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
        if !polygons_file.contains(&sep) && !polygons_file.contains("/") {
            polygons_file = format!("{}{}", working_directory, polygons_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };
        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let polygons = Shapefile::new(&polygons_file, "r")?;

        // make sure the input vector file is of polygon type
        if polygons.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type."));
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        match output.set_data_from_raster(&input) {
            Ok(_) => (), // do nothings
            Err(err) => return Err(err),
        }

        let mut min_elevs = vec![f64::INFINITY; polygons.num_records];

        // trace the perimeter of each lake and find the minimum elevation
        let mut z: f64;
        let mut col: isize;
        let mut row: isize;
        let mut bb = BoundingBox{ ..Default::default() };
        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (isize, isize, isize, isize);
        let mut row_y_coord: f64;
        let mut col_x_coord: f64;
        let (mut x1, mut x2, mut y1, mut y2): (f64, f64, f64, f64);
        let (mut x_prime, mut y_prime): (f64, f64);
        let mut count = 0f64;
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);
            for part in 0..record.num_parts as usize {
                start_point_in_part = record.parts[part] as usize;
                if part < record.num_parts as usize - 1 {
                    end_point_in_part = record.parts[part + 1] as usize - 1;
                } else {
                    end_point_in_part = record.num_points as usize - 1;
                }

                bb.initialize_to_inf();
                for i in start_point_in_part..end_point_in_part+1 {
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
                top_row = input.get_row_from_y(bb.max_y);
                bottom_row = input.get_row_from_y(bb.min_y);
                left_col = input.get_column_from_x(bb.min_x);
                right_col = input.get_column_from_x(bb.max_x);

                // find each intersection with a row.
                for row in top_row..bottom_row+1 {
                    row_y_coord = input.get_y_from_row(row);
                    // find the x-coordinates of each of the line segments 
                    // that intersect this row's y coordinate
                    for i in start_point_in_part..end_point_in_part {
                        if is_between(row_y_coord, record.points[i].y, record.points[i+1].y) {
                            y1 = record.points[i].y;
                            y2 = record.points[i+1].y;
                            if y2 != y1 {
                                x1 = record.points[i].x;
                                x2 = record.points[i+1].x;

                                // calculate the intersection point
                                x_prime = x1 + (row_y_coord - y1) / (y2 - y1) * (x2 - x1);
                                let col = input.get_column_from_x(x_prime);

                                z = input.get_value(row, col);
                                if z != nodata {
                                    if z < min_elevs[record_num] {
                                        min_elevs[record_num] = z;
                                    }
                                }
                            }
                        }
                    }
                }

                // find each intersection with a column.
                for col in left_col..right_col+1 {
                    col_x_coord = output.get_x_from_column(col);
                    for i in start_point_in_part..end_point_in_part {
                        if is_between(col_x_coord, record.points[i].x, record.points[i+1].x) {
                            x1 = record.points[i].x;
                            x2 = record.points[i+1].x;
                            if x1 != x2 {
                                y1 = record.points[i].y;
                                y2 = record.points[i+1].y;

                                // calculate the intersection point
                                y_prime = y1 + (col_x_coord - x1) / (x2 - x1) * (y2 - y1);

                                let row = output.get_row_from_y(y_prime);
                                
                                z = input.get_value(row, col);
                                if z != nodata {
                                    if z < min_elevs[record_num] {
                                        min_elevs[record_num] = z;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            count += 1f64;
            if verbose {
                progress = (100.0_f64 * count / (polygons.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding lake elevations: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let (mut x, mut y): (f64, f64);
        let (mut starting_row, mut ending_row, mut starting_col, mut ending_col): (isize, isize, isize, isize);
        let num_records = polygons.num_records;
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);
            
            for part in 0..record.num_parts as usize {
                if !record.is_hole(part as i32) {
                    // erase cells from this part

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
                    for p in start_point_in_part..end_point_in_part+1 {
                        row = input.get_row_from_y(record.points[p].y);
                        col = input.get_column_from_x(record.points[p].x);
                        if row < starting_row { starting_row = row; }
                        if row > ending_row { ending_row = row; }
                        if col < starting_col { starting_col = col; }
                        if col > ending_col { ending_col = col; }
                    }

                    for r in starting_row..ending_row {
                        y = input.get_y_from_row(r);
                        for c in starting_col..ending_col {
                            x = input.get_x_from_column(c);
                            if vector::point_in_poly(&Point2D{ x: x, y: y }, &record.points[start_point_in_part..end_point_in_part+1]) {
                                output.set_value(r, c, min_elevs[record_num]);
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * r as f64 / (ending_row - starting_row) as f64) as usize;
                            if progress != old_progress {
                                println!("Updating lake elevations ({} of {}): {}%", record_num+1, num_records, progress);
                                old_progress = progress;
                            }
                        }
                    }
                }
            }

            for part in 0..record.num_parts as usize {
                if record.is_hole(part as i32) {
                    // add cells from this part back in

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
                    for p in start_point_in_part..end_point_in_part+1 {
                        row = input.get_row_from_y(record.points[p].y);
                        col = input.get_column_from_x(record.points[p].x);
                        if row < starting_row { starting_row = row; }
                        if row > ending_row { ending_row = row; }
                        if col < starting_col { starting_col = col; }
                        if col > ending_col { ending_col = col; }
                    }

                    for r in starting_row..ending_row {
                        y = input.get_y_from_row(r);
                        for c in starting_col..ending_col {
                            x = input.get_x_from_column(c);
                            if vector::point_in_poly(&Point2D{ x: x, y: y }, &record.points[start_point_in_part..end_point_in_part+1]) {
                                output.set_value(r, c, input.get_value(r, c));
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * r as f64 / (ending_row - starting_row) as f64) as usize;
                            if progress != old_progress {
                                println!("Updating lake elevations ({} of {}): {}%", record_num+1, num_records, progress);
                                old_progress = progress;
                            }
                        }
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }
        
        Ok(())
    }
}

#[inline]
fn is_between(val: f64, threshold1: f64, threshold2: f64) -> bool {
    if val == threshold1 || val == threshold2 {
        return true;
    }
    if threshold2 > threshold1 {
        return val > threshold1 && val < threshold2;
    }
    val > threshold2 && val < threshold1
}