/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/02/2020
Last Modified: 04/03/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::{Array2D, Point2D};
use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
const EPSILON: f64 = std::f64::EPSILON;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to map road and rail embankments in a digital elevation model (DEM).
pub struct MapEmbankments {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MapEmbankments {
    pub fn new() -> MapEmbankments {
        // public constructor
        let name = "MapEmbankments".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Maps road and rail embankments in a digital elevation model (DEM).".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned(), "--dem".to_owned()],
            description: "Input raster digital elevation model (DEM) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Roads Vector File".to_owned(),
            flags: vec!["--roads".to_owned()],
            description: "Input vector roads file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Railway Vector File".to_owned(),
            flags: vec!["--rail".to_owned()],
            description: "Input vector railway file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Raster Embankment File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output embankment raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Contour Interval".to_owned(),
            flags: vec!["--interval".to_owned()],
            description: "Contour interval.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Smoothing Filter Size".to_owned(),
            flags: vec!["--smooth".to_owned()],
            description: "Smoothing filter size (in num. points), e.g. 3, 5, 7, 9, 11..."
                .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("9".to_owned()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=contours.shp --interval=100.0 --base=0.0 --smooth=11 --tolerance=20.0",
            short_exe, name
        )
        .replace("*", &sep);

        MapEmbankments {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MapEmbankments {
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
        let mut roads_file = String::new();
        let mut railway_file = String::new();
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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "dem" {
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
            } else if flag_val == "-interval" {
                contour_interval = if keyval {
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
                base_contour = if keyval {
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
            } else if flag_val == "-tolerance" {
                deflection_tolerance = if keyval {
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
                if deflection_tolerance < 0f64 {
                    deflection_tolerance = 0f64;
                }
                if deflection_tolerance > 45f64 {
                    deflection_tolerance = 45f64;
                }
            } else if flag_val == "-smooth" {
                filter_size = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                if filter_size > 21 {
                    filter_size = 21;
                }
            }
        }

        let filter_radius = filter_size as isize / 2isize;
        deflection_tolerance = deflection_tolerance.to_radians().cos();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let precision = EPSILON * 10f64;

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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r").expect("Error reading input raster."));
        // let input = Raster::new(&input_file, "r").expect("Error reading input raster.");

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

        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)
            .expect("Error creating output vector.");

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 10u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "HEIGHT",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        let dx = [0, 1, 0, -1, 1, 1, -1, -1];
        let dy = [-1, 0, 1, 0, -1, 1, 1, -1];

        // Reclass the input raster
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            data[col as usize] = ((z - base_contour) / contour_interval).floor();
                        }
                    }
                    tx.send((row, data)).expect("Error sending data to thread.");
                }
            });
        }

        let mut reclassed: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            reclassed.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Reclassifying surface: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input);

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

        let edge_offsets_pt1_x = [-half_res_x, half_res_x, half_res_x, -half_res_x];
        let edge_offsets_pt1_y = [half_res_y, half_res_y, -half_res_y, -half_res_y];
        let edge_offsets_pt3_x = [half_res_x, half_res_x, -half_res_x, -half_res_x];
        let edge_offsets_pt3_y = [half_res_y, -half_res_y, -half_res_y, half_res_y];
        let (mut edge_x, mut edge_y): (f64, f64);
        let mut line_segments: Vec<LineSegment> = vec![];
        let (mut x, mut y): (f64, f64);
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
        let mut z: f64;
        let mut zn: f64;
        let (mut z1, mut z2): (isize, isize);
        let (mut p1, mut p2, mut p3): (Point2D, Point2D, Point2D);
        let mut endnode = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                z = reclassed.get_value(row, col);
                if z != nodata {
                    for n in 0..4 {
                        zn = reclassed.get_value(row + dy[n], col + dx[n]);
                        if z > zn && zn != nodata {
                            z1 = zn as isize + 1;
                            z2 = z as isize;
                            for contour_val in z1..=z2 {
                                // if n < 4 {
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

                                line_segments.push(LineSegment::new(p1, p2, contour_val as f64));
                                // }
                            }
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

        drop(reclassed);

        /*
            The structure of endnodes is as such:
            1. the starting node for polyline 'a' is a * 2.
            2. the ending node for polyline 'a' is a * 2 + 1.
            3. endnode to polyline = e / 2
            4. is an endnode a starting point? e % 2 == 0
        */

        let mut segment_live = vec![true; line_segments.len()];
        let num_nodes = line_segments.len() * 2;
        let mut line_segment_n: usize;
        let mut current_node: usize;
        let mut heading: f64;
        let mut max_heading: f64;
        let mut node_of_max_deflection: usize;
        let mut node: usize;
        let mut line_start: usize;
        let mut fid = 1;
        let mut flag: bool;
        for line_segment in 0..line_segments.len() {
            if segment_live[line_segment] {
                z = line_segments[line_segment].value;

                line_start = num_nodes;

                // check the first vertex as a potential line start
                p1 = line_segments[line_segment].first_vertex();
                current_node = line_segment * 2;

                let ret = tree
                    .within(&[p1.x, p1.y], precision, &squared_euclidean)
                    .unwrap();

                // how many points of the same line value are there?
                flag = true;
                for a in 0..ret.len() {
                    node = *ret[a].1;
                    line_segment_n = node / 2;
                    zn = line_segments[line_segment_n].value;
                    if zn == z && segment_live[line_segment_n] && node != current_node {
                        flag = false;
                        break;
                    }
                }
                if flag {
                    line_start = current_node;
                } else {
                    // try the segment endnode
                    // check the first vertex as a potential line start
                    p2 = line_segments[line_segment].last_vertex();
                    current_node = line_segment * 2 + 1;

                    let ret = tree
                        .within(&[p2.x, p2.y], precision, &squared_euclidean)
                        .unwrap();

                    // how many points of the same line value are there?
                    flag = true;
                    for a in 0..ret.len() {
                        node = *ret[a].1;
                        line_segment_n = node / 2;
                        zn = line_segments[line_segment_n].value;
                        if zn == z && segment_live[line_segment_n] && node != current_node {
                            flag = false;
                            break;
                        }
                    }
                    if flag {
                        line_start = current_node;
                    }
                }

                if line_start < num_nodes {
                    // there is only the node itself
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
                            points.push(line_segments[line_segment_n].half_point());
                        } else {
                            // We've now added both ends of this segment. Find the next connecting segment.
                            let ret = tree
                                .within(&[p1.x, p1.y], precision, &squared_euclidean)
                                .unwrap();

                            let mut connected_nodes: Vec<usize> = Vec::with_capacity(ret.len());
                            for a in 0..ret.len() {
                                node = *ret[a].1;
                                line_segment_n = node / 2;
                                zn = line_segments[line_segment_n].value;
                                if zn == z && segment_live[line_segment_n] {
                                    connected_nodes.push(node);
                                }
                            }

                            if connected_nodes.len() == 0 {
                                flag = false; // end of the line; no other connected segments
                            } else if connected_nodes.len() == 1 {
                                current_node = connected_nodes[0]; // only one connected segment; move there.
                            } else if connected_nodes.len() >= 2 {
                                // there are two or more connected segments; choose the node the represents the greatest deflection in path
                                line_segment_n = current_node / 2;
                                p1 = if current_node % 2 == 0 {
                                    line_segments[line_segment_n].last_vertex()
                                } else {
                                    line_segments[line_segment_n].first_vertex()
                                };

                                p2 = if current_node % 2 == 0 {
                                    line_segments[line_segment_n].first_vertex()
                                } else {
                                    line_segments[line_segment_n].last_vertex()
                                };

                                max_heading = 0f64;
                                node_of_max_deflection = num_nodes;
                                for n in 0..connected_nodes.len() {
                                    line_segment_n = connected_nodes[n] / 2;
                                    p3 = if connected_nodes[n] % 2 == 0 {
                                        // get the other end of this segment
                                        line_segments[line_segment_n].last_vertex()
                                    } else {
                                        line_segments[line_segment_n].first_vertex()
                                    };
                                    heading = Point2D::change_in_heading(p1, p2, p3).abs();
                                    if heading > max_heading {
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

                    if points.len() > 1 {
                        // Smooth the points
                        if points.len() > filter_size {
                            for a in 0..points.len() {
                                x = 0f64;
                                y = 0f64;
                                for p in -filter_radius..=filter_radius {
                                    let mut point_id: isize = a as isize + p;
                                    if point_id < 0 {
                                        point_id = 0;
                                    }
                                    if point_id >= points.len() as isize {
                                        point_id = points.len() as isize - 1;
                                    }
                                    x += points[point_id as usize].x;
                                    y += points[point_id as usize].y;
                                }
                                x /= filter_size as f64;
                                y /= filter_size as f64;
                                points[a].x = x;
                                points[a].y = y;
                            }

                            for a in (0..points.len()).rev() {
                                x = 0f64;
                                y = 0f64;
                                for p in -filter_radius..=filter_radius {
                                    let mut point_id: isize = a as isize + p;
                                    if point_id < 0 {
                                        point_id = 0;
                                    }
                                    if point_id >= points.len() as isize {
                                        point_id = points.len() as isize - 1;
                                    }
                                    x += points[point_id as usize].x;
                                    y += points[point_id as usize].y;
                                }
                                x /= filter_size as f64;
                                y /= filter_size as f64;
                                points[a].x = x;
                                points[a].y = y;
                            }
                        }

                        if deflection_tolerance > 0f64 {
                            for a in (1..points.len() - 1).rev() {
                                p1 = points[a - 1];
                                p2 = points[a];
                                p3 = points[a + 1];
                                // heading = Point2D::change_in_heading(p1, p2, p3).abs();
                                if path_deflection(p1, p2, p3) > deflection_tolerance {
                                    points.remove(a);
                                    // num_points_removed += 1;
                                }
                            }
                        }

                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&points);
                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(fid as i32 + 1),
                                FieldData::Real(base_contour + z * contour_interval),
                            ],
                            false,
                        );
                        fid += 1;
                    }
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * line_segment as f64 / (line_segments.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Tracing contours (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // the previous loop found all contours touching the edges of the data. This one finds all closed contours
        let mut num_line_points: usize;
        for line_segment in 0..line_segments.len() {
            if segment_live[line_segment] {
                z = line_segments[line_segment].value;

                line_start = line_segment * 2;

                // there is only the node itself
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
                        points.push(line_segments[line_segment_n].half_point());
                    } else {
                        // We've now added both ends of this segment. Find the next connecting segment.
                        let ret = tree
                            .within(&[p1.x, p1.y], precision, &squared_euclidean)
                            .unwrap();

                        let mut connected_nodes: Vec<usize> = Vec::with_capacity(ret.len());
                        for a in 0..ret.len() {
                            node = *ret[a].1;
                            line_segment_n = node / 2;
                            zn = line_segments[line_segment_n].value;
                            if zn == z && segment_live[line_segment_n] {
                                connected_nodes.push(node);
                            }
                        }

                        if connected_nodes.len() == 0 {
                            flag = false; // end of the line; no other connected segments
                        } else if connected_nodes.len() == 1 {
                            current_node = connected_nodes[0]; // only one connected segment; move there.
                        } else if connected_nodes.len() >= 2 {
                            // there are two or more connected segments; choose the node the represents the greatest deflection in path
                            line_segment_n = current_node / 2;
                            p1 = if current_node % 2 == 0 {
                                line_segments[line_segment_n].last_vertex()
                            } else {
                                line_segments[line_segment_n].first_vertex()
                            };

                            p2 = if current_node % 2 == 0 {
                                line_segments[line_segment_n].first_vertex()
                            } else {
                                line_segments[line_segment_n].last_vertex()
                            };

                            max_heading = 0f64;
                            node_of_max_deflection = num_nodes;
                            for n in 0..connected_nodes.len() {
                                line_segment_n = connected_nodes[n] / 2;
                                p3 = if connected_nodes[n] % 2 == 0 {
                                    // get the other end of this segment
                                    line_segments[line_segment_n].last_vertex()
                                } else {
                                    line_segments[line_segment_n].first_vertex()
                                };
                                heading = Point2D::change_in_heading(p1, p2, p3).abs();
                                if heading > max_heading {
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

                num_line_points = points.len();
                if num_line_points > 1 {
                    if points.len() > filter_size {
                        for a in 0..num_line_points {
                            x = 0f64;
                            y = 0f64;
                            for p in -filter_radius..=filter_radius {
                                let mut point_id: isize = a as isize + p;
                                if point_id < 0 {
                                    point_id += num_line_points as isize - 1;
                                }
                                if point_id >= num_line_points as isize {
                                    point_id -= num_line_points as isize - 1;
                                }
                                x += points[point_id as usize].x;
                                y += points[point_id as usize].y;
                            }
                            x /= filter_size as f64;
                            y /= filter_size as f64;
                            points[a].x = x;
                            points[a].y = y;
                        }

                        // set the final point position to the same as the first to close the loop
                        points[num_line_points - 1].x = points[0].x;
                        points[num_line_points - 1].y = points[0].y;

                        for a in (0..num_line_points).rev() {
                            x = 0f64;
                            y = 0f64;
                            for p in -filter_radius..=filter_radius {
                                let mut point_id: isize = a as isize + p;
                                if point_id < 0 {
                                    point_id += num_line_points as isize - 1;
                                }
                                if point_id >= num_line_points as isize {
                                    point_id -= num_line_points as isize - 1;
                                }
                                x += points[point_id as usize].x;
                                y += points[point_id as usize].y;
                            }
                            x /= filter_size as f64;
                            y /= filter_size as f64;
                            points[a].x = x;
                            points[a].y = y;
                        }

                        // set the final point position to the same as the first to close the loop
                        points[num_line_points - 1].x = points[0].x;
                        points[num_line_points - 1].y = points[0].y;
                    }

                    if deflection_tolerance > 0f64 {
                        for a in (1..points.len() - 1).rev() {
                            p1 = points[a - 1];
                            p2 = points[a];
                            p3 = points[a + 1];
                            // heading = Point2D::change_in_heading(p1, p2, p3).abs();
                            if path_deflection(p1, p2, p3) > deflection_tolerance {
                                points.remove(a);
                                // num_points_removed += 1;
                            }
                        }
                    }

                    // make sure the line is big enough to warrant writing to file.
                    let mut min_x = f64::MAX;
                    let mut max_x = f64::MIN;
                    let mut min_y = f64::MAX;
                    let mut max_y = f64::MIN;
                    for a in 0..points.len() {
                        if points[a].x < min_x {
                            min_x = points[a].x;
                        }
                        if points[a].x > max_x {
                            max_x = points[a].x;
                        }
                        if points[a].y < min_y {
                            min_y = points[a].y;
                        }
                        if points[a].y > max_y {
                            max_y = points[a].y;
                        }
                    }

                    if (max_x - min_x) > res_x || (max_y - min_y) > res_y {
                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&points);
                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(fid as i32 + 1),
                                FieldData::Real(base_contour + z * contour_interval),
                            ],
                            false,
                        );
                        fid += 1;
                    }
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * line_segment as f64 / (line_segments.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Tracing contours (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // println!("Number of points removed: {}", num_points_removed);

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
    value: f64,
}

impl LineSegment {
    fn new(p1: Point2D, p2: Point2D, value: f64) -> LineSegment {
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

    pub fn half_point(&self) -> Point2D {
        Point2D::new(
            (self.p1.x + self.p2.x) / 2f64,
            (self.p1.y + self.p2.y) / 2f64,
        )
    }
}

pub fn path_deflection(previous: Point2D, current: Point2D, next: Point2D) -> f64 {
    let p1 = current - previous;
    let p2 = next - current;
    ((p1 * p2) / (p1.magnitude() * p2.magnitude())).abs()
}
