/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 13/11/2018
Last Modified: 22/11/2018
License: MIT
*/
extern crate kdtree;

use crate::algorithms::{
    interior_point, is_clockwise_order, point_in_poly, poly_in_poly, polygon_area,
};
use crate::structures::{BoundingBox, Polyline};
use crate::tools::*;
use crate::vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON;

/// This tool can be used to remove the interior, or shared, boundaries within a vector
/// polygon coverage. You can either dissolve all interior boundaries or dissolve those
/// boundaries along polygons with the same value of a user-specified attribute within
/// the vector's attribute table. It may be desirable to use the `VectorCleaning` tool
/// to correct any topological errors resulting from the slight misalignment of nodes
/// along shared boundaries in the vector coverage before performing the `Dissolve` operation.
///
/// # See Also
/// `Clip`, `Erase`, `Polygonize`
pub struct Dissolve {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Dissolve {
    pub fn new() -> Dissolve {
        // public constructor
        let name = "Dissolve".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Removes the interior, or shared, boundaries within a vector polygon coverage."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Dissolve Field Attribute".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Dissolve field attribute (optional).".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input".to_string(),
            ),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Snap Tolerance".to_owned(),
            flags: vec!["--snap".to_owned()],
            description: "Snap tolerance.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp --field=SIZE -o=out_file.shp --snap=0.0000001",
            short_exe, name
        ).replace("*", &sep);

