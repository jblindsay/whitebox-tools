/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/04/2018
Last Modified: 22/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::{Array2D, BoundingBox};
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use num_cpus;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Burns streams into a DEM using the FillBurn (Saunders, 1999) method which produces a hydro-enforced DEM.
/// This tool uses the algorithm described in:
///
/// Lindsay JB. 2016. The practice of DEM stream burning revisited. Earth Surface Processes
/// and Landforms, 41(5): 658-668. DOI: 10.1002/esp.3888
///
/// And:
///
/// Saunders, W. 1999. Preparation of DEMs for use in environmental modeling analysis, in: ESRI User
/// Conference. pp. 24-30.
///
/// It should be noted that the output DEM will always be of 64-bit floating-point data type,
/// which will often double the storage requirements as DEMs are often stored with 32-bit precision.
/// This is because the tool will determine an appropriate small increment value based on the range of
/// elevation values in the input DEM to ensure that there is a monotonically descending path along breach
/// channels to satisfy the necessary condition of a downslope gradient for flowpath modelling.
pub struct FillBurn {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FillBurn {
    pub fn new() -> FillBurn {
        // public constructor
        let name = "FillBurn".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Burns streams into a DEM using the FillBurn (Saunders, 1999) method.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif --streams=streams.shp -o=dem_burned.tif", short_exe, name).replace("*", &sep);

        FillBurn {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FillBurn {
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
        let mut dem_file = String::new();
        let mut streams_file = String::new();
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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading streams data...")
        };
        let streams = Shapefile::read(&streams_file)?;

        if verbose {
            println!("Reading DEM data...")
        };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;

        let start = Instant::now();

        // make sure the input vector file is of lines type
        if streams.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polyline base shape type.",
            ));
        }

        // create the output raster file
        let mut raster_streams: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut z: f64;
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

                row = dem.get_row_from_y(record.points[start_point_in_part].y);
                col = dem.get_column_from_x(record.points[start_point_in_part].x);
                if raster_streams.get_value(row, col) == 0u8 {
                    raster_streams.set_value(row, col, 1u8);
                }

