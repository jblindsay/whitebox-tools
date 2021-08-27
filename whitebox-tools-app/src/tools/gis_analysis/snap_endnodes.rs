/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/10/2018
Last Modified: 08/04/2019
License: MIT
*/
extern crate kdtree;

use whitebox_common::algorithms::{
    find_split_points_at_line_intersections, interior_point, is_clockwise_order,
};
use whitebox_common::structures::{BoundingBox, Polyline};
use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::f64::EPSILON;
use std::io::{Error, ErrorKind};
use std::path;

pub struct SnapEndnodes {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SnapEndnodes {
    pub fn new() -> SnapEndnodes {
        // public constructor
        let name = "SnapEndnodes".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Snaps end-nodes in a vector line coverage."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Lines File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector line file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Lines,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Lines,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp -o=out_file.shp --snap=0.0000001",
            short_exe, name
        ).replace("*", &sep);

        SnapEndnodes {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SnapEndnodes {
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
        let mut output_file: String = "".to_string();
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
            } else if flag_val == "-snap" {
                precision = if keyval {
                    vec[1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
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
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        // create output file
        let mut output =  Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;

        // make sure the input vector file is of polyline type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POLYLINE base shape type.",
            ));
        }

        let mut first_point_in_part: usize;
        let mut last_point_in_part: usize;
        let mut polylines: Vec<Polyline> = vec![];
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            if record.shape_type != ShapeType::Null {
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
        let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);
        let mut p1: Point2D;
        let mut p2: Point2D;
        let mut p3: Point2D;
        let mut p4: Point2D;
        for i in 0..polylines.len() {
            p1 = polylines[i].first_vertex();
            kdtree.add([p1.x, p1.y], first_node_id(i)).unwrap();

            p2 = polylines[i].last_vertex();
            kdtree.add([p2.x, p2.y], last_node_id(i)).unwrap();
        }

        // Find the neighbours of each endnode.
        for i in 0..polylines.len() {
            p1 = polylines[i].first_vertex();
            p2 = polylines[i].last_vertex();

            // check the first vertex
            let ret = kdtree.within(&[p1.x, p1.y], precision, &squared_euclidean).unwrap();
            if ret.len() > 1 {
                for a in 0..ret.len() {
                    
                }
            }
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
