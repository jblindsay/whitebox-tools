/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/03/2018
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

/// This tool can be used rasterize an input vector stream network (`--streams`) using on Lindsay (2016) method.
/// The user must specify the name of an existing raster (`--base`), from which the output raster's grid resolution
/// is determined.
///
/// # Reference
/// Lindsay JB. 2016. The practice of DEM stream burning revisited. Earth Surface Processes and Landforms,
/// 41(5): 658–668. DOI: 10.1002/esp.3888
///
/// # See Also
/// `RasterStreamsToVector`
pub struct RasterizeStreams {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterizeStreams {
    pub fn new() -> RasterizeStreams {
        // public constructor
        let name = "RasterizeStreams".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Rasterizes vector streams based on Lindsay (2016) method.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input vector streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Base Raster File".to_owned(),
            flags: vec!["--base".to_owned()],
            description: "Input base raster file.".to_owned(),
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
            name: "Use NoData value for background?".to_owned(),
            flags: vec!["--nodata".to_owned()],
            description: "Use NoData value for background?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use feature number as output value?".to_owned(),
            flags: vec!["--feature_id".to_owned()],
            description: "Use feature number as output value?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=streams.shp --base=raster.tif -o=output.tif", short_exe, name).replace("*", &sep);

        RasterizeStreams {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterizeStreams {
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
        let mut streams_file = String::new();
        let mut base_file = String::new();
        let mut output_file = String::new();
        let mut out_nodata = false;
        let mut background_val = 0f64;
        let mut feature_id = false;

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
            if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-base" {
                base_file = if keyval {
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
            } else if flag_val == "-nodata" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    out_nodata = true;
                }
            } else if flag_val == "-feature_id" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    feature_id = true;
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

        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !base_file.contains(&sep) && !base_file.contains("/") {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading streams data...")
        };
        let streams = Shapefile::read(&streams_file)?;

        if verbose {
            println!("Reading base raster data...")
        };
        let base = Raster::new(&base_file, "r")?;
        let rows = base.configs.rows as isize;
        let columns = base.configs.columns as isize;
        let nodata = base.configs.nodata;

        if out_nodata {
            background_val = nodata;
        }

        let start = Instant::now();

        // make sure the input vector file is of lines type
        if streams.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polyline base shape type.",
            ));
        }

        // create the output raster file
        let mut output = Raster::initialize_using_file(&output_file, &base);
        if !out_nodata {
            output.reinitialize_values(background_val);
        }
        let mut link_end_nodes: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut z: f64;
        let mut col: isize;
        let mut row: isize;
        let mut num_stream_cells = 0;
        let mut num_link_collisions = 0;
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
        let mut count = 0f64;
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        for record_num in 0..streams.num_records {
            let record = streams.get_record(record_num);
            for part in 0..record.num_parts as usize {
                start_point_in_part = record.parts[part] as usize;
                if part < record.num_parts as usize - 1 {
                    end_point_in_part = record.parts[part + 1] as usize - 1;
                } else {
                    end_point_in_part = record.num_points as usize - 1;
                }

                row = output.get_row_from_y(record.points[start_point_in_part].y);
                col = output.get_column_from_x(record.points[start_point_in_part].x);
                if output.get_value(row, col) == background_val {
                    link_end_nodes.set_value(row, col, 1u8);
                    output.set_value(row, col, (record_num + 1) as f64);
                    num_stream_cells += 1;
                }

                row = output.get_row_from_y(record.points[end_point_in_part].y);
                col = output.get_column_from_x(record.points[end_point_in_part].x);
                if output.get_value(row, col) == background_val {
                    link_end_nodes.set_value(row, col, 1u8);
                    output.set_value(row, col, (record_num + 1) as f64);
                    num_stream_cells += 1;
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

                                if output.get_value(row, col) == background_val {
                                    output.set_value(row, col, (record_num + 1) as f64);
                                    num_stream_cells += 1;
                                } else if output.get_value(row, col) != (record_num + 1) as f64
                                    && link_end_nodes.get_value(row, col) != 1u8
                                {
                                    num_link_collisions += 1;
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
                                if output.get_value(row, col) == background_val {
                                    output.set_value(row, col, (record_num + 1) as f64);
                                    num_stream_cells += 1;
                                } else if output.get_value(row, col) != (record_num + 1) as f64
                                    && link_end_nodes.get_value(row, col) != 1u8
                                {
                                    num_link_collisions += 1;
                                }
                            }
                        }
                    }
                }
            }

            count += 1f64;
            if verbose {
                progress = (100.0_f64 * count / (streams.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Rasterizing Streams: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now count the number of stream cell adjacencies
        let mut num_adjacencies = 0;
        let (mut id, mut id_n): (f64, f64);
        let mut is_adjacent: bool;
        let dx4 = [1isize, 0isize, -1isize, 0isize];
        let dy4 = [0isize, 1isize, 0isize, -1isize];
        let (mut row_n, mut col_n): (isize, isize);
        println!("Counting stream cell adjacencies...");
        for row in 0..rows {
            for col in 0..columns {
                id = output.get_value(row, col);
                if id != background_val && link_end_nodes.get_value(row, col) != 1u8 {
                    //  it's a stream cell
                    is_adjacent = false;
                    for n in 0..4 {
                        row_n = row + dy4[n];
                        col_n = col + dx4[n];
                        id_n = output.get_value(row_n, col_n);
                        if id_n != id
                            && id_n != background_val
                            && link_end_nodes.get_value(row_n, col_n) != 1u8
                        {
                            is_adjacent = true;
                            break;
                        }
                    }
                    if is_adjacent {
                        num_adjacencies += 1;
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

        if !feature_id {
            println!("Updating output...");
            for row in 0..rows {
                for col in 0..columns {
                    z = output.get_value(row, col);
                    if z != background_val && z > 0f64 {
                        //  it's a stream cell
                        output.set_value(row, col, 1f64);
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
        }

        if verbose {
            println!("Number of stream cells: {}", num_stream_cells);
            println!("Number of stream collisions: {}", num_link_collisions);
            println!("Number of stream adjacencies: {}", num_adjacencies);
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Input base file: {}", base_file));
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

fn is_between(val: f64, threshold1: f64, threshold2: f64) -> bool {
    if val == threshold1 || val == threshold2 {
        return true;
    }
    if threshold2 > threshold1 {
        return val > threshold1 && val < threshold2;
    }
    val > threshold2 && val < threshold1
}
