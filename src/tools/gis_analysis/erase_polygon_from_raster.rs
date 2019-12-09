/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/03/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::algorithms;
use crate::raster::*;
use crate::structures::Point2D;
use crate::tools::*;
use crate::vector::{ShapeType, Shapefile};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to set values an input raster (`--input`) to a NoData background value with a vector 
/// erasing polygon (`--polygons`). The input erase polygon file must be a vector of a Polygon base shape type. 
/// The erase file may contain multiple polygon features. Polygon hole parts will be respected during clipping, i.e. 
/// polygon holes will not be removed from the output raster. Raster grid cells that fall inside of a polygons in 
/// the erase file will be assigned the NoData background value in the output file.
/// 
/// # See Also
/// `ClipRasterToPolygon`
pub struct ErasePolygonFromRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ErasePolygonFromRaster {
    /// public constructor
    pub fn new() -> ErasePolygonFromRaster {
        let name = "ErasePolygonFromRaster".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Erases (cuts out) a vector polygon from a raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Vector Polygon File".to_owned(),
            flags: vec!["--polygons".to_owned()],
            description: "Input vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='DEM.tif' --polygons='lakes.shp' -o='output.tif'", short_exe, name).replace("*", &sep);

        ErasePolygonFromRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ErasePolygonFromRaster {
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
        let mut polygons_file = String::new();
        let mut output_file = String::new();

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
            } else if flag_val == "-polygon" || flag_val == "-polygons" {
                polygons_file = if keyval {
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

        if verbose {
            println!("Reading data...")
        };
        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let polygons = Shapefile::read(&polygons_file)?;

        // make sure the input vector file is of points type
        if polygons.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polygon base shape type.",
            ));
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        match output.set_data_from_raster(&input) {
            Ok(_) => (), // do nothings
            Err(err) => return Err(err),
        }

        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        let (mut row, mut col): (isize, isize);
        let (mut x, mut y): (f64, f64);
        let (mut starting_row, mut ending_row, mut starting_col, mut ending_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let num_records = polygons.num_records;
        for record_num in 0..polygons.num_records {
            let record = polygons.get_record(record_num);

            let mut part_num = 1;
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
                    for p in start_point_in_part..end_point_in_part + 1 {
                        row = input.get_row_from_y(record.points[p].y);
                        col = input.get_column_from_x(record.points[p].x);
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

                    for r in starting_row..ending_row {
                        y = input.get_y_from_row(r);
                        for c in starting_col..ending_col {
                            x = input.get_x_from_column(c);
                            if algorithms::point_in_poly(
                                &Point2D { x: x, y: y },
                                &record.points[start_point_in_part..end_point_in_part + 1],
                            ) {
                                output.set_value(r, c, nodata);
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * r as f64 / (ending_row - starting_row) as f64)
                                as usize;
                            if progress != old_progress {
                                println!(
                                    "Progress (rec {} of {} part {}): {}%",
                                    record_num + 1,
                                    num_records,
                                    part_num,
                                    progress
                                );
                                old_progress = progress;
                            }
                        }
                    }
                    part_num += 1;
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
                    for p in start_point_in_part..end_point_in_part + 1 {
                        row = input.get_row_from_y(record.points[p].y);
                        col = input.get_column_from_x(record.points[p].x);
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

                    for r in starting_row..ending_row {
                        y = input.get_y_from_row(r);
                        for c in starting_col..ending_col {
                            x = input.get_x_from_column(c);
                            if algorithms::point_in_poly(
                                &Point2D { x: x, y: y },
                                &record.points[start_point_in_part..end_point_in_part + 1],
                            ) {
                                output.set_value(r, c, input.get_value(r, c));
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * r as f64 / (ending_row - starting_row) as f64)
                                as usize;
                            if progress != old_progress {
                                println!(
                                    "Progress (rec {} of {} part {}): {}%",
                                    record_num + 1,
                                    num_records,
                                    part_num,
                                    progress
                                );
                                old_progress = progress;
                            }
                        }
                    }
                    part_num += 1;
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
