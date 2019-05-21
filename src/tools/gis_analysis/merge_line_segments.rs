/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/04/2019
Last Modified: 09/04/2019
License: MIT
*/
extern crate kdtree;

use crate::structures::Polyline;
use crate::tools::*;
use crate::vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON;

/// Vector lines can sometimes contain two features that are connected by a shared end vertex. This tool 
/// identifies connected line features in an input vector file (`--input`) and merges them in the output
/// file (`--output`). Two line features are merged if their ends are coincident, and are not coincident 
/// with any other feature (i.e. a bifurcation junction). End vertices are considered to be coincident if 
/// they are within the specified snap distance (`--snap`). 
///
/// # See Also
/// `SplitWithLines`
pub struct MergeLineSegments {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MergeLineSegments {
    pub fn new() -> MergeLineSegments {
        // public constructor
        let name = "MergeLineSegments".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Merges vector line segments into larger features.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp -o=out_file.shp --snap=0.0000001",
            short_exe, name
        ).replace("*", &sep);

        MergeLineSegments {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MergeLineSegments {
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
        let mut precision = std::f64::EPSILON;

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
            if flag_val == "-i" || flag_val.contains("-input") {
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
            } else if flag_val == "-snap" {
                precision = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
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
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector input must be of the PolyLine ShapeType.",
            ));
        }

        // create output file
        let mut output = Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, false)?;
        output.projection = projection;

        // add the attributes
        output.attributes.add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        let mut input_field_mapping = vec![0; input.attributes.get_num_fields()];
        for i in 0..input.attributes.get_num_fields() {
            let att = input.attributes.get_field(i);
            if att.name != "FID" {
                if !output.attributes.contains_field(att) {
                    output.attributes.add_field(&(att.clone()));
                    input_field_mapping[i] = output.attributes.get_num_fields() - 1;
                } else {
                    input_field_mapping[i] = output.attributes.get_field_num(&att.name).unwrap();
                }
            }
        }

        let num_attributes = output.attributes.get_num_fields();

