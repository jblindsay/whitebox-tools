/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 18/02/2020
Last Modified: 05/03/2020
License: MIT
*/

use whitebox_common::algorithms::is_clockwise_order;
use whitebox_raster::*;
use whitebox_common::structures::{Array2D, Point2D};
use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// Converts a raster data set to a vector of the POLYGON geometry type. The user must specify
/// the name of a raster file (`--input`) and the name of the output (`--output`) vector. All grid cells containing
/// non-zero, non-NoData values will be considered part of a polygon feature. The vector's attribute table
/// will contain a field called 'VALUE' that will contain the cell value for each polygon
/// feature, in addition to the standard feature ID (FID) attribute.
///
/// # See Also
/// `RasterToVectorPoints`, `RasterToVectorLines`
pub struct RasterToVectorPolygons {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterToVectorPolygons {
    pub fn new() -> RasterToVectorPolygons {
        // public constructor
        let name = "RasterToVectorPolygons".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Converts a raster dataset to a vector of the POLYGON shapetype.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Polygons File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygons file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=points.tif -o=out.shp",
            short_exe, name
        )
        .replace("*", &sep);

        RasterToVectorPolygons {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterToVectorPolygons {
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
        let mut input_file = String::new();
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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
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
        let res_x = input.configs.resolution_x;
        let res_y = input.configs.resolution_y;
        let half_res_x = res_x / 2f64;
        let half_res_y = res_y / 2f64;
        let west = input.configs.west;
        let north = input.configs.north;

        let get_x_from_column = |col| -> f64 { west + half_res_x + col as f64 * res_x };
        let get_y_from_row = |row| -> f64 { north - half_res_y - row as f64 * res_y };

        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 10u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "VALUE",
            FieldDataType::Real,
            12u8,
            4u8,
        ));

        let dx = [0, 1, 0, -1, 1, 1, -1, -1];
        let dy = [-1, 0, 1, 0, -1, 1, 1, -1];
        let (mut rn, mut cn): (isize, isize);
        let (mut z, mut zn): (f64, f64);

        // Clump the input raster
        let mut clumps: Array2D<u32> = Array2D::new(rows, columns, 0u32, 0u32)?;
        let mut visited: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut queue = VecDeque::new();
        let mut clump_val = 1u32;
        let mut clump_to_value = vec![];
        clump_to_value.push(0f64); // clump values start at 1
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata && z != 0f64 && visited.get_value(row, col) != 1 {
                    clump_to_value.push(z);
                    clumps.set_value(row, col, clump_val);
                    visited.set_value(row, col, 1);
                    queue.push_back((row, col));
                    while let Some(cell) = queue.pop_front() {
                        for n in 0..8 {
                            rn = cell.0 + dy[n];
                            cn = cell.1 + dx[n];
                            zn = input.get_value(rn, cn);
                            if z == zn && visited.get_value(rn, cn) != 1 {
                                clumps.increment(rn, cn, clump_val);
                                visited.set_value(rn, cn, 1);
                                queue.push_back((rn, cn));
                            }
                        }
                    }
                    clump_val += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Clumping polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input);
        drop(visited);

        /*  Diagram 1:
         *  Edge Numbering (shared edges between cells)
         *  _____________
         *  |     |     |
         *  |     3     |
         *  |__2__|__0__|
         *  |     |     |
         *  |     1     |
         *  |_____|_____|
         *
         */

        /* Diagram 2:
         * Cell Edge Numbering
         *
         *  ___0___
         * |       |
         * |       |
         * 3       1
         * |       |
         * |___2___|
         *
         */

