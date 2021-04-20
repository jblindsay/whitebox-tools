/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/04/2020
Last Modified: 26/04/2020
License: MIT
*/

use whitebox_common::algorithms::triangulate;
use whitebox_common::structures::Point2D;
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
const EPSILON: f64 = std::f64::EPSILON;

/// This tool creates a contour coverage from a set of input points (`--input`). The user must specify the contour
/// interval (`--interval`) and optionally, the base contour value (`--base`). The degree to which contours are
/// smoothed is controlled by the **Smoothing Filter Size** parameter (`--smooth`). This value, which determines
/// the size of a mean filter applied to the x-y position of vertices in each contour, should be an odd integer value, e.g.
/// 3, 5, 7, 9, 11, etc. Larger values will result in smoother contour lines.
///
/// # See Also
/// `ContoursFromRaster`
pub struct ContoursFromPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ContoursFromPoints {
    pub fn new() -> ContoursFromPoints {
        // public constructor
        let name = "ContoursFromPoints".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Creates a contour coverage from a set of input points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
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
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use Shapefile 'z' values?".to_owned(),
            flags: vec!["--use_z".to_owned()],
            description:
                "Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?"
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Vector Lines File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector lines file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Triangle Edge Length (optional)".to_owned(),
            flags: vec!["--max_triangle_edge_length".to_owned()],
            description: "Optional maximum triangle edge length; triangles larger than this size will not be gridded.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
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
            default_value: Some("5".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=HEIGHT -o=contours.shp --max_triangle_edge_length=100.0 --interval=100.0 --base=0.0 --smooth=11",
            short_exe, name
        )
        .replace("*", &sep);

        ContoursFromPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ContoursFromPoints {
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
        let mut input_file: String = "".to_string();
        let mut field_name = String::new();
        let mut use_z = false;
        let mut use_field = false;
        let mut output_file: String = "".to_string();
        let mut max_triangle_edge_length = f64::INFINITY;
        let mut contour_interval = 10f64;
        let mut base_contour = 0f64;
        let mut filter_size = 5;

        // read the arguments
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
                use_field = true;
            } else if flag_val.contains("use_z") {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_z = true;
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-max_triangle_edge_length" {
                max_triangle_edge_length = if keyval {
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

                max_triangle_edge_length *= max_triangle_edge_length; // actually squared distance
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
                if filter_size > 0 && filter_size % 2 == 0 {
                    // it must be odd.
                    filter_size += 1;
                }
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let precision = EPSILON * 10f64;
        let filter_radius = filter_size as isize / 2isize;

        let start = Instant::now();

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point
            && input.header.shape_type.base_shape_type() != ShapeType::MultiPoint
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POINT base shape type.",
            ));
        }

