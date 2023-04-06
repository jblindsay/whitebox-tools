/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. Timofey Samsonov, Dr. John Lindsay
Created: 25/03/2023
Last Modified: 26/03/2023
License: MIT
*/

use whitebox_raster::{Raster, RasterConfigs};
use whitebox_common::structures::{Array2D, Point2D};
use crate::tools::*;
use whitebox_vector::*;
use std::cmp;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::cmp::Ordering;
use std::collections::BTreeSet;
const EPSILON: f64 = f64::EPSILON;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to create a vector contour coverage from an input raster surface model (`--input`), such as a digital
/// elevation model (DEM). The user must specify the contour interval (`--interval`) and optionally, the base contour value (`--base`).
/// The degree to which contours are smoothed is controlled by the **Smoothing Filter Size** parameter (`--smooth`). This value, which
/// determines the size of a mean filter applied to the x-y position of vertices in each contour, should be an odd integer value, e.g.
/// 3, 5, 7, 9, 11, etc. Larger values will result in smoother contour lines. The tolerance parameter (`--tolerance`) controls the
/// amount of line generalization. That is, vertices in a contour line will be selectively removed from the line if they do not result in
/// an angular deflection in the line's path of at least this threshold value. Increasing this value can significantly decrease the size
/// of the output contour vector file, at the cost of generating straighter contour line segments.
///
/// # See Also
/// `RasterToVectorPolygons`
pub struct TopographicHachures {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TopographicHachures {
    pub fn new() -> TopographicHachures {
        // public constructor
        let name = "TopographicHachures".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Derives topographic hachures from a raster surface.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster Surface File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input surface raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Topographic Hachures File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output Topographic Hachures File.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Contour Interval".to_owned(),
            flags: vec!["--interval".to_owned()],
            description: "Contour interval.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("500.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Base Contour".to_owned(),
            flags: vec!["--base".to_owned()],
            description: "Base contour height.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
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

        parameters.push(ToolParameter {
            name: "Tolerance".to_owned(),
            flags: vec!["--tolerance".to_owned()],
            description: "Tolerance factor, in degrees (0-45); determines generalization level."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Seed separation".to_owned(),
            flags: vec!["--sep".to_owned()],
            description: "Separation distance between seed points of hachures (in cells)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.5".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum distance".to_owned(),
            flags: vec!["--distmin".to_owned()],
            description: "Minimum distance between converging flowlines (as a separation ratio)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum distance".to_owned(),
            flags: vec!["--distmax".to_owned()],
            description: "Maximum distance between diverging flowlines (as a separation ratio)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Discretization".to_owned(),
            flags: vec!["--discr".to_owned()],
            description: "Discretization step used in tracing the flowline (in cells)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum turning angle".to_owned(),
            flags: vec!["--turnmax".to_owned()],
            description: "Maximum turning angle valid for hachure, in degrees (0-90)"
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("45.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum slope angle".to_owned(),
            flags: vec!["--slopemin".to_owned()],
            description: "Slope angle, in degrees, at which flowline tracing ends"
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Nesting depth".to_owned(),
            flags: vec!["--depthmax".to_owned()],
            description: "Maximum depth of nested flowlines (0-255)"
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("16".to_owned()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=hachures.shp --interval=100.0 --base=0.0 --smooth=11 --tolerance=20.0 --distance=2.0 --discretization=0.5",
            short_exe, name
        )
        .replace("*", &sep);

        TopographicHachures {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TopographicHachures {
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
        let mut contour_interval = 10f64;
        let mut base_contour = 0f64;
        let mut deflection_tolerance = 10f64;
        let mut filter_size = 9;
        let mut separation = 5f64;
        let mut distmin = 0.5f64;
        let mut distmax = 2.0f64;
        let mut discretization = 2.0f64;
        let mut turnmax = 45.0f64;
        let mut slopemin = 1.0f64;
        let mut depth= 16u8;

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
                if filter_size % 2 == 0 && filter_size != 0 {
                    // must be an odd integer.
                    filter_size += 1;
                }
            } else if flag_val == "-sep" {
                separation = if keyval {
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
            } else if flag_val == "-distmin" {
                distmin = if keyval {
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
            } else if flag_val == "-distmax" {
                distmax = if keyval {
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
            } else if flag_val == "-discr" {
                discretization = if keyval {
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
            } else if flag_val == "-turnmax" {
                turnmax = if keyval {
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
            } else if flag_val == "-slopemin" {
                slopemin = if keyval {
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
            } else if flag_val == "-depthmax" {
                depth = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<u8>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<u8>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        let filter_radius = filter_size as isize / 2isize;
        deflection_tolerance = deflection_tolerance.to_radians().cos();
        turnmax = turnmax.to_radians().cos();
        slopemin = slopemin.to_radians().tan();
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
        let cov = RasterCoverage::new(&input);
        // let input = Raster::new(&input_file, "r").expect("Error reading input raster.");

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let res_x = input.configs.resolution_x;
        let res_y = input.configs.resolution_y;
        let res_xy = 0.5f64 * (res_x + res_y);
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
        output.attributes.add_field(&AttributeField::new(
            "FID",
            FieldDataType::Int,
            10u8,
            0u8
        ));

        output.attributes.add_field(&AttributeField::new(
            "HEIGHT",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "SLOPE",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "ASPECT",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "N",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "NE",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "E",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "SE",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "S",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "SW",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "W",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "NW",
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
        let mut flag: bool;

        let mut contours = Vec::new();

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
                        if points.len() > filter_size && filter_size > 0 {
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

                        contours.push(
                            Contour {
                                points: points,
                                value: base_contour + z * contour_interval,
                                closed: false
                            }
                        );

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
                    if points.len() > filter_size && filter_size > 0 {
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
                        contours.push(
                            Contour {
                                points: points,
                                value: base_contour + z * contour_interval,
                                closed: true
                            }
                        );
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

        contours.sort();
        // contours.reverse();

        let mut counter = 0;
        let mut hid = 1;
        let ncont = contours.len();

        let mut flowlines_prev: Vec<Vec<Point2D>> = Vec::new();
        let mut flowlines: Vec<Vec<Point2D>> = Vec::new();
        let mut starts = BTreeSet::new();
        let mut seed_starts = BTreeSet::new();
        seed_starts.insert(0);

        let mut level_seeds = Vec::new();

        let mut finished_level: bool;

        for contour in &contours {

            let points = &contour.points;
            let npts = points.len();
            let mut perim: f64 = 0.0;
            let mut accdist = vec![0.0; npts];

            for i in 1..npts {
                perim += points[i-1].distance(&points[i]);
                accdist[i] = perim;
            }

            let step = separation * res_xy;
            let num = perim / step;
            let to_up = (num.ceil() - num) < (num - num.floor());
            let new_step = if to_up { perim / num.ceil() } else { perim / num.floor() };
            let num_seeds= (perim / new_step) as i32;

            let discr = discretization * res_xy;
            let val = contour.value;
            let zmin = val - contour_interval;
            let zmax = val + contour_interval;

            let new_distmin = distmin * new_step;
            let new_distmax = distmax * new_step;

            let mut seeds = Vec::new();

            seeds.push(points[0]);

            let mut dist;
            let mut j = 0;

            for i in 1..num_seeds {
                dist = i as f64 * new_step;
                while dist > accdist[j] { j+=1; }
                let t = (dist - accdist[j-1]) / (accdist[j] - accdist[j-1]);

                let seed = Point2D::new(
                    (1.-t) * points[j-1].x + t * points[j].x,
                    (1.-t) * points[j-1].y + t * points[j].y
                );
                seeds.push(seed);
                level_seeds.push(seed);
            }

            seeds.push(points[npts-1]);
            level_seeds.push(points[npts-1]);

            starts.insert(flowlines.len());
            seed_starts.insert(level_seeds.len());

            for seed in &seeds {
                let mut flowline = get_flowline(
                    &cov, &seed, discr,
                    zmin, slopemin, turnmax, true
                );
                if flowline.len() > 1 {
                    let idx = intersection_idx(&flowline, &flowlines,
                                                   new_distmin);
                    flowline.truncate(idx);

                    if flowline.len() > 1 {
                        flowlines.push(flowline);
                    }
                }
            }

            finished_level = false;
            if counter == ncont-1 {
                finished_level = true;
            } else {
                if contours[counter+1].value != val {
                    finished_level = true;
                }
            }

            if finished_level {

                let mut n = flowlines.len();

                if n > 1 {
                    for i in 0..n-1 {
                        if !starts.contains(&(i+1)) {
                            insert_flowlines(&cov, &mut flowlines, i, i+1, 0, 0,
                                             depth, new_distmin, new_distmax,
                                             discr, zmin, slopemin, turnmax, true);
                        }
                    }
                }

                let mut flowlines_up: Vec<Vec<Point2D>> = Vec::new();
                let mut idxs: Vec<usize> = Vec::new();
                let mut i: usize = 0;

                for seed in &level_seeds {

                    let mut flowline = get_flowline(
                        &cov, &seed, discr,
                        zmax, slopemin, turnmax, false
                    );

                    if flowline.len() > 1 {
                        let idx1 = intersection_idx(&flowline, &flowlines_prev,
                                                        step);

                        let idx2 = intersection_idx(&flowline, &flowlines_up,
                                                       new_distmin);

                        let idx = cmp::min(idx1, idx2);

                        flowline.truncate(idx);

                        if flowline.len() > 1 {
                            flowlines_up.push(flowline);
                            idxs.push(i);
                        }
                    }
                    i += 1;

                }

                n = flowlines_up.len();

                if n > 1 {
                    for i in 0..n-1 {
                        if (!seed_starts.contains(&idxs[i+1])) && (idxs[i+1]-idxs[i] == 1) {
                            insert_flowlines(&cov, &mut flowlines_up, i, i+1, 0, 0,
                                             depth, new_distmin, new_distmax,
                                             discr, zmax, slopemin, turnmax, false);
                        }
                    }
                }

                level_seeds = Vec::new();
                flowlines_prev = flowlines.clone();
                flowlines.append(&mut flowlines_up);

                let mut dxsum: f64;
                let mut dysum: f64;
                let mut grad: [f64; 2];
                let mut grad_len: f64;
                let mut dx: f64;
                let mut dy: f64;
                let mut dx1: f64;
                let mut dy1: f64;
                let mut slope: f64;
                let mut math_aspect: f64;
                let mut aspect: f64;

                let mut cos_n: f64;
                let mut cos_ne: f64;
                let mut cos_e: f64;
                let mut cos_se: f64;
                let mut cos_s: f64;
                let mut cos_sw: f64;
                let mut cos_w: f64;
                let mut cos_nw: f64;

                let sqrt_05 = 0.5_f64.sqrt();

                for flowline in &flowlines {

                    dxsum = 0.0;
                    dysum = 0.0;

                    for point in flowline {
                        grad = cov.get_gradient(point.x, point.y);
                        dxsum += grad[0];
                        dysum += grad[1];
                    }

                    dx = -dxsum / flowline.len() as f64;
                    dy = -dysum / flowline.len() as f64;
                    grad_len = (dx*dx + dy*dy).sqrt();

                    slope = grad_len.atan().to_degrees();
                    math_aspect = dy.atan2(dx).to_degrees();
                    aspect = if math_aspect < 90.0 { 90.0 - math_aspect } else { 450.0 - math_aspect };

                    dx1 = dx / grad_len;
                    dy1 = dy / grad_len;

                    cos_n =       0.0 * dx1 +     1.0 * dy1;
                    cos_ne =  sqrt_05 * dx1 + sqrt_05 * dy1;
                    cos_e =       1.0 * dx1 +     0.0 * dy1;
                    cos_se =  sqrt_05 * dx1 - sqrt_05 * dy1;
                    cos_s =       0.0 * dx1 -     1.0 * dy1;
                    cos_sw = -sqrt_05 * dx1 - sqrt_05 * dy1;
                    cos_w =      -1.0 * dx1 +     0.0 * dy1;
                    cos_nw = -sqrt_05 * dx1 + sqrt_05 * dy1;

                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&flowline);
                    output.add_record(sfg);
                    output.attributes.add_record(
                        vec![
                            FieldData::Int(hid as i32),
                            FieldData::Real(val),
                            FieldData::Real(slope),
                            FieldData::Real(aspect),
                            FieldData::Real(cos_n),
                            FieldData::Real(cos_ne),
                            FieldData::Real(cos_e),
                            FieldData::Real(cos_se),
                            FieldData::Real(cos_s),
                            FieldData::Real(cos_sw),
                            FieldData::Real(cos_w),
                            FieldData::Real(cos_nw)
                        ],
                        false,
                    );
                    hid += 1;
                }

                flowlines.clear();
                starts.clear();
                seed_starts.clear();
                seed_starts.insert(0);
            }

            counter += 1;

            if verbose {
                progress =
                    (100.0_f64 * counter as f64 / (ncont- 1) as f64) as usize;
                if progress != old_progress {
                    println!("Tracing hachures: {}%", progress);
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

pub fn path_turn(previous: Point2D, current: Point2D, next: Point2D) -> f64 {
    let p1 = current - previous;
    let p2 = next - current;
    (p1 * p2) / (p1.magnitude() * p2.magnitude())
}

pub struct Contour {
    points: Vec<Point2D>,
    value: f64,
    closed: bool
}

impl PartialOrd for Contour {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Contour {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.value > other.value {
            Ordering::Less
        }
        else if self.value > other.value {
            Ordering::Greater
        }
        else {
            Ordering::Equal
        }
    }
}


impl PartialEq for Contour {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.closed == other.closed
    }
}

impl Eq for Contour {}

#[derive(Default, Clone)]
pub struct RasterCoverage {
    pub configs: RasterConfigs,

    // bilinear interpolation coefficients
    a00: Vec<f64>,
    a10: Vec<f64>,
    a01: Vec<f64>,
    a11: Vec<f64>
}

impl RasterCoverage {
    pub fn new<'a>(raster: &'a Raster) -> RasterCoverage {

        let rows = raster.configs.rows as isize;
        let columns = raster.configs.columns as isize;
        let npixels = (rows * columns) as usize;

        let mut output = RasterCoverage {
            configs: raster.configs.clone(),
            a00: vec![0f64; npixels],
            a10: vec![0f64; npixels],
            a01: vec![0f64; npixels],
            a11: vec![0f64; npixels]
        };

        for row in 0..rows {
            for col in 0..columns {
                let z00 = raster.get_value(row + 1, col);
                let z10 = raster.get_value(row + 1, col + 1);
                let z01 = raster.get_value(row, col);
                let z11 = raster.get_value(row, col + 1);

                let idx= (row * columns + col) as usize;

                output.a00[idx] = z00;
                output.a10[idx] = z10 - z00;
                output.a01[idx] = z01 - z00;
                output.a11[idx] = z00 + z11 - z01 - z10;
            }
        }

        output

    }

    pub fn get_column_from_x(&self, x: f64) -> isize {
        ((x - self.configs.west - 0.5*self.configs.resolution_x) / self.configs.resolution_x).floor() as isize
    }

    pub fn get_row_from_y(&self, y: f64) -> isize {
        ((self.configs.north - y - 0.5*self.configs.resolution_y) / self.configs.resolution_y).floor() as isize
    }

    pub fn get_x_from_column(&self, column: isize) -> f64 {
        // self.configs.west - self.configs.resolution_x / 2f64 +
        // column as f64 * self.configs.resolution_x
        // Not sure why it must be + 1/2 resolution rather than minus
        self.configs.west
            + self.configs.resolution_x / 2f64
            + column as f64 * self.configs.resolution_x
    }

    pub fn get_y_from_row(&self, row: isize) -> f64 {
        self.configs.north
            - self.configs.resolution_y / 2f64
            - row as f64 * self.configs.resolution_y
    }

    pub fn get_cell_coords(&self, x: f64, y: f64) -> (usize, f64, f64) {
        let row = self.get_row_from_y(y);
        let col = self.get_column_from_x(x);

        if  row < 0 || col < 0 ||
            row as usize >= self.configs.rows-1 ||
            col as usize >= self.configs.columns-1 {
            return (usize::MAX, -1f64, -1f64)
        } else {
            let xcol = self.get_x_from_column(col);
            let yrow = self.get_y_from_row(row);

            let idx= (row * self.configs.columns as isize + col) as usize;

            let xcell = (x - xcol) / self.configs.resolution_x;
            let ycell = 1.0 - (yrow - y) / self.configs.resolution_y;

            (idx, xcell, ycell)
        }
    }

    pub fn get_value(&self, x: f64, y: f64) -> f64 {
        let (idx, xcell, ycell) = self.get_cell_coords(x, y);

        if idx == usize::MAX {
            self.configs.nodata
        } else {
            self.a00[idx] + self.a10[idx] * xcell +
                self.a01[idx] * ycell + self.a11[idx] * xcell * ycell
        }
    }

    pub fn get_gradient(&self, x: f64, y: f64) -> [f64; 2] {
        let (idx, xcell, ycell) = self.get_cell_coords(x, y);

        [
            (self.a10[idx] + self.a11[idx] * ycell) / self.configs.resolution_x,
            (self.a01[idx] + self.a11[idx] * xcell) / self.configs.resolution_y
        ]
    }

    pub fn get_slope(&self, x: f64, y: f64) -> f64 {
        let grad = self.get_gradient(x, y);
        (grad[0]*grad[0] + grad[1]*grad[1]).sqrt()
    }

    // pub fn get_slope_rad(&self, x: f64, y: f64) -> f64 {
    //     self.get_slope(x, y).atan()
    // }
    //
    // pub fn get_slope_deg(&self, x: f64, y: f64) -> f64 {
    //     self.get_slope(x, y).atan().to_degrees()
    // }
}

/// Traces the flowline from `p` using `discr` step until:
/// - elevation is smaller than `zmin` or
/// - slope is smaller than `slopemin` or
/// - deflection is larger than `defmax`
pub fn get_flowline(cov: &RasterCoverage, p: &Point2D,
                    discr: f64, zlim: f64, slopemin: f64, defmin: f64, down: bool) -> Vec<Point2D> {
    let mut points = vec![];
    let mut zcur: f64;
    let mut zprev: f64;
    let mut slope: f64;
    let mut grad: [f64; 2];
    let mut grad2: [f64; 2];

    let sign = if down { 1.0 } else { -1.0 };

    let mut p1: Point2D = p.clone();
    let mut p2: Point2D;

    zprev = cov.get_value(p1.x, p1.y);

    if zprev == zlim || zprev == cov.configs.nodata {
        return points;
    }

    points.push(p1);

    loop {
        slope = cov.get_slope(p1.x, p1.y);

        if slope < slopemin { break; }

        grad = cov.get_gradient(p1.x, p1.y);

        p2 = Point2D::new(
          p1.x - sign * discr * grad[0] / slope,
          p1.y - sign * discr * grad[1] / slope,
        );

        zcur = cov.get_value(p2.x, p2.y);

        if zcur == cov.configs.nodata {
            break;
        } else {
            grad2 = cov.get_gradient(p2.x, p2.y);
            grad[0] = 0.5 * (grad[0] + grad2[0]);
            grad[1] = 0.5 * (grad[1] + grad2[1]);

            p2 = Point2D::new(
                p1.x - sign * discr * grad[0] / (grad[0]*grad[0] + grad[1]*grad[1]).sqrt(),
                p1.y - sign * discr * grad[1] / (grad[0]*grad[0] + grad[1]*grad[1]).sqrt(),
            );

            zcur = cov.get_value(p2.x, p2.y);
        }

        if (down && (zcur < zlim)) || (!down && (zcur > zlim)) {
            let t = (zprev - zlim) / (zprev  - zcur);
            let pend = Point2D::new(
                (1.0 - t)*p1.x + t*p2.x,
                (1.0 - t)*p1.y + t*p2.y
            );
            points.push(pend);
            break;
        } else if (down && (zcur < zprev)) || (!down && (zcur > zprev))  {
            points.push(p2);
            p1 = p2;
            zprev = zcur;
        } else {
            break;
        }

        let n = points.len();
        if n >= 3 {
            if path_turn(points[n-3], points[n-2], points[n-1]) < defmin {
                points.pop();
                break
            }
        }
    }

    points
}

pub fn insert_flowlines(cov: &RasterCoverage, flowlines: &mut Vec<Vec<Point2D>>,
                        n1: usize, n2: usize, k1: usize, k2:usize, depth: u8, distmin: f64,
                        distmax: f64, discr: f64, zlim: f64, slopemin: f64, defmin: f64, down: bool) {
    if depth == 0 { return }

    let mut p1: Point2D;
    let mut p2: Point2D;
    let p3: Point2D;
    let mut dist: f64;
    let mut flowline: Vec<Point2D>;
    let idx: usize;
    let nlast: usize;

    let n = cmp::min(flowlines[n1].len()-k1, flowlines[n2].len()-k2);

    for i in 0..n {
        p1 = flowlines[n1][i+k1];
        p2 = flowlines[n2][i+k2];
        dist = p1.distance(&p2);

        if dist >= distmax {
            p3 = Point2D::midpoint(&p1, &p2);
            flowline = get_flowline(cov, &p3, discr, zlim, slopemin, defmin, down);

            if flowline.len() > 1 {
                idx = intersection_idx(&flowline, flowlines,distmin);
                flowline.truncate(idx);

                if flowline.len() > 1 {
                    flowlines.push(flowline);
                    nlast = flowlines.len()-1;
                    insert_flowlines(cov, flowlines, n1, nlast, i+k1, 0,
                                     depth-1, distmin, distmax, discr,
                                     zlim, slopemin, defmin, down);
                    insert_flowlines(cov, flowlines, n2, nlast, i+k2, 0,
                                     depth-1, distmin, distmax, discr,
                                     zlim, slopemin, defmin, down);
                }
            }

            return
        }
    }
}


pub fn intersection_idx(newline: &Vec<Point2D>, lines: &Vec<Vec<Point2D>>, dist: f64) -> usize {

    let mut imin = newline.len();
    for line in lines.iter().rev() {
        let d1 = newline[0].distance(&newline[newline.len()-1]);
        let d2 = line[0].distance(&line[line.len()-1]);

        let c1 = Point2D::midpoint(&newline[0], &newline[newline.len()-1]);
        let c2 = Point2D::midpoint(&line[0], &line[line.len()-1]);

        let d3 = c1.distance(&c2);

        if d3 < (d1 + d2)/2.0 {
            for i in 1..newline.len() {
                for j in 1..line.len() {
                    if newline[i].distance(&line[j]) < dist {
                        imin = if i < imin { i } else { imin };
                        if imin == 1 { return imin }
                    }
                    if is_intersection(&newline[i-1], &newline[i], &line[j-1], &line[j]) {
                        imin = if i < imin { i } else { imin };
                        if imin == 1 { return imin }
                    }
                }
            }
        }
    }
    imin
}

pub fn point_side(p1: &Point2D, p2: &Point2D, p3: &Point2D) -> bool {
    (p3.x - p1.x)*(p2.y - p1.y) < (p3.y - p1.y)*(p2.x - p1.x)
}

pub fn is_intersection(p1: &Point2D, p2: &Point2D, p3: &Point2D, p4: &Point2D) -> bool {
    (point_side(p1, p2, p3) != point_side(p1, p2, p4)) &&
    (point_side(p3, p4, p1) != point_side(p3, p4, p2))
}