        Dissolve {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Dissolve {
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
        let mut dissolve_key = String::new();
        let mut output_file = String::new();
        let mut precision = std::f64::EPSILON;

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
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-field" {
                dissolve_key = if keyval {
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
            } else if flag_val == "-snap" {
                precision = if keyval {
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
                if precision == 0f64 {
                    precision = std::f64::EPSILON;
                }
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Shapefile::read(&input_file)?;
        let projection = input.projection.clone();

        // The overlay file must be of the same ShapeType as the input file
        if input.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector layer must be of the POLYGON base shape type.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, false)?;
        output.projection = projection;

        // What is the index of the foreign field?
        let mut use_dissolve_key = !dissolve_key.is_empty();
        let mut key_index = 0;

        if !dissolve_key.is_empty() {
            key_index = match input.attributes.get_field_num(&dissolve_key) {
                Some(i) => {
                    use_dissolve_key = true;
                    i
                }
                None => {
                    use_dissolve_key = false;
                    0
                }
            };
        }

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));
        if use_dissolve_key {
            output
                .attributes
                .add_field(input.attributes.get_field(key_index));
        }

        // let num_decimals = 6;
        // let precision = 1f64 / num_decimals as f64;

        // how many polygons are there (record parts)
        let mut num_polygons = 0;
        for record_num in 0..input.num_records {
            num_polygons += input.records[record_num].num_parts as usize;
        }

        let mut fid: i32 = 1;
        let mut polygons: Vec<Polyline> = Vec::with_capacity(num_polygons);
        let mut attribute: Vec<FieldData> = Vec::with_capacity(num_polygons);
        let mut polygon_areas: Vec<f64> = Vec::with_capacity(num_polygons);
        let mut is_part_a_hole: Vec<bool> = Vec::with_capacity(num_polygons);
        let mut polygon_bb: Vec<BoundingBox> = Vec::with_capacity(num_polygons);
        let mut first_point_in_part: usize;
        let mut last_point_in_part: usize;
        let mut att: FieldData;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            att = if use_dissolve_key {
                input.attributes.get_value(record_num, &dissolve_key)
            } else {
                // all polygons get a null attribute
                FieldData::Null
            };
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
                polygon_areas.push(polygon_area(&pl.vertices));
                polygon_bb.push(pl.get_bounding_box());
                polygons.push(pl);
                attribute.push(att.clone());

                if record.is_hole(part as i32) {
                    is_part_a_hole.push(true);
                } else {
                    is_part_a_hole.push(false);
                }
            }
        }

        let mut p: Point2D;

        // Break the polygons up into line segments at junction points and endnodes.
        if verbose {
            println!("Breaking polygons into line segments...")
        };
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
        for i in 0..num_polygons {
            for j in 0..polygons[i].len() {
                p = polygons[i][j];
                if j > 0 && j < polygons[i].len() - 1 {
                    tree.add([p.x, p.y], (i, j, false)).unwrap();
                } else {
                    // end node
                    tree.add([p.x, p.y], (i, j, true)).unwrap();
                }
            }
        }

        let mut polylines: Vec<Polyline> = vec![];
        let mut id: usize;
        let mut jn: usize;
        let mut endnode_n: bool;
        let mut dist1: f64;
        let mut dist2: f64;
        let mut num_neighbours: usize;
        let mut neighbour_set = HashSet::new();
        for i in 0..num_polygons {
            let mut line_node = vec![false; polygons[i].len()];
            line_node[0] = true;
            line_node[polygons[i].len() - 1] = true;
            for j in 1..polygons[i].len() - 1 {
                p = polygons[i][j];
                let ret = tree
                    .within(&[p.x, p.y], precision, &squared_euclidean)
                    .unwrap();

                neighbour_set.clear();
                for n in &ret {
                    let data = *n.1;
                    id = data.0;
                    if id != i {
                        neighbour_set.insert(id);
                    }
                }
                num_neighbours = neighbour_set.len();

                if num_neighbours > 1 {
                    // If this point connects three or more polygons, it's a junction.
                    line_node[j] = true;
                } else if num_neighbours == 1 {
                    // what is the neighbouring polygon and node?
                    id = 0;
                    jn = 0;
                    endnode_n = false;
                    for n in &ret {
                        let data = *n.1;
                        id = data.0;
                        if id != i {
                            jn = data.1;
                            endnode_n = data.2;
                            break;
                        }
                    }

                    if endnode_n {
                        // The point may be mid-line, but the neighbouring poly is at an endnode.
                        // We'll have to split this poly here too.
                        line_node[j] = true;
                    } else if jn != 0 {
                        // This is the cleverest part of the process. It handles polygons
                        // that are on the outside. That is polygons that have a neighbouring
                        // poly on one side and no poly on the other side. Part of the polygon
                        // will be a shared boundary but some of it will be part of the exterior
                        // hull of the polygon group. The vertex where this split happens isn't
                        // a junction that can be recognized by the ret.len() > 2 criteria.
                        // Instead, we're hunting for vertices with 1 neighbouring poly but
                        // where the vertex before it or after it are not neighbouring the
                        // same poly.
                        dist1 = (polygons[i][j - 1].distance(&polygons[id][jn - 1]))
                            .min(polygons[i][j - 1].distance(&polygons[id][jn + 1]));
                        dist2 = (polygons[i][j + 1].distance(&polygons[id][jn - 1]))
                            .min(polygons[i][j + 1].distance(&polygons[id][jn + 1]));
                        if dist1 > precision || dist2 > precision {
                            line_node[j] = true;
                        }
                    }
                }
            }

            let mut pl = Polyline::new_empty(i);
            pl.vertices.push(polygons[i][0]);
            for j in 1..polygons[i].len() {
                pl.vertices.push(polygons[i][j]);
                if line_node[j] {
                    polylines.push(pl.clone());
                    pl = Polyline::new_empty(i);
                    pl.vertices.push(polygons[i][j]);
                }
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / num_polygons as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Remove any zero-length line segments
        for i in 0..polylines.len() {
            for j in (1..polylines[i].len()).rev() {
                if polylines[i][j] == polylines[i][j - 1] {
                    polylines[i].remove(j);
                }
            }
        }
        // Remove any single-point lines result from above.
        for i in (0..polylines.len()).rev() {
            if polylines[i].len() < 2 {
                polylines.remove(i);
            }
        }

        if verbose {
            println!("Removing duplicate line segments...")
        };
        // Find duplicate polylines and remove them
        let mut duplicate = vec![false; polylines.len()];
        for i in 0..polylines.len() {
            if !duplicate[i] {
                att = attribute[polylines[i].id].clone();
                for j in (i + 1)..polylines.len() {
                    if polylines[i].nearly_equals(&polylines[j], precision) {
                        if att == attribute[polylines[j].id].clone() {
                            duplicate[i] = true;
                        }
                        duplicate[j] = true;
                        break; // we don't really have more than two overlapping lines ever.
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        for i in (0..polylines.len()).rev() {
            if duplicate[i] {
                polylines.remove(i);
            }
        }

        let num_polylines = polylines.len();

        /*
        ////////////////////////////////////////////////////////////////////
        let mut output2 =
            Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, true)?;
        for i in 0..num_polylines {
            let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
            sfg.add_part(&(polylines[i].vertices));
            output2.add_record(sfg);

            output2
                .attributes
                .add_record(vec![FieldData::Int(fid)], false);
            fid += 1;
        }

        if verbose {
            println!("Saving data...")
        };
        let _ = match output2.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };
        return Ok(());
        ////////////////////////////////////////////////////////////////////
        */

        if verbose {
            println!("Rebuilding polygons...")
        };

        let num_endnodes = num_polylines * 2;
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
        for i in 0..num_polylines {
            p1 = polylines[i].first_vertex();
            kdtree.add([p1.x, p1.y], first_node_id(i)).unwrap();

            p2 = polylines[i].last_vertex();
            kdtree.add([p2.x, p2.y], last_node_id(i)).unwrap();
        }

        // Find the neighbours of each endnode and check for dangling arcs
        // and self-closing arcs which form single-line polys.
        let mut is_acyclic_arc = vec![false; num_polylines];
        let mut node_angles: Vec<Vec<f64>> = vec![vec![]; num_endnodes];
        let mut heading: f64;
        let mut index: usize;
        for i in 0..num_polylines {
            p1 = polylines[i].first_vertex();
            p2 = polylines[i].last_vertex();

            // check the first vertex
            let ret = kdtree
                .within(&[p1.x, p1.y], precision, &squared_euclidean)
                .unwrap();
            if ret.len() == 1 {
                is_acyclic_arc[i] = true;
            } else {
                p3 = polylines[i][1];
                for a in 0..ret.len() {
                    index = *ret[a].1;
                    if index == last_node_id(i) && polylines[i].len() <= 2 {
                        is_acyclic_arc[i] = true;
                    }
                    if index != first_node_id(i) && !is_acyclic_arc[index / 2] {
                        p4 = if is_first_node(index) {
                            polylines[index / 2][1]
                        } else {
                            polylines[index / 2][polylines[index / 2].len() - 2]
                        };
                        heading = Point2D::change_in_heading(p3, p1, p4);
                        if !heading.is_nan() {
                            node_angles[first_node_id(i)].push(heading);
                            endnodes[first_node_id(i)].push(index);
                        }
                    }
                }
                if endnodes[first_node_id(i)].len() == 0 {
                    is_acyclic_arc[i] = true;
                }
            }

            // the first vertex showed that this line is a dangling arc,
            // don't bother connecting it to the graph
            if !is_acyclic_arc[i] {
                // check the last vertex
                let ret = kdtree
                    .within(&[p2.x, p2.y], precision, &squared_euclidean)
                    .unwrap();
                if ret.len() == 1 {
                    is_acyclic_arc[i] = true;
                } else {
                    p3 = polylines[i][polylines[i].len() - 2];
                    for a in 0..ret.len() {
                        index = *ret[a].1;
                        if index != last_node_id(i) && !is_acyclic_arc[index / 2] {
                            p4 = if is_first_node(index) {
                                polylines[index / 2][1]
                            } else {
                                polylines[index / 2][polylines[index / 2].len() - 2]
                            };
                            heading = Point2D::change_in_heading(p3, p2, p4);
                            if !heading.is_nan() {
                                node_angles[last_node_id(i)].push(heading);
                                endnodes[last_node_id(i)].push(index);
                            }
                        }
                    }
                    if endnodes[last_node_id(i)].len() == 0 {
                        is_acyclic_arc[i] = true;
                    }
                }
            }
        }

        let mut source_node: usize;
        let mut target_node: usize;

        /*
        let mut lengths = Vec::with_capacity(num_polylines);
        for i in 0..num_polylines {
            lengths.push(polylines[i].length());
        }

        // Find connecting arcs. These are arcs that don't form loops. The only way to
        // travel from one endnode to the other is to travel through the polyline. They
        // can be safely removed from the graph.
        for i in 0..num_polylines {
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
                    let link = queue.pop().unwrap();
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
        }
        */

        let mut node1: usize;
        let mut node2: usize;
        for i in 0..num_polylines {
            node1 = first_node_id(i);
            for n in (0..endnodes[node1].len()).rev() {
                node2 = endnodes[node1][n];
                if is_acyclic_arc[node2 / 2] {
                    endnodes[node1].remove(n);
                    node_angles[node1].remove(n);
                }
            }
            node1 = last_node_id(i);
            for n in (0..endnodes[node1].len()).rev() {
                node2 = endnodes[node1][n];
                if is_acyclic_arc[node2 / 2] {
                    endnodes[node1].remove(n);
                    node_angles[node1].remove(n);
                }
            }
        }

        ///////////////////////////////////////////////////////////////////////////////////////////
        // This is the main part of the analysis. It is responsible for rebuilding the polygons. //
        ///////////////////////////////////////////////////////////////////////////////////////////
        let mut current_node: usize;
        let mut neighbour_node: usize;
        let mut num_neighbours: usize;
        let mut existing_polygons = HashSet::new();
        let mut existing_hull = HashSet::new();
        let mut bb: Vec<BoundingBox> = vec![];
        let mut feature_geometries: Vec<ShapefileGeometry> = vec![];
        let mut hull_geometries: Vec<ShapefileGeometry> = vec![];
        let mut poly_id: Vec<usize> = vec![];
        let mut p: Point2D;
        let mut max_val: f64;
        let mut max_val_index: usize;
        let mut k: usize;
        let mut num_vertices: usize;
        let mut other_side: usize;
        let mut target_found: bool;
        let mut assigned = vec![0usize; num_polylines];
        let mut is_clockwise: bool;
        let mut last_index: usize;
        let mut min_area: f64;
        let mut min_index: usize;
        const INFINITY: f64 = std::f64::INFINITY;
        for i in 0..num_polylines {
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
                        if prev[other_side] != num_endnodes && other_side != target_node {
                            break;
                        }
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
                        if prev[other_side] != num_endnodes && other_side != target_node {
                            break;
                        }
                        prev[neighbour_node] = current_node;
                        prev[other_side] = neighbour_node;
                        if neighbour_node == target_node || other_side == target_node {
                            target_found = true;
                            break;
                        }
                        current_node = other_side;
                    } else {
                        // because we've removed links to dangling arcs, this should never occur
                        break;
                    }
                }

                if target_found {
                    // traverse from the target to the source
                    let mut lines: Vec<usize> = vec![];
                    let mut backlinks: Vec<usize> = vec![];
                    k = target_node;
                    num_vertices = 0;
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

                    // a minimum of four points are needed to form a closed polygon (triangle)
                    if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                        p = vertices[0];
                        if vertices[0].distance(&vertices[vertices.len() - 1]) < precision {
                            last_index = vertices.len() - 1;
                            vertices[last_index] = p;
                        } else {
                            vertices.push(p);
                        }
                    }

                    // Is it clockwise order?
                    is_clockwise = is_clockwise_order(&vertices);

                    // don't add the same poly more than once
                    let mut test_poly = lines.clone();
                    test_poly.sort();
                    if !existing_polygons.contains(&test_poly) {
                        if is_clockwise {
                            if vertices.len() > 3 {
                                // Find the smallest input polygon that this overlaps with
                                p = interior_point(&vertices);
                                min_area = INFINITY;
                                min_index = num_polygons;
                                for j in 0..num_polygons {
                                    if polygon_bb[j].is_point_in_box(p.x, p.y)
                                        && point_in_poly(&p, &(polygons[j].vertices))
                                    {
                                        if polygon_areas[j] < min_area {
                                            min_area = polygon_areas[j];
                                            min_index = j;
                                        }
                                    }
                                }
                                if min_index != num_polygons && !is_part_a_hole[min_index] {
                                    existing_polygons.insert(test_poly);
                                    for a in 0..lines.len() {
                                        assigned[lines[a]] += 1;
                                    }
                                    // output the polygon
                                    let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                    sfg.add_part(&vertices);
                                    bb.push(sfg.get_bounding_box());
                                    feature_geometries.push(sfg);
                                    poly_id.push(min_index);
                                } else {
                                    // It either overlaps with a hole or with no input poly at all.
                                    if !existing_hull.contains(&test_poly) {
                                        existing_hull.insert(test_poly);
                                        for a in 0..lines.len() {
                                            assigned[lines[a]] += 1;
                                        }
                                        vertices.reverse();
                                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                        sfg.add_part(&vertices);
                                        hull_geometries.push(sfg);
                                    }
                                }
                            }
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
                            if vertices.len() > 3 {
                                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                sfg.add_part(&vertices);
                                hull_geometries.push(sfg);
                            }
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
                            if prev[other_side] != num_endnodes && other_side != target_node {
                                break;
                            }
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
                            if prev[other_side] != num_endnodes && other_side != target_node {
                                break;
                            }
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
                            let mut v = (polylines[pl].vertices).clone();
                            if backlinks[a * 2] > backlinks[a * 2 + 1] {
                                v.reverse();
                            }
                            if a < lines.len() - 1 {
                                v.pop();
                            }
                            vertices.append(&mut v);
                        }

                        // a minimum of four points are needed to form a closed polygon (triangle)
                        if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                            p = vertices[0];
                            if vertices[0].distance(&vertices[vertices.len() - 1]) < precision {
                                last_index = vertices.len() - 1;
                                vertices[last_index] = p;
                            } else {
                                vertices.push(p);
                            }
                        }

                        // Is it clockwise order?
                        is_clockwise = is_clockwise_order(&vertices);

                        // don't add the same poly more than once
                        let mut test_poly = lines.clone();
                        test_poly.sort();
                        if !existing_polygons.contains(&test_poly) {
                            if is_clockwise {
                                if vertices.len() > 3 {
                                    // Find an input polygon that this overlaps with
                                    p = interior_point(&vertices);
                                    min_area = INFINITY;
                                    min_index = num_polygons;
                                    for j in 0..num_polygons {
                                        if polygon_bb[j].is_point_in_box(p.x, p.y)
                                            && point_in_poly(&p, &(polygons[j].vertices))
                                        {
                                            if polygon_areas[j] < min_area {
                                                min_area = polygon_areas[j];
                                                min_index = j;
                                            }
                                        }
                                    }
                                    if min_index != num_polygons && !is_part_a_hole[min_index] {
                                        existing_polygons.insert(test_poly);
                                        for a in 0..lines.len() {
                                            assigned[lines[a]] += 1;
                                        }
                                        // output the polygon
                                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                        sfg.add_part(&vertices);
                                        bb.push(sfg.get_bounding_box());
                                        feature_geometries.push(sfg);
                                        poly_id.push(min_index);
                                    } else {
                                        // It either overlaps with a hole or with no input poly at all.
                                        if !existing_hull.contains(&test_poly) {
                                            existing_hull.insert(test_poly);
                                            for a in 0..lines.len() {
                                                assigned[lines[a]] += 1;
                                            }
                                            vertices.reverse();
                                            let mut sfg =
                                                ShapefileGeometry::new(ShapeType::Polygon);
                                            sfg.add_part(&vertices);
                                            hull_geometries.push(sfg);
                                        }
                                    }
                                }
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
                                if vertices.len() > 3 {
                                    let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                                    sfg.add_part(&vertices);
                                    hull_geometries.push(sfg);
                                }
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / num_polylines as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // can any of the hulls be added as holes in other polygons?
        if verbose {
            println!("Resolving polygon holes...")
        };
        for a in 0..hull_geometries.len() {
            let hull_bb = hull_geometries[a].get_bounding_box();
            for b in 0..bb.len() {
                if hull_bb.entirely_contained_within(bb[b]) {
                    if feature_geometries[b].num_parts > 1 {
                        if poly_in_poly(
                            &(hull_geometries[a].points),
                            &(feature_geometries[b].points
                                [0..feature_geometries[b].parts[1] as usize]),
                        ) {
                            feature_geometries[b].add_part(&(hull_geometries[a].points));
                            break; // it can only be a hole for one polygon
                        }
                    } else {
                        if poly_in_poly(
                            &(hull_geometries[a].points),
                            &(feature_geometries[b].points),
                        ) {
                            feature_geometries[b].add_part(&(hull_geometries[a].points));
                            break; // it can only be a hole for one polygon
                        }
                    }
                }
            }
        }

        for a in 0..feature_geometries.len() {
            output.add_record(feature_geometries[a].clone());
            if use_dissolve_key {
                let att = attribute[poly_id[a]].clone();
                output
                    .attributes
                    .add_record(vec![FieldData::Int(fid), att], false);
            } else {
                output
                    .attributes
                    .add_record(vec![FieldData::Int(fid)], false);
            }
            fid += 1;
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