        output.header.shape_type = ShapeType::PolyLine;
        let mut first_point_in_part: usize;
        let mut last_point_in_part: usize;
        let mut polylines: Vec<Polyline> = vec![];
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
                let mut pl = Polyline::new(
                    &(record.points[first_point_in_part..=last_point_in_part]),
                    record_num,
                );
                pl.source_file = 1;
                polylines.push(pl);
            }
        }

        // Break the polylines up into shorter lines at junction points.
        let dimensions = 2;
        let capacity_per_node = 64;
        let mut tree = KdTree::new_with_capacity(dimensions, capacity_per_node);
        let mut p1: Point2D;
        let mut p2: Point2D;
        for i in 0..polylines.len() {
            p1 = polylines[i].first_vertex();
            tree.add([p1.x, p1.y], first_node_id(i)).unwrap();

            p2 = polylines[i].last_vertex();
            tree.add([p2.x, p2.y], last_node_id(i)).unwrap();
        }

        let mut fid = 1i32;
        let mut index: usize;
        let mut current_endnode: usize;
        let mut current_feature: usize;
        let mut flag: bool;
        let mut already_added = vec![false; polylines.len()];
        for i in 0..polylines.len() {
            if !already_added[i] {
                p1 = polylines[i].first_vertex();
                let ret1 = tree.within(&[p1.x, p1.y], precision, &squared_euclidean).unwrap();

                p2 = polylines[i].last_vertex();
                let ret2 = tree.within(&[p2.x, p2.y], precision, &squared_euclidean).unwrap();

                if ret1.len() != 2 && ret2.len() != 2 {
                    // this feature doesn't have an endnode to join; just output it as is
                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&polylines[i].vertices);
                    output.add_record(sfg);
                    let mut out_atts = vec![FieldData::Null; num_attributes];
                    out_atts[0] = FieldData::Int(fid);
                    fid += 1;
                    let atts = input.attributes.get_record(polylines[i].id);
                    for att_num in 0..atts.len() {
                        if input_field_mapping[att_num] != 0 {
                            out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                        }
                    }
                    output.attributes.add_record(out_atts, false);
                    already_added[i] = true;
                } else if ret1.len() != 2 || ret2.len() != 2 {
                    let mut pl = Polyline::new_empty(i);
                    current_endnode = if ret1.len() != 2 {
                        first_node_id(i)
                    } else {
                        last_node_id(i)
                    };

                    flag = true;
                    while flag {
                        current_feature = current_endnode / 2;
                        if is_first_node(current_endnode) {
                            for j in 0..polylines[current_feature].len() {
                                pl.push(polylines[current_feature][j]);
                            }
                        } else {
                            for j in (0..polylines[current_feature].len()).rev() {
                                pl.push(polylines[current_feature][j]);
                            }
                        }
                        already_added[current_feature] = true;

                        // now switch to the other endnode
                        current_endnode = get_other_endnode(current_endnode);

                        // find any attached line segments to the current_endnode
                        p1 = if is_first_node(current_endnode) {
                            polylines[current_feature].first_vertex()
                        } else {
                            polylines[current_feature].last_vertex()
                        };
                        let ret = tree.within(&[p1.x, p1.y], precision, &squared_euclidean).unwrap();
                        if ret.len() == 2 {
                            for a in 0..ret.len() {
                                index = *ret[a].1;
                                if index / 2 != current_feature {
                                    current_endnode = index;
                                    if already_added[current_endnode / 2] {
                                        flag = false;
                                    } 
                                }
                            }
                        } else {
                            flag = false;
                        }
                    }

                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&pl.vertices);
                    output.add_record(sfg);
                    let mut out_atts = vec![FieldData::Null; num_attributes];
                    out_atts[0] = FieldData::Int(fid);
                    fid += 1;
                    let atts = input.attributes.get_record(polylines[i].id);
                    for att_num in 0..atts.len() {
                        if input_field_mapping[att_num] != 0 {
                            out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                        }
                    }
                    output.attributes.add_record(out_atts, false);
                } else if ret1.len() == 2 && ret2.len() == 2 {
                    // This might be a single-segment closed loop, in which case, it should be output
                    flag = true;
                    for a in 0..ret1.len() {
                        index = *ret1[a].1;
                        if index / 2 != i {
                            flag = false; // it's not a closed loop
                        }
                    }
                    if flag {
                        // it's a closed loop, so output it.
                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&polylines[i].vertices);
                        output.add_record(sfg);
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = input.attributes.get_record(polylines[i].id);
                        for att_num in 0..atts.len() {
                            if input_field_mapping[att_num] != 0 {
                                out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                        already_added[i] = true;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // The only line segments that should be remaining are those that are part of multi-segment closed loops.
        for i in 0..polylines.len() {
            if !already_added[i] {
                let mut pl = Polyline::new_empty(i);
                current_endnode = first_node_id(i);

                flag = true;
                while flag {
                    current_feature = current_endnode / 2;
                    if is_first_node(current_endnode) {
                        for j in 0..polylines[current_feature].len() {
                            pl.push(polylines[current_feature][j]);
                        }
                    } else {
                        for j in (0..polylines[current_feature].len()).rev() {
                            pl.push(polylines[current_feature][j]);
                        }
                    }
                    already_added[current_feature] = true;

                    // now switch to the other endnode
                    current_endnode = get_other_endnode(current_endnode);

                    // find any attached line segments to the current_endnode
                    p1 = if is_first_node(current_endnode) {
                        polylines[current_feature].first_vertex()
                    } else {
                        polylines[current_feature].last_vertex()
                    };
                    let ret = tree.within(&[p1.x, p1.y], precision, &squared_euclidean).unwrap();
                    if ret.len() == 2 {
                        for a in 0..ret.len() {
                            index = *ret[a].1;
                            if index / 2 != current_feature {
                                current_endnode = index;
                                if already_added[current_endnode / 2] {
                                    flag = false;
                                } 
                            }
                        }
                    } else {
                        flag = false;
                    }
                }

                let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                sfg.add_part(&pl.vertices);
                output.add_record(sfg);
                let mut out_atts = vec![FieldData::Null; num_attributes];
                out_atts[0] = FieldData::Int(fid);
                fid += 1;
                let atts = input.attributes.get_record(polylines[i].id);
                for att_num in 0..atts.len() {
                    if input_field_mapping[att_num] != 0 {
                        out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                    }
                }
                output.attributes.add_record(out_atts, false);
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
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
