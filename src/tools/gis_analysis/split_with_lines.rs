/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/10/2018
Last Modified: 21/10/2018
License: MIT
*/
extern crate kdtree;

use algorithms::{find_split_points_at_line_intersections, interior_point, is_clockwise_order};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::f64::EPSILON;
use std::io::{Error, ErrorKind};
use std::path;
use structures::{BoundingBox, Polyline};
use tools::*;
use vector::*;

/// This tool splits the lines or polygons in one layer using the lines in another layer
/// to define the breaking points. Intersection points between geometries in both layers
/// are considered as split points. The input layer (`--input`) can be of either
/// POLYLINE or POLYGON ShapeType and the output file will share this geometry type.
/// The user must also specify an split layer (`--split`), of POLYLINE ShapeType, used
/// to bisect the input geometries.
///
/// Each split geometry's attribute record will contain `FID` and `PARENT_FID` values
/// and all of the attributes (excluding `FID`'s) of the input layer.
pub struct SplitWithLines {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SplitWithLines {
    pub fn new() -> SplitWithLines {
        // public constructor
        let name = "SplitWithLines".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Splits the lines or polygons in one layer using the lines in another layer."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines or Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector line or polygon file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["--split".to_owned()],
            description: "Input vector polyline file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=polygons.shp --split=lines.shp -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        SplitWithLines {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SplitWithLines {
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
        let mut input1_file: String = "".to_string();
        let mut input2_file: String = "".to_string();
        let mut output_file: String = "".to_string();

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
                input1_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-split" {
                input2_file = if keyval {
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

        if !input1_file.contains(path::MAIN_SEPARATOR) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }

        if !input2_file.contains(path::MAIN_SEPARATOR) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input1 = Shapefile::read(&input1_file)?;

        // make sure the input vector file is of polyline type
        if input1.header.shape_type.base_shape_type() != ShapeType::PolyLine
            && input1.header.shape_type.base_shape_type() != ShapeType::Polygon
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE or POLYGON base shape type.",
            ));
        }

        let input2 = Shapefile::read(&input2_file)?;

        // make sure the input vector file is of polyline type
        if input2.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE base shape type.",
            ));
        }

        // Create lists of imported attributes from each input.
        // Append all fields except the ubiquitous FID field
        let mut input1_attributes = vec![];
        let mut input1_att_nums = vec![];
        for a in 0..input1.attributes.get_num_fields() {
            let f = input1.attributes.get_field(a);
            if f.name.to_lowercase() != "fid" {
                input1_attributes.push(f.clone());
                input1_att_nums.push(a);
            }
        }

        // Get the polylines and bounding boxes of each of the features in input1 and input 2
        let mut num_polys = 0;
        let mut first_point_in_part: usize;
        let mut last_point_in_part: usize;
        let mut polylines1: Vec<Polyline> = Vec::with_capacity(input1.get_total_num_parts());
        let mut bb1: Vec<BoundingBox> = Vec::with_capacity(input1.get_total_num_parts());
        for record_num in 0..input1.num_records {
            let record = input1.get_record(record_num);
            for part in 0..record.num_parts as usize {
                num_polys += 1;
                first_point_in_part = record.parts[part] as usize;
                last_point_in_part = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };

                // Create a polyline from the part
                let mut pl = Polyline::new(
                    &(record.points[first_point_in_part..=last_point_in_part]),
                    record_num,
                );

                // Find the bounding box for the part?
                bb1.push(pl.get_bounding_box());

                polylines1.push(pl);
            }
        }

        let mut polylines2: Vec<Polyline> = Vec::with_capacity(input2.get_total_num_parts());
        let mut bb2: Vec<BoundingBox> = Vec::with_capacity(input2.get_total_num_parts());
        for record_num in 0..input2.num_records {
            let record = input2.get_record(record_num);
            for part in 0..record.num_parts as usize {
                first_point_in_part = record.parts[part] as usize;
                last_point_in_part = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };

                // Create a polyline from the part
                let mut pl = Polyline::new(
                    &(record.points[first_point_in_part..=last_point_in_part]),
                    record_num + num_polys,
                );

                // Find the bounding box for the part?
                bb2.push(pl.get_bounding_box());

                polylines2.push(pl);
            }
        }

        if input1.header.shape_type.base_shape_type() == ShapeType::PolyLine {
            // create output file
            let mut output = Shapefile::initialize_using_file(
                &output_file,
                &input1,
                ShapeType::PolyLine,
                false,
            )?;

            // add the attributes
            output
                .attributes
                .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

            output.attributes.add_field(&AttributeField::new(
                "PARENT_FID",
                FieldDataType::Int,
                7u8,
                0u8,
            ));

            for a in 0..input1_attributes.len() {
                output.attributes.add_field(&input1_attributes[a].clone());
            }

            // hunt for intersections in the overlapping bounding boxes
            let mut fid = 1i32;
            for record_num1 in 0..polylines1.len() {
                for record_num2 in 0..polylines2.len() {
                    if bb1[record_num1].overlaps(bb2[record_num2]) {
                        // find any intersections between the polylines
                        find_split_points_at_line_intersections(
                            &mut polylines1[record_num1],
                            &mut polylines2[record_num2],
                        );
                    }
                }
                let split_lines = polylines1[record_num1].split();
                for j in 0..split_lines.len() {
                    // output the polylines
                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&(split_lines[j].vertices));
                    output.add_record(sfg);

                    let mut atts: Vec<FieldData> = Vec::with_capacity(input1_att_nums.len() + 2);
                    atts.push(FieldData::Int(fid));
                    fid += 1;
                    atts.push(FieldData::Int(split_lines[j].id as i32));
                    let in_atts = input1.attributes.get_record(split_lines[j].id);
                    for a in 0..input1_att_nums.len() {
                        atts.push(in_atts[input1_att_nums[a]].clone());
                    }
                    output.attributes.add_record(atts, false);
                }

                if verbose {
                    progress =
                        (100.0_f64 * (record_num1 + 1) as f64 / polylines1.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // Some features were found. Save the output file.
            if verbose {
                println!("Saving data...")
            };
            let _ = match output.write() {
                Ok(_) => if verbose {
                    println!("Output file written")
                },
                Err(e) => return Err(e),
            };
        } else {
            // create output file
            let mut output =
                Shapefile::initialize_using_file(&output_file, &input1, ShapeType::Polygon, false)?;

            // add the attributes
            output
                .attributes
                .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

            output.attributes.add_field(&AttributeField::new(
                "PARENT_FID",
                FieldDataType::Int,
                7u8,
                0u8,
            ));

            for a in 0..input1_attributes.len() {
                output.attributes.add_field(&input1_attributes[a].clone());
            }

            // hunt for intersections in the overlapping bounding boxes
            let mut polylines = vec![];
            let mut lengths = vec![];
            for record_num1 in 0..polylines1.len() {
                for record_num2 in 0..polylines2.len() {
                    if bb1[record_num1].overlaps(bb2[record_num2]) {
                        // find any intersections between the polylines
                        find_split_points_at_line_intersections(
                            &mut polylines1[record_num1],
                            &mut polylines2[record_num2],
                        );
                    }
                }
                let split_lines = polylines1[record_num1].split();
                for j in 0..split_lines.len() {
                    polylines.push(split_lines[j].clone());
                    lengths.push(split_lines[j].length());
                }

                if verbose {
                    progress =
                        (100.0_f64 * (record_num1 + 1) as f64 / polylines1.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            for record_num2 in 0..polylines2.len() {
                let split_lines = polylines2[record_num2].split();
                for j in 0..split_lines.len() {
                    polylines.push(split_lines[j].clone());
                    lengths.push(split_lines[j].length());
                }
            }

            let num_endnodes = polylines.len() * 2;
            /*
                The structure of endnodes is as such:
                1. the first node for polyline 'a' is a * 2.
                2. the last node for polyline 'a' is a * 2 + 1.
                3. endnode to polyline = e / 2
                4. is an endnode a starting point? e % 2 == 0
            */
            let mut endnodes: Vec<Vec<usize>> = vec![vec![]; num_endnodes];

            // now add the endpoints of each polyline into a kd tree
            let dimensions = 2;
            let capacity_per_node = 64;
            let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);
            let mut p: Point2D;
            for i in 0..polylines.len() {
                p = polylines[i].first_vertex();
                kdtree.add([p.x, p.y], first_node_id(i)).unwrap();

                p = polylines[i].last_vertex();
                kdtree.add([p.x, p.y], last_node_id(i)).unwrap();

                if verbose {
                    progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Creating tree: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // Find the neighbours of each endnode and check for dangling arcs
            let mut is_dangling_arc = vec![false; polylines.len()];
            for i in 0..polylines.len() {
                // first vertex
                p = polylines[i].first_vertex();
                let ret = kdtree
                    .within(&[p.x, p.y], EPSILON, &squared_euclidean)
                    .unwrap();
                if ret.len() == 1 {
                    is_dangling_arc[i] = true;
                } else {
                    for a in 0..ret.len() {
                        let index = *ret[a].1;
                        if index != first_node_id(i) {
                            endnodes[first_node_id(i)].push(index);
                        }
                    }
                }

                // last vertex
                p = polylines[i].last_vertex();
                let ret = kdtree
                    .within(&[p.x, p.y], EPSILON, &squared_euclidean)
                    .unwrap();
                if ret.len() == 1 {
                    is_dangling_arc[i] = true;
                } else {
                    for a in 0..ret.len() {
                        let index = *ret[a].1;
                        if index != last_node_id(i) {
                            endnodes[last_node_id(i)].push(index);
                        }
                    }
                }

                if verbose {
                    progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Finding node vertices: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let mut existing_polygons = HashSet::new();
            let mut fid = 1i32;
            let mut assigned = vec![0; polylines.len()];
            let mut parent_poly: usize;
            for i in 0..polylines.len() {
                if !is_dangling_arc[i] && assigned[i] < 2 {
                    let source_node = last_node_id(i); // start with the end node of the polyline
                    let target_node = first_node_id(i); // end with the start node of the polyline
                    let mut prev = vec![num_endnodes; num_endnodes];
                    parent_poly = num_polys;

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
                        let other_side = get_other_endnode(link.id);
                        prev[other_side] = link.id;
                        for a in &endnodes[other_side] {
                            if prev[*a] == num_endnodes {
                                // It hasn't been previously linked to if its prev[] value is num_endnodes.
                                // This is the first time we've encountered this node.
                                prev[*a] = other_side;
                                if *a == target_node {
                                    target_found = true;
                                    break;
                                }
                                queue.push(Link {
                                    id: *a,
                                    priority: link.priority + lengths[link.id / 2],
                                });
                            }
                        }
                    }

                    if target_found {
                        // traverse from the target to the source
                        let mut lines: Vec<usize> = vec![];
                        let mut backlinks: Vec<usize> = vec![];
                        let mut k = target_node;
                        let mut num_vertices = 0;
                        while k != source_node {
                            k = prev[k];
                            backlinks.push(k);
                            let pl = k / 2;
                            if !is_first_node(k) {
                                // don't add polylines twice. Add at the ending node.
                                lines.push(pl);
                                num_vertices += polylines[pl].len() - 1;
                                if polylines[pl].id < num_polys {
                                    parent_poly = polylines[pl].id;
                                }
                            }
                        }
                        backlinks.push(target_node);

                        // join the lines and then output the polygon
                        lines.reverse();
                        backlinks.reverse();
                        let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices + 1);
                        let mut output_poly = true;
                        for a in 0..lines.len() {
                            let pl = lines[a];
                            // none of the composing lines can have been used more than twice already.
                            if assigned[pl] > 1 {
                                output_poly = false;
                                break;
                            }
                            let mut v = (polylines[pl].vertices).clone();
                            if backlinks[a * 2] > backlinks[a * 2 + 1] {
                                v.reverse();
                            }
                            if a < lines.len() - 1 {
                                v.pop();
                            }
                            vertices.append(&mut v);
                        }

                        // don't add the same poly more than once
                        let mut test_poly = lines.clone();
                        test_poly.sort();
                        if existing_polygons.contains(&test_poly) {
                            output_poly = false;
                        } else {
                            existing_polygons.insert(test_poly);
                        }

                        if parent_poly >= num_polys {
                            // This would be a polygon formed by the intersection of split lines only.
                            // There is no side that is part of the hull of an input polygon.
                            output_poly = false;
                        }

                        if output_poly {
                            // Is the polygon within the hull of the parent (input) polygon?
                            let interior_point = interior_point(&vertices);
                            if !input1.records[parent_poly].is_point_within_hull(&interior_point) {
                                // A point interior to the output poly should also be interior to the parent poly.
                                output_poly = false;
                            }
                        }

                        if output_poly {
                            for a in 0..lines.len() {
                                assigned[lines[a]] += 1;
                            }
                            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                            if !is_clockwise_order(&vertices) {
                                vertices.reverse();
                            }
                            sfg.add_part(&vertices);
                            output.add_record(sfg);

                            // output
                            //     .attributes
                            //     .add_record(vec![FieldData::Int(fid)], false);
                            // fid += 1;

                            let mut atts: Vec<FieldData> =
                                Vec::with_capacity(input1_att_nums.len() + 2);
                            atts.push(FieldData::Int(fid));
                            fid += 1;
                            atts.push(FieldData::Int(parent_poly as i32));
                            let in_atts = input1.attributes.get_record(parent_poly);
                            for a in 0..input1_att_nums.len() {
                                atts.push(in_atts[input1_att_nums[a]].clone());
                            }
                            output.attributes.add_record(atts, false);
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

            // Some features were found. Save the output file.
            if verbose {
                println!("Saving data...")
            };
            let _ = match output.write() {
                Ok(_) => if verbose {
                    println!("Output file written")
                },
                Err(e) => return Err(e),
            };
        }

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
