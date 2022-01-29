/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/10/2019
Last Modified: 29/12/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::{Array2D, BoundingBox};
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool decrements (lowers) the elevations of pixels within an input digital elevation model (DEM) (`--dem`)
/// along an input vector stream network (`--streams`) at the sites of road (`--roads`) intersections. In addition
/// to the input data layers, the user must specify the output raster DEM (`--output`), and the maximum road embankment width
/// (`--width`), in map units. The road width parameter is used to determine the length of channel along stream
/// lines, at the junctions between streams and roads, that the burning (i.e. decrementing) operation occurs. The
/// algorithm works by identifying stream-road intersection cells, then traversing along the rasterized stream path
/// in the upstream and downstream directions by half the maximum road embankment width. The minimum elevation in each
/// stream traversal is identified and then elevations that are higher than this value are lowered to the minimum
/// elevation during a second stream traversal.
///
/// ![](../../doc_img/BreachStreamsAtRoads.png)
///
/// # Reference
/// Lindsay JB. 2016. [The practice of DEM stream burning revisited](https://onlinelibrary.wiley.com/doi/abs/10.1002/esp.3888). 
/// Earth Surface Processes and Landforms, 41(5): 658–668. DOI: 10.1002/esp.3888
///
/// # See Also
/// `RasterStreamsToVector`, `RasterizeStreams`
pub struct BurnStreamsAtRoads {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BurnStreamsAtRoads {
    pub fn new() -> BurnStreamsAtRoads {
        // public constructor
        let name = "BurnStreamsAtRoads".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Burns-in streams at the sites of road embankments.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster digital elevation model (DEM) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

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
            name: "Input Vector Roads File".to_owned(),
            flags: vec!["--roads".to_owned()],
            description: "Input vector roads file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
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

        parameters.push(ToolParameter {
            name: "Road Embankment Width".to_owned(),
            flags: vec!["--width".to_owned()],
            description: "Maximum road embankment width, in map units".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=raster.tif --streams=streams.shp --roads=roads.shp -o=output.tif --width=50.0", short_exe, name).replace("*", &sep);

        BurnStreamsAtRoads {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BurnStreamsAtRoads {
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
        let mut roads_file = String::new();
        let mut dem_file = String::new();
        let mut output_file = String::new();
        let mut road_width = 0f64;

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
            if flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-roads" {
                roads_file = if keyval {
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
                road_width = if keyval {
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

        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !roads_file.contains(&sep) && !roads_file.contains("/") {
            roads_file = format!("{}{}", working_directory, roads_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading streams and roads data...")
        };
        let streams = Shapefile::read(&streams_file)?;
        let roads = Shapefile::read(&roads_file)?;

        if verbose {
            println!("Reading DEM raster...")
        };
        let dem = Raster::new(&dem_file, "r")?;
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let max_elev = dem.configs.maximum;
        let grid_res = (dem.configs.resolution_x + dem.configs.resolution_y) / 2f64;
        let width_in_cells = (road_width / grid_res).ceil() as usize / 2;

        let start = Instant::now();

        // make sure the input vector file is of lines type
        if streams.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector streams data must be of polyline base shape type.",
            ));
        }

        if roads.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector roads data must be of polyline base shape type.",
            ));
        }

        // create the output raster file
        let mut output = Raster::initialize_using_file(&output_file, &dem);
        output.set_data_from_raster(&dem)?;
        drop(dem);

        let mut raster_lines: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;

        // First rasterize the streams.
        // let mut z: f64;
        let mut col: isize;
        let mut row: isize;
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
                if raster_lines.get_value(row, col) == 0 {
                    raster_lines.set_value(row, col, 1i8);
                }

                row = output.get_row_from_y(record.points[end_point_in_part].y);
                col = output.get_column_from_x(record.points[end_point_in_part].x);
                if raster_lines.get_value(row, col) == 0 {
                    raster_lines.set_value(row, col, 1i8);
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

                                if raster_lines.get_value(row, col) == 0 {
                                    raster_lines.set_value(row, col, 1i8);
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
                                if raster_lines.get_value(row, col) == 0 {
                                    raster_lines.set_value(row, col, 1i8);
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

        // Now find the road intersections.
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut intersections = vec![];
        count = 0f64;
        for record_num in 0..roads.num_records {
            let record = roads.get_record(record_num);
            for part in 0..record.num_parts as usize {
                start_point_in_part = record.parts[part] as usize;
                if part < record.num_parts as usize - 1 {
                    end_point_in_part = record.parts[part + 1] as usize - 1;
                } else {
                    end_point_in_part = record.num_points as usize - 1;
                }

                row = output.get_row_from_y(record.points[start_point_in_part].y);
                col = output.get_column_from_x(record.points[start_point_in_part].x);
                if raster_lines.get_value(row, col) == 1i8 {
                    // we have a road/stream intersection cell
                    intersections.push((row, col));
                    raster_lines.set_value(row, col, 4i8);
                } else {
                    raster_lines.set_value(row, col, 2i8);
                }

                row = output.get_row_from_y(record.points[end_point_in_part].y);
                col = output.get_column_from_x(record.points[end_point_in_part].x);
                if raster_lines.get_value(row, col) == 1i8 {
                    // we have a road/stream intersection cell
                    intersections.push((row, col));
                    raster_lines.set_value(row, col, 4i8);
                } else {
                    raster_lines.set_value(row, col, 2i8);
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
                                if raster_lines.get_value(row, col) == 1i8 {
                                    // we have a road/stream intersection cell
                                    intersections.push((row, col));
                                    raster_lines.set_value(row, col, 4i8);
                                } else if raster_lines.get_value(row, col) == 0i8 {
                                    raster_lines.set_value(row, col, 2i8);
                                    if raster_lines.get_value(row + dy[0], col + dx[0]) == 2i8
                                        && raster_lines.get_value(row + dy[7], col + dx[7]) == 1i8
                                        && raster_lines.get_value(row + dy[1], col + dx[1]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[2], col + dx[2]) == 2i8
                                        && raster_lines.get_value(row + dy[3], col + dx[3]) == 1i8
                                        && raster_lines.get_value(row + dy[1], col + dx[1]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[4], col + dx[4]) == 2i8
                                        && raster_lines.get_value(row + dy[3], col + dx[3]) == 1i8
                                        && raster_lines.get_value(row + dy[5], col + dx[5]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[6], col + dx[6]) == 2i8
                                        && raster_lines.get_value(row + dy[7], col + dx[7]) == 1i8
                                        && raster_lines.get_value(row + dy[5], col + dx[5]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }
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
                                if raster_lines.get_value(row, col) == 1i8 {
                                    // we have a road/stream intersection cell
                                    intersections.push((row, col));
                                    raster_lines.set_value(row, col, 4i8);
                                } else if raster_lines.get_value(row, col) == 0i8 {
                                    raster_lines.set_value(row, col, 2i8);
                                    if raster_lines.get_value(row + dy[0], col + dx[0]) == 2i8
                                        && raster_lines.get_value(row + dy[7], col + dx[7]) == 1i8
                                        && raster_lines.get_value(row + dy[1], col + dx[1]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[2], col + dx[2]) == 2i8
                                        && raster_lines.get_value(row + dy[3], col + dx[3]) == 1i8
                                        && raster_lines.get_value(row + dy[1], col + dx[1]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[4], col + dx[4]) == 2i8
                                        && raster_lines.get_value(row + dy[3], col + dx[3]) == 1i8
                                        && raster_lines.get_value(row + dy[5], col + dx[5]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }

                                    if raster_lines.get_value(row + dy[6], col + dx[6]) == 2i8
                                        && raster_lines.get_value(row + dy[7], col + dx[7]) == 1i8
                                        && raster_lines.get_value(row + dy[5], col + dx[5]) == 1i8
                                    {
                                        // we have a road/stream intersection cell
                                        intersections.push((row, col));
                                        raster_lines.set_value(row, col, 4i8);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            count += 1f64;
            if verbose {
                progress = (100.0_f64 * count / (roads.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding road intersections: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for cell in &intersections {
            let row = cell.0;
            let col = cell.1;
            let mut neighbouring_intersection = false;
            for d in 0..8 {
                if raster_lines.get_value(row + dy[d], col + dx[d]) == 4i8 {
                    neighbouring_intersection = true;
                    break;
                }
            }
            if neighbouring_intersection {
                raster_lines.set_value(row, col, 1)
            }
        }

        for cell in &intersections {
            let row = cell.0;
            let col = cell.1;
            if raster_lines.get_value(row, col) == 4i8 {
                // it's still an intersection; some will have been removed because they touch others
                let mut stack = vec![];
                let mut minz = max_elev;
                for e in 0..8 {
                    if raster_lines.get_value(row + dy[e], col + dx[e]) == 1i8 {
                        stack.push((row + dy[e], col + dx[e], 1usize)); // the third element is the distance
                        while !stack.is_empty() {
                            let cell2 = stack.pop().expect("Error during pop operation.");
                            let r = cell2.0;
                            let c = cell2.1;
                            if minz > output.get_value(r, c) {
                                minz = output.get_value(r, c);
                            }
                            if cell2.2 + 1 < width_in_cells {
                                for d in 0..8 {
                                    if raster_lines.get_value(r + dy[d], c + dx[d]) == 1i8 {
                                        raster_lines.set_value(r + dy[d], c + dx[d], 3i8);
                                        stack.push((r + dy[d], c + dx[d], cell2.2 + 1));
                                    }
                                }
                            }
                        }
                    }
                }

                output.set_value(row, col, minz);

                for e in 0..8 {
                    if raster_lines.get_value(row + dy[e], col + dx[e]) == 3i8 {
                        stack.push((row + dy[e], col + dx[e], 1usize)); // the third element is the distance
                        while !stack.is_empty() {
                            let cell2 = stack.pop().expect("Error during pop operation.");
                            let r = cell2.0;
                            let c = cell2.1;
                            if output.get_value(r, c) > minz {
                                output.set_value(r, c, minz);
                            }
                            if cell2.2 + 1 < width_in_cells {
                                for d in 0..8 {
                                    if raster_lines.get_value(r + dy[d], c + dx[d]) == 3i8 {
                                        raster_lines.set_value(r + dy[d], c + dx[d], 1i8);
                                        stack.push((r + dy[d], c + dx[d], cell2.2 + 1));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // for row in 0..rows {
        //     for col in 0..columns {
        //         output.set_value(row, col, raster_lines.get_value(row, col) as f64);
        //     }
        // }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Input roads file: {}", roads_file));
        output.add_metadata_entry(format!("Input base file: {}", dem_file));
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