        if use_z && input.header.shape_type.dimension() != ShapeTypeDimension::Z {
            return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input vector data must be of 'POINTZ' or 'MULTIPOINTZ' ShapeType to use the --use_z flag.",
                ));
        } else if use_field {
            // What is the index of the field to be analyzed?
            let field_index = match input.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The specified field name does not exist in input shapefile.",
                    ))
                }
            };

            // Is the field numeric?
            if !input.attributes.is_field_numeric(field_index) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified attribute field is non-numeric.",
                ));
            }
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;

        // set the projection information
        output.projection = input.projection.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));

        output.attributes.add_field(&AttributeField::new(
            &field_name,
            FieldDataType::Real,
            10u8,
            4u8,
        ));

        let mut points: Vec<Point2D> = vec![];
        let mut z_values: Vec<f64> = vec![];

        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
                if use_z {
                    z_values.push(record.z_array[i]);
                } else if use_field {
                    match input.attributes.get_value(record_num, &field_name) {
                        FieldData::Int(val) => {
                            z_values.push(val as f64);
                        }
                        FieldData::Real(val) => {
                            z_values.push(val);
                        }
                        _ => {
                            // likely a null field
                            z_values.push(0f64);
                        }
                    }
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Reading points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input);

        if points.len() <= 3 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "There are too few input points.",
            ));
        }
        if verbose {
            println!("Performing triangulation...");
        }
        // this is where the heavy-lifting is
        let result = triangulate(&points).expect("No triangulation exists.");

        let (mut p1, mut p2, mut p3): (usize, usize, usize);
        let (mut min_val, mut max_val): (f64, f64);
        let (mut lower_interval, mut upper_interval): (usize, usize);
        let mut contour_z: f64;
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
        let mut node = 0usize;
        let mut contour_points: Vec<(Point2D, f64)> = vec![];
        let (mut x, mut y, mut fraction): (f64, f64, f64);
        let mut fid = 1;
        let (mut pt1, mut pt2, mut pt3): (Point2D, Point2D, Point2D);
        let mut num_intersections: usize;
        let (mut intersect1, mut intersect2, mut intersect3): (bool, bool, bool);
        for i in (0..result.triangles.len()).step_by(3) {
            p1 = result.triangles[i + 2];
            p2 = result.triangles[i + 1];
            p3 = result.triangles[i];

            if max_distance_squared(
                points[p1],
                points[p2],
                points[p3],
                z_values[p1],
                z_values[p2],
                z_values[p3],
            ) < max_triangle_edge_length
            {
                min_val = z_values[p1].min(z_values[p2].min(z_values[p3]));
                max_val = z_values[p1].max(z_values[p2].max(z_values[p3]));
                lower_interval = ((min_val - base_contour) / contour_interval).ceil() as usize;
                upper_interval = ((max_val - base_contour) / contour_interval).floor() as usize;
                for a in lower_interval..=upper_interval {
                    contour_z = base_contour + a as f64 * contour_interval;

                    pt1 = Point2D::new(0f64, 0f64);
                    pt2 = Point2D::new(0f64, 0f64);
                    pt3 = Point2D::new(0f64, 0f64);
                    num_intersections = 0;
                    intersect1 = false;
                    intersect2 = false;
                    intersect3 = false;
                    if contour_z >= z_values[p1].min(z_values[p2])
                        && contour_z <= z_values[p1].max(z_values[p2])
                    {
                        num_intersections += 1;
                        intersect1 = true;
                        fraction = if z_values[p1] != z_values[p2] {
                            (contour_z - z_values[p1]) / (z_values[p2] - z_values[p1])
                        } else {
                            0f64
                        };
                        x = points[p1].x + fraction * (points[p2].x - points[p1].x);
                        y = points[p1].y + fraction * (points[p2].y - points[p1].y);
                        pt1 = Point2D::new(x, y);
                    }
                    if contour_z >= z_values[p2].min(z_values[p3])
                        && contour_z <= z_values[p2].max(z_values[p3])
                    {
                        num_intersections += 1;
                        intersect2 = true;
                        fraction = if z_values[p2] != z_values[p3] {
                            (contour_z - z_values[p2]) / (z_values[p3] - z_values[p2])
                        } else {
                            0f64
                        };
                        x = points[p2].x + fraction * (points[p3].x - points[p2].x);
                        y = points[p2].y + fraction * (points[p3].y - points[p2].y);
                        pt2 = Point2D::new(x, y);
                    }
                    if contour_z >= z_values[p1].min(z_values[p3])
                        && contour_z <= z_values[p1].max(z_values[p3])
                    {
                        num_intersections += 1;
                        intersect3 = true;
                        fraction = if z_values[p1] != z_values[p3] {
                            (contour_z - z_values[p1]) / (z_values[p3] - z_values[p1])
                        } else {
                            0f64
                        };
                        x = points[p1].x + fraction * (points[p3].x - points[p1].x);
                        y = points[p1].y + fraction * (points[p3].y - points[p1].y);
                        pt3 = Point2D::new(x, y);
                    }

                    if num_intersections == 3 {
                        // The contour intersects one of the vertices and two of these three points are the same.
                        // Remove one of the two identical points.
                        if pt1.distance(&pt2) < precision {
                            intersect2 = false;
                            num_intersections -= 1;
                        }
                        if pt1.distance(&pt3) < precision {
                            intersect3 = false;
                            num_intersections -= 1;
                        }
                        if pt2.distance(&pt3) < precision {
                            intersect3 = false;
                            num_intersections -= 1;
                        }
                    }

                    if num_intersections != 2 && verbose {
                        println!("Warning: An error occurred during the contouring operation.");
                    }

                    if intersect2 && intersect3 {
                        pt1 = pt2;
                        pt2 = pt3;
                    } else if intersect1 && intersect3 {
                        pt2 = pt3;
                    }

                    // The contour may only intersect a triangle at one of the triangle's vertices.
                    // We don't want to record this segment.
                    if pt1.distance(&pt2) > precision {
                        contour_points.push((pt1, contour_z));
                        tree.add([pt1.x, pt1.y], node).unwrap();
                        node += 1;

                        contour_points.push((pt2, contour_z));
                        tree.add([pt2.x, pt2.y], node).unwrap();
                        node += 1;
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * i as f64 / (result.triangles.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let num_points = contour_points.len();
        let mut unvisited = vec![true; num_points];
        let mut num_neighbours: usize;
        let mut flag: bool;
        let mut found_node: bool;
        let mut other_node: usize;
        for i in 0..num_points {
            if unvisited[i] {
                contour_z = contour_points[i].1;

                // is it an endnode?
                let ret = tree
                    .within(
                        &[contour_points[i].0.x, contour_points[i].0.y],
                        precision,
                        &squared_euclidean,
                    )
                    .unwrap();

                num_neighbours = 0;
                for a in 0..ret.len() {
                    node = *ret[a].1;
                    if contour_points[node].1 == contour_z {
                        num_neighbours += 1;
                    }
                }
                if num_neighbours == 1 {
                    let mut line_points = vec![];
                    node = i;
                    line_points.push(contour_points[node].0);
                    unvisited[node] = false;
                    flag = true;
                    while flag {
                        // get the other side of this line segment
                        other_node = if node % 2 == 0 { node + 1 } else { node - 1 };
                        if unvisited[other_node] {
                            if filter_size > 0 {
                                // Add a mid-point
                                x = (contour_points[node].0.x + contour_points[other_node].0.x)
                                    / 2f64;
                                y = (contour_points[node].0.y + contour_points[other_node].0.y)
                                    / 2f64;
                                line_points.push(Point2D::new(x, y));
                            }
                            node = other_node;
                            line_points.push(contour_points[node].0);
                            unvisited[node] = false;
                        } else {
                            found_node = false;
                            let ret = tree
                                .within(
                                    &[contour_points[node].0.x, contour_points[node].0.y],
                                    precision,
                                    &squared_euclidean,
                                )
                                .unwrap();
                            for a in 0..ret.len() {
                                other_node = *ret[a].1;
                                if other_node != node
                                    && contour_points[other_node].1 == contour_z
                                    && unvisited[other_node]
                                {
                                    node = other_node;
                                    line_points.push(contour_points[node].0);
                                    unvisited[node] = false;
                                    found_node = true;
                                    break;
                                }
                            }

                            if !found_node {
                                // we've located the other end of the line.
                                flag = false;
                            }
                        }
                    }

                    // remove the duplicate points
                    for a in (1..line_points.len()).rev() {
                        if line_points[a] == line_points[a - 1] {
                            line_points.remove(a);
                        }
                    }

                    if line_points.len() > 1 {
                        // Smooth the points
                        if line_points.len() > filter_size && filter_size > 0 {
                            for a in 0..line_points.len() {
                                x = 0f64;
                                y = 0f64;
                                for p in -filter_radius..=filter_radius {
                                    let mut point_id: isize = a as isize + p;
                                    if point_id < 0 {
                                        point_id = 0;
                                    }
                                    if point_id >= line_points.len() as isize {
                                        point_id = line_points.len() as isize - 1;
                                    }
                                    x += line_points[point_id as usize].x;
                                    y += line_points[point_id as usize].y;
                                }
                                x /= filter_size as f64;
                                y /= filter_size as f64;
                                line_points[a].x = x;
                                line_points[a].y = y;
                            }

                            for a in (0..line_points.len()).rev() {
                                x = 0f64;
                                y = 0f64;
                                for p in -filter_radius..=filter_radius {
                                    let mut point_id: isize = a as isize + p;
                                    if point_id < 0 {
                                        point_id = 0;
                                    }
                                    if point_id >= line_points.len() as isize {
                                        point_id = line_points.len() as isize - 1;
                                    }
                                    x += line_points[point_id as usize].x;
                                    y += line_points[point_id as usize].y;
                                }
                                x /= filter_size as f64;
                                y /= filter_size as f64;
                                line_points[a].x = x;
                                line_points[a].y = y;
                            }
                        }

                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&line_points);
                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![FieldData::Int(fid as i32 + 1), FieldData::Real(contour_z)],
                            false,
                        );
                        fid += 1;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / (num_points - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Closed contours
        let mut num_line_points: usize;
        for i in 0..num_points {
            if unvisited[i] {
                contour_z = contour_points[i].1;
                let mut line_points = vec![];
                node = i;
                line_points.push(contour_points[node].0);
                unvisited[node] = false;
                flag = true;
                while flag {
                    // get the other side of this line segment
                    other_node = if node % 2 == 0 { node + 1 } else { node - 1 };
                    if unvisited[other_node] {
                        if filter_size > 0 {
                            // Add a mid-point
                            x = (contour_points[node].0.x + contour_points[other_node].0.x) / 2f64;
                            y = (contour_points[node].0.y + contour_points[other_node].0.y) / 2f64;
                            line_points.push(Point2D::new(x, y));
                        }
                        node = other_node;
                        line_points.push(contour_points[node].0);
                        unvisited[node] = false;
                    } else {
                        found_node = false;
                        let ret = tree
                            .within(
                                &[contour_points[node].0.x, contour_points[node].0.y],
                                precision,
                                &squared_euclidean,
                            )
                            .unwrap();
                        for a in 0..ret.len() {
                            other_node = *ret[a].1;
                            if other_node != node
                                && contour_points[other_node].1 == contour_z
                                && unvisited[other_node]
                            {
                                node = other_node;
                                line_points.push(contour_points[node].0);
                                unvisited[node] = false;
                                found_node = true;
                            }
                        }
                        if !found_node {
                            // we've located the other end of the line.
                            flag = false;
                        }
                    }
                }

                // remove the duplicate points
                for a in (1..line_points.len()).rev() {
                    if line_points[a] == line_points[a - 1] {
                        line_points.remove(a);
                    }
                }

                num_line_points = line_points.len();
                if num_line_points > 1 {
                    if num_line_points > filter_size && filter_size > 0 {
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
                                x += line_points[point_id as usize].x;
                                y += line_points[point_id as usize].y;
                            }
                            x /= filter_size as f64;
                            y /= filter_size as f64;
                            line_points[a].x = x;
                            line_points[a].y = y;
                        }

                        // set the final point position to the same as the first to close the loop
                        line_points[num_line_points - 1].x = line_points[0].x;
                        line_points[num_line_points - 1].y = line_points[0].y;

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
                                x += line_points[point_id as usize].x;
                                y += line_points[point_id as usize].y;
                            }
                            x /= filter_size as f64;
                            y /= filter_size as f64;
                            line_points[a].x = x;
                            line_points[a].y = y;
                        }

                        // set the final point position to the same as the first to close the loop
                        line_points[num_line_points - 1].x = line_points[0].x;
                        line_points[num_line_points - 1].y = line_points[0].y;
                    }

                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&line_points);
                    output.add_record(sfg);
                    output.attributes.add_record(
                        vec![FieldData::Int(fid as i32 + 1), FieldData::Real(contour_z)],
                        false,
                    );
                    fid += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / (num_points - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

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

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}

/// Calculate squared Euclidean distance between the point and another.
pub fn max_distance_squared(
    p1: Point2D,
    p2: Point2D,
    p3: Point2D,
    z1: f64,
    z2: f64,
    z3: f64,
) -> f64 {
    let mut dx = p1.x - p2.x;
    let mut dy = p1.y - p2.y;
    let mut dz = z1 - z2;
    let mut max_dist = dx * dx + dy * dy + dz * dz;

    dx = p1.x - p3.x;
    dy = p1.y - p3.y;
    dz = z1 - z3;
    let mut dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    dx = p2.x - p3.x;
    dy = p2.y - p3.y;
    dz = z2 - z3;
    dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    max_dist
}

// pub fn path_deflection(previous: Point2D, current: Point2D, next: Point2D) -> f64 {
//     let p1 = current - previous;
//     let p2 = next - current;
//     ((p1 * p2) / (p1.magnitude() * p2.magnitude())).abs()
// }