                row = dem.get_row_from_y(record.points[end_point_in_part].y);
                col = dem.get_column_from_x(record.points[end_point_in_part].x);
                if raster_streams.get_value(row, col) == 0u8 {
                    raster_streams.set_value(row, col, 1u8);
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
                top_row = dem.get_row_from_y(bb.max_y);
                bottom_row = dem.get_row_from_y(bb.min_y);
                left_col = dem.get_column_from_x(bb.min_x);
                right_col = dem.get_column_from_x(bb.max_x);

                // find each intersection with a row.
                for row in top_row..bottom_row + 1 {
                    row_y_coord = dem.get_y_from_row(row);
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
                                let col = dem.get_column_from_x(x_prime);

                                if raster_streams.get_value(row, col) == 0u8 {
                                    raster_streams.set_value(row, col, 1u8);
                                }
                            }
                        }
                    }
                }

                // find each intersection with a column.
                for col in left_col..right_col + 1 {
                    col_x_coord = dem.get_x_from_column(col);
                    for i in start_point_in_part..end_point_in_part {
                        if is_between(col_x_coord, record.points[i].x, record.points[i + 1].x) {
                            x1 = record.points[i].x;
                            x2 = record.points[i + 1].x;
                            if x1 != x2 {
                                y1 = record.points[i].y;
                                y2 = record.points[i + 1].y;

                                // calculate the intersection point
                                y_prime = y1 + (col_x_coord - x1) / (x2 - x1) * (y2 - y1);

                                let row = dem.get_row_from_y(y_prime);
                                if raster_streams.get_value(row, col) == 0u8 {
                                    raster_streams.set_value(row, col, 1u8);
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

        // Perform line-thinning
        let mut did_something = true;
        let mut loop_num = 0;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let elements = vec![
            vec![6, 7, 0, 4, 3, 2],
            vec![7, 0, 1, 3, 5],
            vec![0, 1, 2, 4, 5, 6],
            vec![1, 2, 3, 5, 7],
            vec![2, 3, 4, 6, 7, 0],
            vec![3, 4, 5, 7, 1],
            vec![4, 5, 6, 0, 1, 2],
            vec![5, 6, 7, 1, 3],
        ];

        let vals = vec![
            vec![0u8, 0u8, 0u8, 1u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8, 1u8],
            vec![0u8, 0u8, 0u8, 1u8, 1u8],
        ];

        let mut neighbours = [0u8; 8];
        let mut pattern_match: bool;
        let mut zu8: u8;
        while did_something {
            loop_num += 1;
            did_something = false;
            for a in 0..8 {
                for row in 0..rows {
                    for col in 0..columns {
                        zu8 = raster_streams.get_value(row, col);
                        if zu8 > 0u8 {
                            // fill the neighbours array
                            for i in 0..8 {
                                neighbours[i] = raster_streams.get_value(row + dy[i], col + dx[i]);
                            }

                            // scan through element
                            pattern_match = true;
                            for i in 0..elements[a].len() {
                                if neighbours[elements[a][i]] != vals[a][i] {
                                    pattern_match = false;
                                }
                            }
                            if pattern_match {
                                raster_streams.set_value(row, col, 0u8);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * a as f64 / 7.0) as usize;
                    if progress != old_progress {
                        println!("Line Thinning {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }
        }

        // Make a copy of the DEM where each stream cell
        //  has been lowered by 10,000 elevation units.
        let raster_streams = Arc::new(raster_streams);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let dem = dem.clone();
            let raster_streams = raster_streams.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = dem.get_value(row, col);
                        if raster_streams.get_value(row, col) == 0u8 && z != nodata {
                            data[col as usize] = z;
                        } else if raster_streams.get_value(row, col) == 1u8 && z != nodata {
                            data[col as usize] = z - 10000f64;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &dem);
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing output: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Fill the streams-decremented DEM.
        let mut in_queue: Array2D<u8> = Array2D::new(rows, columns, 0u8, 2u8)?;

        /*
        Find the data edges. This is complicated by the fact that DEMs frequently
        have nodata edges, whereby the DEM does not occupy the full extent of
        the raster. One approach to doing this would be simply to scan the
        raster, looking for cells that neighbour nodata values. However, this
        assumes that there are no interior nodata holes in the dataset. Instead,
        the approach used here is to perform a region-growing operation, looking
        for nodata values along the raster's edges.
        */

        let mut queue: VecDeque<(isize, isize)> =
            VecDeque::with_capacity((rows * columns) as usize);
        for row in 0..rows {
            /*
            Note that this is only possible because Whitebox rasters
            allow you to address cells beyond the raster extent but
            return the nodata value for these regions.
            */
            queue.push_back((row, -1));
            queue.push_back((row, columns));
        }

        for col in 0..columns {
            queue.push_back((-1, col));
            queue.push_back((rows, col));
        }

        /*
        minheap is the priority queue. Note that I've tested using integer-based
        priority values, by multiplying the elevations, but this didn't result
        in a significant performance gain over the use of f64s.
        */
        let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        let num_cells = rows * columns;
        let mut zout: f64; // value of row, col in output raster
        let mut zout_n: f64; // value of neighbour of row, col in output raster
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            row = cell.0;
            col = cell.1;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                if in_queue.get_value(row_n, col_n) == 0u8 {
                    if dem.get_value(row_n, col_n) == nodata {
                        queue.push_back((row_n, col_n));
                    } else {
                        // Push it onto the priority queue for the priority flood operation
                        minheap.push(GridCell {
                            row: row_n,
                            column: col_n,
                            priority: output.get_value(row_n, col_n),
                        });
                    }
                    in_queue.set_value(row_n, col_n, 1u8);
                    num_solved_cells += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Perform the priority flood operation.
        output.configs.data_type = DataType::F64; // Don't take any chances and promote to 64-bit
        let elev_digits = (dem.configs.maximum as i64).to_string().len();
        let elev_multiplier = 10.0_f64.powi((12 - elev_digits) as i32);
        let small_num = 1.0 / elev_multiplier as f64;

        while !minheap.is_empty() {
            let cell = minheap.pop().expect("Error during pop operation.");
            row = cell.row;
            col = cell.column;
            zout = output.get_value(row, col);
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                if in_queue.get_value(row_n, col_n) == 0u8 {
                    zout_n = output.get_value(row_n, col_n);
                    // zin_n = input[(row_n, col_n)];
                    if zout_n != nodata {
                        if zout_n < (zout + small_num) {
                            zout_n = zout + small_num;
                        } // We're in a depression. Raise the elevation.
                        output.set_value(row_n, col_n, zout_n);
                        minheap.push(GridCell {
                            row: row_n,
                            column: col_n,
                            priority: zout_n,
                        });
                    }
                    in_queue.set_value(row_n, col_n, 1u8);
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Filling DEM: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Find the minimum elevation difference between the
        // filled DEM and the original DEM along the
        // stream network and raise all stream cells by this
        // value less 1 m.
        let mut min_diff = f64::INFINITY;
        for row in 0..rows {
            for col in 0..columns {
                if raster_streams.get_value(row, col) > 0u8 && dem.get_value(row, col) != nodata {
                    z = dem.get_value(row, col) - output.get_value(row, col);
                    if z < min_diff {
                        min_diff = z;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Updating Stream Elevations: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        min_diff -= 1f64;

        for row in 0..rows {
            for col in 0..columns {
                if raster_streams.get_value(row, col) > 0u8 && dem.get_value(row, col) != nodata {
                    z = output.get_value(row, col) + min_diff;
                    output.set_value(row, col, z);
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Updating Stream Elevations: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        // output.configs.palette = "qual.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Input DEM file: {}", dem_file));
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

#[derive(PartialEq, Debug)]
struct GridCell {
    row: isize,
    column: isize,
    // priority: usize,
    priority: f64,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &GridCell) -> Option<Ordering> {
        // Some(other.priority.cmp(&self.priority))
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell {
    fn cmp(&self, other: &GridCell) -> Ordering {
        // other.priority.cmp(&self.priority)
        let ord = self.partial_cmp(other).unwrap();
        match ord {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => ord,
        }
    }
}