        const EPSILON: f64 = std::f64::EPSILON;
        let prec = (5f64 * EPSILON).tan();
        let (mut p1, mut p2, mut p3): (Point2D, Point2D, Point2D);
        let mut z: u32;
        let mut zn: u32;
        let (mut x, mut y): (f64, f64);
        let (mut edge_x, mut edge_y): (f64, f64);
        let mut line_segments: Vec<LineSegment> = vec![];
        let edge_offsets_pt1_x = [-half_res_x, half_res_x, half_res_x, -half_res_x];
        let edge_offsets_pt1_y = [half_res_y, half_res_y, -half_res_y, -half_res_y];
        let edge_offsets_pt3_x = [half_res_x, half_res_x, -half_res_x, -half_res_x];
        let edge_offsets_pt3_y = [half_res_y, -half_res_y, -half_res_y, half_res_y];
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
        let mut endnode = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                z = clumps.get_value(row, col);
                if z != 0 {
                    for n in 0..4 {
                        zn = clumps.get_value(row + dy[n], col + dx[n]);
                        if z != zn {
                            x = get_x_from_column(col);
                            y = get_y_from_row(row);

                            edge_x = x + edge_offsets_pt1_x[n];
                            edge_y = y + edge_offsets_pt1_y[n];
                            p1 = Point2D::new(edge_x, edge_y);

                            tree.add([p1.x, p1.y], endnode).unwrap();
                            endnode += 1;

                            edge_x = x + edge_offsets_pt3_x[n];
                            edge_y = y + edge_offsets_pt3_y[n];
                            p2 = Point2D::new(edge_x, edge_y);

                            tree.add([p2.x, p2.y], endnode).unwrap();
                            endnode += 1;

                            line_segments.push(LineSegment::new(p1, p2, z));
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding edges: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(clumps);

        let mut geometries =
            vec![ShapefileGeometry::new(ShapeType::Polygon); clump_val as usize - 1];
        let mut node_live = vec![true; line_segments.len() * 2];
        let num_nodes = line_segments.len() * 2;
        let mut line_segment_n: usize;
        let mut current_node: usize;
        let mut node_n: usize;
        let mut heading: f64;
        let mut max_heading: f64;
        let mut node_of_max_deflection: usize;
        let mut line_segment: usize;
        let mut line_start: usize;
        let mut flag: bool;
        for node in 0..line_segments.len() * 2 {
            if node_live[node] {
                line_segment = node / 2;
                z = line_segments[line_segment].value;

                line_start = node;
                current_node = node;
                let mut points = vec![];
                flag = true;
                while flag {
                    line_segment_n = current_node / 2;

                    // Add the current_node to points.
                    p1 = if current_node % 2 == 0 {
                        line_segments[line_segment_n].first_vertex()
                    } else {
                        line_segments[line_segment_n].last_vertex()
                    };
                    points.push(p1);
                    node_live[current_node] = false;

                    // We've now added both ends of this segment. Find the next connecting segment.
                    let ret = tree
                        .within(&[p1.x, p1.y], prec, &squared_euclidean)
                        .unwrap();

                    let mut connected_nodes: Vec<usize> = Vec::with_capacity(ret.len());
                    for a in 0..ret.len() {
                        node_n = *ret[a].1;
                        line_segment_n = node_n / 2;
                        zn = line_segments[line_segment_n].value;
                        if zn == z && node_live[node_n] {
                            connected_nodes.push(node_n);
                        }
                    }

                    if connected_nodes.len() == 0 {
                        // Retrieve the other end
                        current_node = if current_node % 2 == 0 {
                            current_node + 1
                        } else {
                            current_node - 1
                        };

                        // Is the other end of this segment still live? If not, end the trace.
                        if !node_live[current_node] {
                            p1 = if line_start % 2 == 0 {
                                line_segments[line_start / 2].first_vertex()
                            } else {
                                line_segments[line_start / 2].last_vertex()
                            };
                            points.push(p1);
                            // flag = false;
                            break;
                        }
                    } else if connected_nodes.len() == 1 {
                        // only one connected segment; move there.
                        // current_node = connected_nodes[0];
                        current_node = if connected_nodes[0] % 2 == 0 {
                            connected_nodes[0] + 1
                        } else {
                            connected_nodes[0] - 1
                        };
                        node_live[connected_nodes[0]] = false;
                    } else {
                        // connected_nodes.len() >= 2
                        // there are two or more connected segments; choose the node the represents the greatest deflection in path

                        // current point is already in p1.
                        p2 = points[points.len() - 2]; // previous point

                        max_heading = -10f64;
                        node_of_max_deflection = num_nodes;
                        for n in 0..connected_nodes.len() {
                            line_segment_n = connected_nodes[n] / 2;
                            p3 = if connected_nodes[n] % 2 == 0 {
                                // get the other end of this segment
                                line_segments[line_segment_n].last_vertex()
                            } else {
                                line_segments[line_segment_n].first_vertex()
                            };
                            heading = -Point2D::change_in_heading(p2, p1, p3); //.abs(); // go left if you can.
                            if heading > max_heading && heading != 0f64 {
                                // never go straight if you have the option not to.
                                max_heading = heading;
                                node_of_max_deflection = n;
                            }
                        }
                        if node_of_max_deflection < num_nodes {
                            // none found.
                            // current_node = connected_nodes[node_of_max_deflection];
                            // Retrieve the other end
                            current_node = if connected_nodes[node_of_max_deflection] % 2 == 0 {
                                connected_nodes[node_of_max_deflection] + 1
                            } else {
                                connected_nodes[node_of_max_deflection] - 1
                            };
                            node_live[connected_nodes[node_of_max_deflection]] = false;
                        } else {
                            flag = false; // we should not get here
                        }
                    }
                }

                if points.len() > 2 {
                    // Remove unnecessary points
                    for a in (1..points.len() - 1).rev() {
                        p1 = points[a - 1];
                        p2 = points[a];
                        p3 = points[a + 1];
                        if ((p2.y - p1.y) * (p3.x - p2.x) - (p3.y - p2.y) * (p2.x - p1.x)).abs()
                            <= ((p2.x - p1.x) * (p3.x - p2.x) + (p2.y - p1.y) * (p3.y - p2.y)).abs()
                                * prec
                        {
                            points.remove(a);
                        }
                    }
                    if points.len() > 2 {
                        if !points[0].nearly_equals(&points[points.len() - 1]) {
                            points.push(points[0].clone());
                        }

                        if geometries[z as usize - 1].num_parts > 0 {
                            // It's a hole.
                            if is_clockwise_order(&points) {
                                points.reverse();
                            }
                        }
                        geometries[z as usize - 1].add_part(&points);
                    }
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * node as f64 / (line_segments.len() * 2 - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Tracing polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        /*
        let mut geometries =
            vec![ShapefileGeometry::new(ShapeType::Polygon); clump_val as usize - 1];
        let mut segment_live = vec![true; line_segments.len()];
        let num_nodes = line_segments.len() * 2;
        let mut line_segment_n: usize;
        let mut current_node: usize;
        let mut heading: f64;
        let mut max_heading: f64;
        let mut node_of_max_deflection: usize;
        let mut node: usize;
        let mut line_start: usize;
        let mut flag: bool;
        for line_segment in 0..line_segments.len() {
            if segment_live[line_segment] {
                z = line_segments[line_segment].value;

                line_start = line_segment * 2;
                current_node = line_start;
                let mut points = vec![];
                flag = true;
                while flag {
                    line_segment_n = current_node / 2;

                    // Add the current_node to points.
                    // Is the current_node a starting point?
                    p1 = if current_node % 2 == 0 {
                        line_segments[line_segment_n].first_vertex()
                    } else {
                        line_segments[line_segment_n].last_vertex()
                    };
                    points.push(p1);

                    // Is it the first node encountered from this segment?
                    if segment_live[line_segment_n] {
                        segment_live[line_segment_n] = false;
                        // This is the first node encountered from this segment, retrieve the other end
                        current_node = if current_node % 2 == 0 {
                            current_node + 1
                        } else {
                            current_node - 1
                        };
                        // points.push(line_segments[line_segment_n].half_point());
                    } else {
                        // We've now added both ends of this segment. Find the next connecting segment.
                        let ret = tree
                            .within(&[p1.x, p1.y], prec, &squared_euclidean)
                            .unwrap();

                        let mut connected_nodes: Vec<usize> = Vec::with_capacity(ret.len());
                        for a in 0..ret.len() {
                            node = *ret[a].1;
                            line_segment_n = node / 2;
                            zn = line_segments[line_segment_n].value;
                            if zn == z && segment_live[line_segment_n] {
                                connected_nodes.push(node);
                            } else if node == line_start {
                                // println!("End found {}", geometries.len()+1);
                                line_segment_n = line_start / 2;
                                p1 = if line_start % 2 == 0 {
                                    line_segments[line_segment_n].first_vertex()
                                } else {
                                    line_segments[line_segment_n].last_vertex()
                                };
                                points.push(p1);
                                flag = false;
                                break;
                            }
                        }

                        if connected_nodes.len() == 0 {
                            flag = false; // end of the line; no other connected segments
                        } else if connected_nodes.len() == 1 {
                            current_node = connected_nodes[0]; // only one connected segment; move there.
                        } else if connected_nodes.len() >= 2 {
                            // there are two or more connected segments; choose the node the represents the greatest deflection in path
                            // line_segment_n = current_node / 2;
                            // p1 = if current_node % 2 == 0 {
                            //     line_segments[line_segment_n].last_vertex()
                            // } else {
                            //     line_segments[line_segment_n].first_vertex()
                            // };

                            // p2 = if current_node % 2 == 0 {
                            //     line_segments[line_segment_n].first_vertex()
                            // } else {
                            //     line_segments[line_segment_n].last_vertex()
                            // };

                            // current point is already in p1.
                            p2 = points[points.len() - 2]; // previous point

                            max_heading = -10f64;
                            node_of_max_deflection = num_nodes;
                            for n in 0..connected_nodes.len() {
                                line_segment_n = connected_nodes[n] / 2;
                                p3 = if connected_nodes[n] % 2 == 0 {
                                    // get the other end of this segment
                                    line_segments[line_segment_n].last_vertex()
                                } else {
                                    line_segments[line_segment_n].first_vertex()
                                };
                                heading = Point2D::change_in_heading(p2, p1, p3); //.abs(); // go left if you can.
                                if heading > max_heading && heading != 0f64 { // never go straight if you have the option not to.
                                    max_heading = heading;
                                    node_of_max_deflection = n;
                                }
                            }
                            if node_of_max_deflection < num_nodes {
                                current_node = connected_nodes[node_of_max_deflection];
                            } else {
                                flag = false; // we should not get here
                            }
                        }
                    }
                }

                if points.len() > 2 {
                    // Remove unnecessary points
                    for a in (1..points.len() - 1).rev() {
                        p1 = points[a - 1];
                        p2 = points[a];
                        p3 = points[a + 1];
                        if ((p2.y - p1.y) * (p3.x - p2.x) - (p3.y - p2.y) * (p2.x - p1.x)).abs()
                            <= ((p2.x - p1.x) * (p3.x - p2.x) + (p2.y - p1.y) * (p3.y - p2.y)).abs()
                                * prec
                        {
                            points.remove(a);
                        }
                    }
                    if points.len() > 2 {
                        if !points[0].nearly_equals(&points[points.len() - 1]) {
                            points.push(points[0].clone());
                        }

                        // println!("{:?}", points);

                        if geometries[z as usize - 1].num_parts > 0 {
                            // It's a hole.
                            if is_clockwise_order(&points) {
                                points.reverse();
                            }
                        }
                        geometries[z as usize - 1].add_part(&points);
                    }
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * line_segment as f64 / (line_segments.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Tracing polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        */

        for fid in 0..geometries.len() {
            output.add_record(geometries[fid].clone());
            output.attributes.add_record(
                vec![
                    FieldData::Int(fid as i32 + 1),
                    FieldData::Real(clump_to_value[fid + 1]),
                ],
                false,
            );

            if verbose {
                progress = (100.0_f64 * fid as f64 / (geometries.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating geometries: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

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

#[derive(Clone, Copy)]
struct LineSegment {
    p1: Point2D,
    p2: Point2D,
    value: u32,
}

impl LineSegment {
    fn new(p1: Point2D, p2: Point2D, value: u32) -> LineSegment {
        LineSegment {
            p1: p1,
            p2: p2,
            value: value,
        }
    }

    pub fn first_vertex(&self) -> Point2D {
        self.p1
    }

    pub fn last_vertex(&self) -> Point2D {
        self.p2
    }

    // pub fn half_point(&self) -> Point2D {
    //     Point2D::new(
    //         (self.p1.x + self.p2.x) / 2f64,
    //         (self.p1.y + self.p2.y) / 2f64,
    //     )
    // }
}
