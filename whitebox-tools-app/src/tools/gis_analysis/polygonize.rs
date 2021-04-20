/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/10/2018
Last Modified: 28/10/2018
License: MIT
*/
extern crate kdtree;

use whitebox_common::algorithms::{
    find_split_points_at_line_intersections, is_clockwise_order, poly_in_poly,
};
use whitebox_common::structures::{BoundingBox, Polyline, Point2D};
use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON; //1.0e-7f64;

/// This tool outputs a vector polygon layer from two or more intersecting line features
/// contained in one or more input vector line files. Each space enclosed by the intersecting
/// line set is converted to polygon added to the output layer. This tool should not be
/// confused with the `LinesToPolygons` tool, which can be used to convert a vector file of
/// polylines into a set of polygons, simply by closing each line feature. The `LinesToPolygons`
/// tool does not deal with line intersection in the same way that the `Polygonize` tool does.
///
/// # See Also
/// `LinesToPolygons`
pub struct Polygonize {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Polygonize {
    pub fn new() -> Polygonize {
        // public constructor
        let name = "Polygonize".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Creates a polygon layer from two or more intersecting line features contained in one or more input vector line files."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input vector polyline file.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='lines1.shp;lines2.shp;lines3.shp' -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        Polygonize {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Polygonize {
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
        let mut input_files = String::new();
        let mut output_file: String = "".to_string();

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
            if flag_val == "-i" || flag_val.contains("-input") {
                input_files = if keyval {
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

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
        if num_files < 1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "At least one input file is required to operate this tool.",
            ));
        }

        let mut in_polylines: Vec<Polyline> = vec![];
        let mut bb: Vec<BoundingBox> = vec![];

        if verbose {
            println!("Reading data...")
        };
        let mut projection = String::new();
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                let input = Shapefile::read(&input_file)?;
                projection = input.projection.clone();

                // make sure the input vector file is of polyline type
                if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The input vector data must be of POLYLINE base shape type.",
                    ));
                }

                // Get the polylines and bounding boxes of each of the features in input1 and input 2
                let mut first_point_in_part: usize;
                let mut last_point_in_part: usize;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    for part in 0..record.num_parts as usize {
                        first_point_in_part = record.parts[part] as usize;
                        last_point_in_part = if part < record.num_parts as usize - 1 {
                            record.parts[part + 1] as usize - 1
                        } else {
                            record.num_points as usize - 1
                        };

                        // Create a polyline from the part
                        let pl = Polyline::new(
                            &(record.points[first_point_in_part..=last_point_in_part]),
                            record_num,
                        );

                        // Find the bounding box for the part?
                        bb.push(pl.get_bounding_box());

                        in_polylines.push(pl);
                    }
                }
            }
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;
        output.projection = projection;

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        // hunt for intersections in the overlapping bounding boxes
        let mut polylines = vec![];
        let mut lengths = vec![];
        let mut line_length: f64;
        for record_num1 in 0..in_polylines.len() {
            let mut pl = in_polylines[record_num1].clone();
            // let num_splits = pl.num_splits();
            for record_num2 in record_num1 + 1..in_polylines.len() {
                if bb[record_num1].overlaps(bb[record_num2]) {
                    // find any intersections between the polylines
                    find_split_points_at_line_intersections(
                        &mut pl,
                        &mut (in_polylines[record_num2]),
                    );
                }
            }
            let split_lines = pl.split();
            for j in 0..split_lines.len() {
                line_length = split_lines[j].length();
                if line_length > EPSILON {
                    polylines.push(split_lines[j].clone());
                    lengths.push(line_length);
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * (record_num1 + 1) as f64 / in_polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Finding line intersections: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Find duplicate polylines and remove them
        let mut duplicates = vec![false; polylines.len()];
        for i in 0..polylines.len() {
            if !duplicates[i] {
                for j in (i + 1)..polylines.len() {
                    if polylines[i] == polylines[j] {
                        duplicates[j] = true;
                    }
                }
            }
        }
        for i in (0..polylines.len()).rev() {
            if duplicates[i] {
                polylines.remove(i);
            }
        }

        let num_endnodes = polylines.len() * 2;
        /*
            The structure of endnodes is as such:
            1. the starting node for polyline 'a' is a * 2.
            2. the ending node for polyline 'a' is a * 2 + 1.
            3. endnode to polyline = e / 2
            4. is an endnode a starting point? e % 2 == 0
        */
        let mut endnodes: Vec<Vec<usize>> = vec![vec![]; num_endnodes];

        // now add the endpoints of each polyline into a kd tree
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut kdtree = KdTree::with_capacity(dimensions, capacity_per_node);
        let mut p1: Point2D;
        let mut p2: Point2D;
        let mut p3: Point2D;
        let mut p4: Point2D;
        println!("Creating tree...");
        for i in 0..polylines.len() {
            p1 = polylines[i].first_vertex();
            kdtree.add([p1.x, p1.y], first_node_id(i)).unwrap();

            p2 = polylines[i].last_vertex();
            kdtree.add([p2.x, p2.y], last_node_id(i)).unwrap();
        }

        // Find the neighbours of each endnode and check for dangling arcs
        // and self-closing arcs which form single-line polys.
        println!("Finding node vertices...");
        let mut is_acyclic_arc = vec![false; polylines.len()];
        let mut node_angles: Vec<Vec<f64>> = vec![vec![]; num_endnodes];
        let mut heading: f64;
        for i in 0..polylines.len() {
            p1 = polylines[i].first_vertex();
            p2 = polylines[i].last_vertex();

            // check the first vertex
            let ret = kdtree
                .within(&[p1.x, p1.y], EPSILON, &squared_euclidean)
                .unwrap();
            if ret.len() == 1 {
                is_acyclic_arc[i] = true;
            } else {
                p3 = polylines[i][1];
                for a in 0..ret.len() {
                    let index = *ret[a].1;
                    if index == last_node_id(i) && polylines[i].len() <= 2 {
                        is_acyclic_arc[i] = true;
                    }
                    if index != first_node_id(i) && !is_acyclic_arc[index / 2] {
                        // && index != last_node_id(i)
                        endnodes[first_node_id(i)].push(index);
                        p4 = if is_first_node(index) {
                            polylines[index / 2][1]
                        } else {
                            polylines[index / 2][polylines[index / 2].len() - 2]
                        };
                        heading = Point2D::change_in_heading(p3, p1, p4);
                        node_angles[first_node_id(i)].push(heading);
                    }
                }
            }

            // the first vertex showed that this line is a dangling arc,
            // don't bother connecting it to the graph
            if !is_acyclic_arc[i] {
                // check the last vertex
                let ret = kdtree
                    .within(&[p2.x, p2.y], EPSILON, &squared_euclidean)
                    .unwrap();
                if ret.len() == 1 {
                    is_acyclic_arc[i] = true;
                } else {
                    p3 = polylines[i][polylines[i].len() - 2];
                    for a in 0..ret.len() {
                        let index = *ret[a].1;
                        if index != last_node_id(i) && !is_acyclic_arc[index / 2] {
                            // && index != first_node_id(i)
                            endnodes[last_node_id(i)].push(index);
                            p4 = if is_first_node(index) {
                                polylines[index / 2][1]
                            } else {
                                polylines[index / 2][polylines[index / 2].len() - 2]
                            };
                            heading = Point2D::change_in_heading(p3, p2, p4);
                            node_angles[last_node_id(i)].push(heading);
                        }
                    }
                }
            }
        }

        // Find connecting arcs. These are arcs that don't form loops. The only way to
        // travel from one endnode to the other is to travel through the polyline. They
        // can be safely removed from the graph.
        let mut source_node: usize;
        let mut target_node: usize;
        for i in 0..polylines.len() {
            if !is_acyclic_arc[i] {
                source_node = last_node_id(i); // start with the end node of the polyline
                target_node = first_node_id(i); // end with the start node of the polyline

                let mut prev = vec![num_endnodes; num_endnodes];

                // set the source node's prev value to anything other than num_endnodes
                prev[source_node] = num_endnodes + 1;

                // initialize the queue
                let mut queue = BinaryHeap::with_capacity(num_endnodes);
                for a in &endnodes[source_node] {
                    prev[*a] = source_node;
                    queue.push(Link {
                        id: *a,
                        priority: 0f64,
                    });
                }

                let mut target_found = false;
                while !queue.is_empty() && !target_found {
                    let link = queue.pop().expect("Error during pop operation.");
                    if link.id == target_node {
                        // This happens for a single-line polygon.
                        target_found = true;
                        break;
                    }
                    let other_side = get_other_endnode(link.id);
                    prev[other_side] = link.id;
                    for a in &endnodes[other_side] {
                        if prev[*a] == num_endnodes {
                            prev[*a] = other_side;
                            if *a == target_node {
                                target_found = true;
                                break;
                            }
                            if !is_acyclic_arc[*a / 2] {
                                queue.push(Link {
                                    id: *a,
                                    priority: link.priority + lengths[link.id / 2],
                                });
                            }
                        }
                    }
                }

                if !target_found {
                    is_acyclic_arc[i] = true;
                }
            }

            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Finding acyclic arcs: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for i in 0..polylines.len() {
            let node1 = first_node_id(i);
            for n in (0..endnodes[node1].len()).rev() {
                let node2 = endnodes[node1][n];
                if is_acyclic_arc[node2 / 2] {
                    endnodes[node1].remove(n);
                    node_angles[node1].remove(n);
                }
            }
            let node1 = last_node_id(i);
            for n in (0..endnodes[node1].len()).rev() {
                let node2 = endnodes[node1][n];
                if is_acyclic_arc[node2 / 2] {
                    endnodes[node1].remove(n);
                    node_angles[node1].remove(n);
                }
            }
        }

        bb.clear();
        let mut current_node: usize;
        let mut neighbour_node: usize;
        let mut num_neighbours: usize;
        let mut existing_polygons = HashSet::new();
        let mut existing_hull = HashSet::new();
        let mut hull_geometries: Vec<ShapefileGeometry> = vec![];
        let mut max_val: f64;
        let mut max_val_index: usize;
        let mut k: usize;
        let mut num_vertices: usize;
        let mut other_side: usize;
        let mut target_found: bool;
        let mut assigned = vec![0usize; polylines.len()];
        let mut is_clockwise: bool;
        let mut fid = 1;
        for i in 0..polylines.len() {
            if !is_acyclic_arc[i] && assigned[i] < 2 {
                // starting at the last vertex, traverse a chain of lines, always
                // taking the rightmost line at each junction. Stop when you encounter
                // the first vertex of the line.

                source_node = last_node_id(i); // start with the end node of the polyline
                target_node = first_node_id(i); // end with the start node of the polyline

                let mut prev = vec![num_endnodes; num_endnodes];

                target_found = false;

                current_node = source_node;
                loop {
                    num_neighbours = endnodes[current_node].len();
                    if num_neighbours > 1 {
                        // We're at a junction and we should take the
                        // rightmost line.
                        max_val = node_angles[current_node][0];
                        max_val_index = 0;
                        for a in 1..num_neighbours {
                            if node_angles[current_node][a] > max_val {
                                max_val = node_angles[current_node][a];
                                max_val_index = a;
                            }
                            if endnodes[current_node][a] == target_node {
                                neighbour_node = endnodes[current_node][a];
                                prev[neighbour_node] = current_node;
                                break;
                            }
                        }
                        neighbour_node = endnodes[current_node][max_val_index];
                        other_side = get_other_endnode(neighbour_node);
                        prev[neighbour_node] = current_node;
                        prev[other_side] = neighbour_node;
                        if neighbour_node == target_node || other_side == target_node {
                            target_found = true;
                            break;
                        }
                        current_node = other_side;
                    } else if num_neighbours == 1 {
                        // There's only one way forward, so take it.
                        neighbour_node = endnodes[current_node][0];
                        other_side = get_other_endnode(neighbour_node);
                        prev[neighbour_node] = current_node;
                        prev[other_side] = neighbour_node;
                        if neighbour_node == target_node || other_side == target_node {
                            target_found = true;
                            break;
                        }
                        current_node = other_side;
                    } else {
                        // because we've removed links to danling arcs, this should never occur
                        break;
                    }
                }

                if target_found {
                    // traverse from the target to the source
                    let mut lines: Vec<usize> = vec![];
                    let mut backlinks: Vec<usize> = vec![];
                    k = target_node;
                    num_vertices = 0;
                    // let mut output_poly = true;
                    while k != source_node {
                        k = prev[k];
                        backlinks.push(k);
                        let pl = k / 2;
                        if !is_first_node(k) {
                            // don't add polylines twice. Add at the ending node.
                            lines.push(pl);
                            num_vertices += polylines[pl].len() - 1;
                        }
                    }
                    backlinks.push(target_node);

                    // join the lines
                    lines.reverse();
                    backlinks.reverse();
                    let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
                    for a in 0..lines.len() {
                        let pl = lines[a];
                        let mut v = (polylines[pl].vertices).clone();
                        if backlinks[a * 2] > backlinks[a * 2 + 1] {
                            v.reverse();
                        }
                        if a < lines.len() - 1 {
                            v.pop();
                        }
                        vertices.append(&mut v);
                    }

                    // Is it clockwise order?
                    is_clockwise = is_clockwise_order(&vertices);

                    // don't add the same poly more than once
                    let mut test_poly = lines.clone();
                    test_poly.sort();
                    if !existing_polygons.contains(&test_poly) {
                        if is_clockwise {
                            existing_polygons.insert(test_poly);
                            for a in 0..lines.len() {
                                assigned[lines[a]] += 1;
                            }

                            // output the polygon
                            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                            sfg.add_part(&vertices);
                            bb.push(sfg.get_bounding_box());
                            output.add_record(sfg);

                            output
                                .attributes
                                .add_record(vec![FieldData::Int(fid)], false);
                            fid += 1;
                        }
                    }

                    if !is_clockwise {
                        // This could be a hull.
                        test_poly = lines.clone();
                        test_poly.sort();
                        if !existing_hull.contains(&test_poly) {
                            existing_hull.insert(test_poly);
                            for a in 0..lines.len() {
                                assigned[lines[a]] += 1;
                            }

                            // vertices.reverse();
                            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                            sfg.add_part(&vertices);
                            hull_geometries.push(sfg);
                        }
                    }
                }

                if assigned[i] < 2 {
                    ///////////////////////////////////////
                    // now check for a left-side polygon //
                    ///////////////////////////////////////
                    source_node = first_node_id(i); // start with the first node of the polyline
                    target_node = last_node_id(i); // end with the last node of the polyline

                    let mut prev = vec![num_endnodes; num_endnodes];

                    target_found = false;

                    current_node = source_node;
                    loop {
                        num_neighbours = endnodes[current_node].len();
                        if num_neighbours > 1 {
                            // We're at a junction and we should take the
                            // rightmost line.
                            max_val = node_angles[current_node][0];
                            max_val_index = 0;
                            for a in 1..num_neighbours {
                                if node_angles[current_node][a] > max_val {
                                    max_val = node_angles[current_node][a];
                                    max_val_index = a;
                                }
                                if endnodes[current_node][a] == target_node {
                                    neighbour_node = endnodes[current_node][a];
                                    prev[neighbour_node] = current_node;
                                    break;
                                }
                            }
                            neighbour_node = endnodes[current_node][max_val_index];
                            other_side = get_other_endnode(neighbour_node);
                            prev[neighbour_node] = current_node;
                            prev[other_side] = neighbour_node;
                            if neighbour_node == target_node || other_side == target_node {
                                target_found = true;
                                break;
                            }
                            current_node = other_side;
                        } else if num_neighbours == 1 {
                            // There's only one way forward, so take it.
                            neighbour_node = endnodes[current_node][0];
                            other_side = get_other_endnode(neighbour_node);
                            prev[neighbour_node] = current_node;
                            prev[other_side] = neighbour_node;
                            if neighbour_node == target_node || other_side == target_node {
                                target_found = true;
                                break;
                            }
                            current_node = other_side;
                        } else {
                            // because we've removed links to danling arcs, this should never occur
                            break;
                        }
                    }

                    if target_found {
                        // traverse from the target to the source
                        let mut lines: Vec<usize> = vec![];
                        let mut backlinks: Vec<usize> = vec![];
                        k = target_node;
                        num_vertices = 0;
                        // let mut output_poly = true;
                        while k != source_node {
                            k = prev[k];
                            backlinks.push(k);
                            let pl = k / 2;
                            if is_first_node(k) {
                                // don't add polylines twice. Add at the first node.
                                lines.push(pl);
                                num_vertices += polylines[pl].len() - 1;
                            }
                        }
                        backlinks.push(target_node);

                        // join the lines and then output the polygon
                        lines.reverse();
                        backlinks.reverse();
                        let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
                        for a in 0..lines.len() {
                            let pl = lines[a];
                            // assigned[pl] += 1;
                            let mut v = (polylines[pl].vertices).clone();
                            if backlinks[a * 2] > backlinks[a * 2 + 1] {
                                v.reverse();
                            }
                            if a < lines.len() - 1 {
                                v.pop();
                            }
                            vertices.append(&mut v);
                        }

                        // Is it clockwise order?
                        is_clockwise = is_clockwise_order(&vertices);

                        // don't add the same poly more than once
                        let mut test_poly = lines.clone();
                        test_poly.sort();
                        if !existing_polygons.contains(&test_poly) {
                            if is_clockwise {
                                existing_polygons.insert(test_poly);
                                for a in 0..lines.len() {
                                    assigned[lines[a]] += 1;
                                }

                                // output the polygon
                                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                sfg.add_part(&vertices);
                                bb.push(sfg.get_bounding_box());
                                output.add_record(sfg);

                                output
                                    .attributes
                                    .add_record(vec![FieldData::Int(fid)], false);
                                fid += 1;
                            }
                        }

                        if !is_clockwise {
                            // This could be a hull.
                            test_poly = lines.clone();
                            test_poly.sort();
                            if !existing_hull.contains(&test_poly) {
                                for a in 0..lines.len() {
                                    assigned[lines[a]] += 1;
                                }
                                existing_hull.insert(test_poly);

                                // vertices.reverse();
                                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                sfg.add_part(&vertices);
                                hull_geometries.push(sfg);
                            }
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Finding polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // can any of the hulls be added as holes in other polygons?
        for a in 0..hull_geometries.len() {
            let hull_bb = hull_geometries[a].get_bounding_box();
            for b in 0..bb.len() {
                if hull_bb.entirely_contained_within(bb[b]) {
                    if output.records[b].num_parts > 1 {
                        if poly_in_poly(
                            &(hull_geometries[a].points),
                            &(output.records[b].points[0..output.records[b].parts[1] as usize]),
                        ) {
                            output.records[b].add_part(&(hull_geometries[a].points));
                        }
                    } else {
                        if poly_in_poly(&(hull_geometries[a].points), &(output.records[b].points)) {
                            output.records[b].add_part(&(hull_geometries[a].points));
                        }
                    }
                }
            }
        }

        // let mut fid = 1;
        // for i in 0..polylines.len() {
        //     if !is_acyclic_arc[i] {
        //         let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
        //         sfg.add_part(&polylines[i].vertices);
        //         output.add_record(sfg);

        //         output
        //             .attributes
        //             .add_record(vec![FieldData::Int(fid)], false);
        //         fid += 1;
        //     }
        // }

        // if verbose {
        //     println!("Saving data...")
        // };
        // let _ = match output.write() {
        //     Ok(_) => if verbose {
        //         println!("Output file written")
        //     },
        //     Err(e) => return Err(e),
        // };

        // let elapsed_time = get_formatted_elapsed_time(start);

        // if verbose {
        //     println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        // }

        // return Ok(());

        // // calculate the lengths of polylines
        // println!("Calculating line lengths...");
        // let mut source_node: usize;
        // let mut target_node: usize;
        // let mut poly_lengths = Vec::with_capacity(polylines.len());
        // let mut length: f64;
        // let mut flag: bool;
        // for i in 0..polylines.len() {
        //     length = polylines[i].length();
        //     flag = true;
        //     source_node = first_node_id(i);
        //     while flag {
        //         if num_neighbours[source_node] == 1 {
        //             // straight line connection; non-junction
        //             source_node = endnodes[source_node][0];
        //             if source_node / 2 != i {
        //                 length += polylines[source_node / 2].length();
        //             } else {
        //                 flag = false;
        //             }
        //         } else {
        //             flag = false;
        //         }
        //     }
        //     flag = true;
        //     source_node = last_node_id(i);
        //     while flag {
        //         if num_neighbours[source_node] == 1 {
        //             // straight line connection; non-junction
        //             source_node = endnodes[source_node][0];
        //             if source_node / 2 != i {
        //                 length += polylines[source_node / 2].length();
        //             } else {
        //                 flag = false;
        //             }
        //         } else {
        //             flag = false;
        //         }
        //     }
        //     poly_lengths.push((length, i));
        // }

        // poly_lengths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // let mut fid = 1i32;
        // let mut assigned = vec![0usize; polylines.len()];
        // let mut i: usize;
        // for j in 0..polylines.len() {
        //     i = poly_lengths[j].1;
        //     if assigned[i] == 0 && !is_dangling_arc[i] {
        //         source_node = last_node_id(i); // start with the end node of the polyline
        //         target_node = first_node_id(i); // end with the start node of the polyline

        //         let mut prev = vec![num_endnodes; num_endnodes];

        //         // set the source node's prev value to anything other than num_endnodes
        //         prev[source_node] = num_endnodes + 1;

        //         // initialize the queue
        //         let mut queue = BinaryHeap::with_capacity(num_endnodes);
        //         for a in &endnodes[source_node] {
        //             prev[*a] = source_node;
        //             queue.push(Link {
        //                 id: *a,
        //                 priority: 0f64,
        //             });
        //         }

        //         let mut num_links = 0;
        //         let mut target_found = false;
        //         while !queue.is_empty() && !target_found {
        //             let link = queue.pop().expect("Error during pop operation.");
        //             if link.id == target_node {
        //                 // this happens for a single-line polygon.
        //                 target_found = true;
        //                 break;
        //             }
        //             num_links += 1;
        //             let other_side = get_other_endnode(link.id);
        //             prev[other_side] = link.id;
        //             for a in &endnodes[other_side] {
        //                 if prev[*a] == num_endnodes {
        //                     prev[*a] = other_side;
        //                     if *a == target_node {
        //                         target_found = true;
        //                         break;
        //                     }
        //                     if assigned[*a / 2] < 2 {
        //                         queue.push(Link {
        //                             id: *a,
        //                             priority: link.priority + lengths[link.id / 2],
        //                         });
        //                     }
        //                 }
        //             }
        //         }

        //         if target_found {
        //             // traverse from the target to the source
        //             let mut lines: Vec<usize> = vec![];
        //             let mut backlinks: Vec<usize> = vec![];
        //             let mut k = target_node;
        //             let mut num_vertices = 0;
        //             let mut output_poly = true;
        //             while k != source_node {
        //                 k = prev[k];
        //                 backlinks.push(k);
        //                 if backlinks.len() > num_links + 10 {
        //                     // println!("I can't find my way back on {}", i);
        //                     output_poly = false;
        //                     break;
        //                 }
        //                 let pl = k / 2;
        //                 if !is_first_node(k) {
        //                     // don't add polylines twice. Add at the ending node.
        //                     lines.push(pl);
        //                     num_vertices += polylines[pl].len() - 1;
        //                 }
        //             }
        //             backlinks.push(target_node);

        //             // don't add the same poly more than once
        //             // let mut output_poly = true;
        //             let mut test_poly = lines.clone();
        //             test_poly.sort();
        //             if !existing_polygons.contains(&test_poly) {
        //                 existing_polygons.insert(test_poly);
        //             } else {
        //                 output_poly = false;
        //             }
        //             if output_poly {
        //                 // join the lines and then output the polygon
        //                 lines.reverse();
        //                 backlinks.reverse();
        //                 let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
        //                 for a in 0..lines.len() {
        //                     let pl = lines[a];
        //                     assigned[pl] += 1;
        //                     let mut v = (polylines[pl].vertices).clone();
        //                     if backlinks[a * 2] > backlinks[a * 2 + 1] {
        //                         v.reverse();
        //                     }
        //                     if a < lines.len() - 1 {
        //                         v.pop();
        //                     }
        //                     vertices.append(&mut v);
        //                 }

        //                 let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
        //                 if !is_clockwise_order(&vertices) {
        //                     vertices.reverse();
        //                 }
        //                 sfg.add_part(&vertices);
        //                 output.add_record(sfg);

        //                 output
        //                     .attributes
        //                     .add_record(vec![FieldData::Int(fid)], false);
        //                 fid += 1;

        //                 // If it's a junction, don't ever traverse this pathway through the junction again.
        //                 // If it's a straightline connection, it's fine to traverse a second time.
        //                 // for a in 0..backlinks.len() - 1 {
        //                 //     let node1 = backlinks[a];
        //                 //     let node2 = backlinks[a + 1];
        //                 //     if node1 / 2 != node2 / 2 {
        //                 //         // it's not the same line
        //                 //         if num_neighbours[node1] > 1 && num_neighbours[node2] > 1 {
        //                 //             // it's not a straight line connection; it's a three or more junction
        //                 //             for n in 0..endnodes[node1].len() {
        //                 //                 if endnodes[node1][n] == node2 {
        //                 //                     endnodes[node1].remove(n);
        //                 //                     // num_neighbours[node1] -= 1;
        //                 //                     break;
        //                 //                 }
        //                 //             }
        //                 //             for n in 0..endnodes[node2].len() {
        //                 //                 if endnodes[node2][n] == node1 {
        //                 //                     endnodes[node2].remove(n);
        //                 //                     // num_neighbours[node2] -= 1;
        //                 //                     break;
        //                 //                 }
        //                 //             }
        //                 //         }
        //                 //     }
        //                 // }
        //             }
        //         }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * (j + 1) as f64 / polylines.len() as f64) as usize;
        //         if progress != old_progress {
        //             println!("Finding polygons: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // for j in 0..polylines.len() {
        //     i = poly_lengths[j].1;
        //     if assigned[i] == 1 && !is_dangling_arc[i] {
        //         source_node = last_node_id(i); // start with the end node of the polyline
        //         target_node = first_node_id(i); // end with the start node of the polyline

        //         let mut prev = vec![num_endnodes; num_endnodes];

        //         // set the source node's prev value to anything other than num_endnodes
        //         prev[source_node] = num_endnodes + 1;

        //         // initialize the queue
        //         let mut queue = BinaryHeap::with_capacity(num_endnodes);
        //         for a in &endnodes[source_node] {
        //             prev[*a] = source_node;
        //             queue.push(Link {
        //                 id: *a,
        //                 priority: 0f64,
        //             });
        //         }

        //         let mut target_found = false;
        //         while !queue.is_empty() && !target_found {
        //             let link = queue.pop().expect("Error during pop operation.");
        //             if link.id == target_node {
        //                 // this happens at a single-line polygon.
        //                 target_found = true;
        //                 break;
        //             }
        //             let other_side = get_other_endnode(link.id);
        //             prev[other_side] = link.id;
        //             for a in &endnodes[other_side] {
        //                 if prev[*a] == num_endnodes {
        //                     // It hasn't been previously linked to if its prev[] value is num_endnodes.
        //                     // This is the first time we've encountered this node.
        //                     prev[*a] = other_side;
        //                     if *a == target_node {
        //                         target_found = true;
        //                         break;
        //                     }
        //                     if assigned[*a / 2] < 2 {
        //                         queue.push(Link {
        //                             id: *a,
        //                             priority: link.priority + lengths[link.id / 2],
        //                         });
        //                     }
        //                 }
        //             }
        //         }

        //         if target_found {
        //             // traverse from the target to the source
        //             let mut lines: Vec<usize> = vec![];
        //             let mut backlinks: Vec<usize> = vec![];
        //             let mut k = target_node;
        //             let mut num_vertices = 0;
        //             while k != source_node {
        //                 k = prev[k];
        //                 backlinks.push(k);
        //                 let pl = k / 2;
        //                 if !is_first_node(k) {
        //                     // don't add polylines twice. Add at the ending node.
        //                     lines.push(pl);
        //                     num_vertices += polylines[pl].len() - 1;
        //                 }
        //             }
        //             backlinks.push(target_node);

        //             // don't add the same poly more than once
        //             let mut output_poly = true;
        //             let mut test_poly = lines.clone();
        //             test_poly.sort();
        //             if !existing_polygons.contains(&test_poly) {
        //                 existing_polygons.insert(test_poly);
        //             } else {
        //                 output_poly = false;
        //             }
        //             if output_poly {
        //                 // join the lines and then output the polygon
        //                 lines.reverse();
        //                 backlinks.reverse();
        //                 let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
        //                 for a in 0..lines.len() {
        //                     let pl = lines[a];
        //                     assigned[pl] += 1;
        //                     let mut v = (polylines[pl].vertices).clone();
        //                     if backlinks[a * 2] > backlinks[a * 2 + 1] {
        //                         v.reverse();
        //                     }
        //                     if a < lines.len() - 1 {
        //                         v.pop();
        //                     }
        //                     vertices.append(&mut v);
        //                 }
        //                 let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
        //                 if !is_clockwise_order(&vertices) {
        //                     vertices.reverse();
        //                 }
        //                 sfg.add_part(&vertices);
        //                 output.add_record(sfg);

        //                 output
        //                     .attributes
        //                     .add_record(vec![FieldData::Int(fid)], false);
        //                 fid += 1;

        //                 // If it's a junction, don't ever traverse this pathway through the junction again.
        //                 // If it's a straightline connection, it's fine to traverse a second time.
        //                 // for a in 0..backlinks.len() - 1 {
        //                 //     let node1 = backlinks[a];
        //                 //     let node2 = backlinks[a + 1];
        //                 //     if node1 / 2 != node2 / 2 {
        //                 //         // it's not the same line
        //                 //         if num_neighbours[node1] > 1 && num_neighbours[node2] > 1 {
        //                 //             // it's not a straight line connection; it's a three or more junction
        //                 //             for n in 0..endnodes[node1].len() {
        //                 //                 if endnodes[node1][n] == node2 {
        //                 //                     endnodes[node1].remove(n);
        //                 //                     // num_neighbours[node1] -= 1;
        //                 //                     break;
        //                 //                 }
        //                 //             }
        //                 //             for n in 0..endnodes[node2].len() {
        //                 //                 if endnodes[node2][n] == node1 {
        //                 //                     endnodes[node2].remove(n);
        //                 //                     // num_neighbours[node2] -= 1;
        //                 //                     break;
        //                 //                 }
        //                 //             }
        //                 //         }
        //                 //     }
        //                 // }
        //             }
        //         }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * (j + 1) as f64 / polylines.len() as f64) as usize;
        //         if progress != old_progress {
        //             println!("Finding polygons: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

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

#[derive(Debug)]
struct Link {
    id: usize,
    priority: f64,
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        (self.priority - other.priority).abs() < EPSILON && self.id == other.id
    }
}

impl Eq for Link {}

impl Ord for Link {
    fn cmp(&self, other: &Link) -> Ordering {
        // this sorts priorities from low to high
        // and when priorities are equal, id's from
        // high to low.
        let mut ord = other.priority.partial_cmp(&self.priority).unwrap();
        if ord == Ordering::Equal {
            ord = self.id.cmp(&other.id);
        }
        ord
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Link) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn get_other_endnode(index: usize) -> usize {
    if index % 2 == 0 {
        // it's a starting node and we need the end
        return index + 1;
    }
    // it's an end node and we need the starting node
    index - 1
}

fn is_first_node(index: usize) -> bool {
    index % 2 == 0
}

fn first_node_id(polyline: usize) -> usize {
    polyline * 2
}

fn last_node_id(polyline: usize) -> usize {
    polyline * 2 + 1
}
