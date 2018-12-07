/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 28/10/2018
Last Modified: 3/11/2018
License: MIT
*/
extern crate kdtree;

use crate::algorithms::{
    find_split_points_at_line_intersections, interior_point, is_clockwise_order, point_in_poly,
    poly_in_poly, poly_overlaps_poly,
};
use crate::structures::{BoundingBox, Polyline};
use crate::tools::*;
use crate::vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

// const EPSILON: f64 = 0.000000001f64; //std::f64::EPSILON;
const EPSILON: f64 = std::f64::EPSILON;

/// This tool will extract all the features, or parts of features, that overlap with the features
/// of the clip vector file. The clipping operation is one of the most common vector overlay
/// operations in GIS and effectively imposes the boundary of the clip layer on a set of input
/// vector features, or target features. The operation is sometimes likened to a 'cookie-cutter'.
/// The input vector file can be of any feature type (i.e. points, lines, polygons), however, the
/// clip vector must consist of polygons.
///
/// # See Also
/// `Erase`
pub struct Clip {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Clip {
    pub fn new() -> Clip {
        // public constructor
        let name = "Clip".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Extract all the features, or parts of features, that overlap with the features of the clip vector."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Clip Polygon Vector File".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Input clip polygon vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lines1.shp --clip=clip_poly.shp -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        Clip {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Clip {
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
        let mut clip_file = String::new();
        let mut output_file = String::new();

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
            } else if flag_val == "-clip" {
                clip_file = if keyval {
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !clip_file.contains(&sep) && !clip_file.contains("/") {
            clip_file = format!("{}{}", working_directory, clip_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let clip = Arc::new(Shapefile::read(&clip_file)?);

        // The clip file must be of Polygon base ShapeType
        if clip.header.shape_type.base_shape_type() != ShapeType::Polygon {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input clip vector data must be of POLYGON base shape type.",
            ));
        }

        // Get the bounding boxes of each of the clip parts.
        let mut clip_bb: Vec<BoundingBox> = vec![];
        let mut clip_polylines: Vec<Polyline> = vec![];
        let mut is_clip_part_a_hole: Vec<bool> = vec![];
        let mut first_point_in_part: usize;
        let mut last_point_in_part: usize;
        for record_num in 0..clip.num_records {
            let record = clip.get_record(record_num);
            for part in 0..record.num_parts as usize {
                first_point_in_part = record.parts[part] as usize;
                last_point_in_part = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };
                clip_bb.push(BoundingBox::from_points(
                    &(record.points[first_point_in_part..=last_point_in_part]),
                ));

                // Create a polyline from the part
                let mut pl = Polyline::new(
                    &(record.points[first_point_in_part..=last_point_in_part]),
                    record_num,
                );
                pl.source_file = 1; // 1 will mean the clip file and 2 will mean the input file.
                clip_polylines.push(pl);

                if record.is_hole(part as i32) {
                    is_clip_part_a_hole.push(true);
                } else {
                    is_clip_part_a_hole.push(false);
                }
            }
        }

        let input = Shapefile::read(&input_file)?;
        let projection = input.projection.clone();

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;
        output.projection = projection;

        let (table_contains_fid, fid_field_num) = match output.attributes.get_field_num("FID") {
            Some(v) => (true, v),
            None => (false, 0),
        };

        let clip_bb = Arc::new(clip_bb);
        let is_clip_part_a_hole = Arc::new(is_clip_part_a_hole);

        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();

        match input.header.shape_type.base_shape_type() {
            ShapeType::Point => {
                let clip_polylines = Arc::new(clip_polylines);
                for tid in 0..num_procs {
                    let input = input.clone();
                    let clip_bb = clip_bb.clone();
                    let clip_polylines = clip_polylines.clone();
                    let is_clip_part_a_hole = is_clip_part_a_hole.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let mut p: Point2D;
                        let mut out: bool;
                        for record_num in (0..input.num_records).filter(|r| r % num_procs == tid) {
                            out = false;
                            let record = input.get_record(record_num);
                            p = record.points[0];
                            for a in 0..clip_polylines.len() {
                                if clip_bb[a].is_point_in_box(p.x, p.y) {
                                    if point_in_poly(&p, &(clip_polylines[a].vertices)) {
                                        if !is_clip_part_a_hole[a] {
                                            out = true;
                                        } else {
                                            out = false;
                                        }
                                    }
                                }
                            }
                            tx.send((record_num, out)).unwrap();
                        }
                    });
                }

                let mut output_feature: Vec<bool> = vec![false; input.num_records];
                for r in 0..input.num_records {
                    let (record_num, out) = rx.recv().unwrap();
                    if out {
                        output_feature[record_num] = true;
                    }
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (input.num_records - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                let mut fid = 1;
                for r in 0..input.num_records {
                    if output_feature[r] {
                        let record = input.get_record(r).clone();
                        output.add_record(record);

                        if table_contains_fid {
                            let mut att = input.attributes.get_record(r).clone();
                            att[fid_field_num] = FieldData::Int(fid);
                            fid += 1;
                            output.attributes.add_record(att, false);
                        } else {
                            output
                                .attributes
                                .add_record(input.attributes.get_record(r).clone(), false)
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (input.num_records - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            ShapeType::MultiPoint => {
                let clip_polylines = Arc::new(clip_polylines);
                let mut fid = 1;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num).clone();
                    let record = Arc::new(record);
                    let num_points = record.num_points as usize;
                    for tid in 0..num_procs {
                        let record = record.clone();
                        let clip_bb = clip_bb.clone();
                        let clip_polylines = clip_polylines.clone();
                        let is_clip_part_a_hole = is_clip_part_a_hole.clone();
                        let tx = tx.clone();
                        thread::spawn(move || {
                            let mut p: Point2D;
                            let mut out: bool;
                            for point_num in (0..num_points).filter(|r| r % num_procs == tid) {
                                p = record.points[point_num].clone();
                                out = false;
                                for a in 0..clip_polylines.len() {
                                    if clip_bb[a].is_point_in_box(p.x, p.y) {
                                        if point_in_poly(&p, &(clip_polylines[a].vertices)) {
                                            if !is_clip_part_a_hole[a] {
                                                out = true;
                                            } else {
                                                out = false;
                                            }
                                        }
                                    }
                                }
                                tx.send((point_num, out)).unwrap();
                            }
                        });
                    }
                    let mut output_feature: Vec<bool> = vec![false; num_points];
                    let mut num_out_pnts = 0;
                    for _ in 0..num_points {
                        let (point_num, out) = rx.recv().unwrap();
                        if out {
                            output_feature[point_num] = true;
                            num_out_pnts += 1;
                        }
                    }
                    if num_out_pnts > 0 {
                        let mut sfg = ShapefileGeometry::new(input.header.shape_type);
                        match input.header.shape_type.dimension() {
                            ShapeTypeDimension::XY => {
                                for point_num in 0..num_points {
                                    if output_feature[point_num] {
                                        sfg.add_point((record.points[point_num]).clone());
                                    }
                                }
                            }
                            ShapeTypeDimension::Measure => {
                                for point_num in 0..num_points {
                                    if output_feature[point_num] {
                                        sfg.add_pointm(
                                            (record.points[point_num]).clone(),
                                            record.m_array[point_num],
                                        );
                                    }
                                }
                            }
                            ShapeTypeDimension::Z => {
                                for point_num in 0..num_points {
                                    if output_feature[point_num] {
                                        sfg.add_pointz(
                                            (record.points[point_num]).clone(),
                                            record.m_array[point_num],
                                            record.z_array[point_num],
                                        );
                                    }
                                }
                            }
                        }
                        output.add_record(sfg);
                        if table_contains_fid {
                            let mut att = input.attributes.get_record(record_num).clone();
                            att[fid_field_num] = FieldData::Int(fid);
                            fid += 1;
                            output.attributes.add_record(att, false);
                        } else {
                            output
                                .attributes
                                .add_record(input.attributes.get_record(record_num).clone(), false)
                        }
                    }

                    if verbose {
                        progress = (100.0_f64 * record_num as f64 / (input.num_records - 1) as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            ShapeType::PolyLine => {
                // The polyline splitline method makes keeping track of
                // measure and z data for split lines difficult. Regardless
                // of the input shapefile dimension, the output will be XY only.
                output.header.shape_type = ShapeType::PolyLine;

                // First, find the itersection points and split the input features up at these sites
                let mut features_bb: Vec<BoundingBox> = vec![];
                let mut features_polylines: Vec<Polyline> = vec![];
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    for part in 0..record.num_parts as usize {
                        first_point_in_part = record.parts[part] as usize;
                        last_point_in_part = if part < record.num_parts as usize - 1 {
                            record.parts[part + 1] as usize - 1
                        } else {
                            record.num_points as usize - 1
                        };
                        features_bb.push(BoundingBox::from_points(
                            &(record.points[first_point_in_part..=last_point_in_part]),
                        ));

                        // Create a polyline from the part
                        let mut pl = Polyline::new(
                            &(record.points[first_point_in_part..=last_point_in_part]),
                            record_num,
                        );
                        pl.source_file = 2; // 1 will mean the clip file and 2 will mean the input file.
                        features_polylines.push(pl);
                    }
                }

                // hunt for intersections in the overlapping bounding boxes
                for record_num1 in 0..features_polylines.len() {
                    for record_num2 in 0..clip_polylines.len() {
                        if features_bb[record_num1].overlaps(clip_bb[record_num2]) {
                            // find any intersections between the polylines
                            find_split_points_at_line_intersections(
                                &mut features_polylines[record_num1],
                                &mut clip_polylines[record_num2],
                            );
                        }
                    }
                }

                let mut fid = 1i32;
                for record_num1 in 0..features_polylines.len() {
                    let split_lines = features_polylines[record_num1].split();
                    for j in 0..split_lines.len() {
                        if split_lines[j].len() > 1 {
                            let mut out = false;
                            let p = Point2D::midpoint(&split_lines[j][0], &split_lines[j][1]); // lies along the polyline
                            for record_num2 in 0..clip_polylines.len() {
                                if clip_bb[record_num2].is_point_in_box(p.x, p.y) {
                                    if point_in_poly(&p, &(clip_polylines[record_num2].vertices)) {
                                        if !is_clip_part_a_hole[record_num2] {
                                            out = true;
                                        } else {
                                            out = false;
                                        }
                                    }
                                }
                            }
                            if out {
                                // output the polylines
                                let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                                sfg.add_part(&(split_lines[j].vertices));
                                output.add_record(sfg);

                                if table_contains_fid {
                                    let mut att = input
                                        .attributes
                                        .get_record(features_polylines[record_num1].id)
                                        .clone();
                                    att[fid_field_num] = FieldData::Int(fid);
                                    fid += 1;
                                    output.attributes.add_record(att, false);
                                } else {
                                    output.attributes.add_record(
                                        input
                                            .attributes
                                            .get_record(features_polylines[record_num1].id)
                                            .clone(),
                                        false,
                                    )
                                }
                            }
                        }
                    }

                    if verbose {
                        progress = (100.0_f64 * (record_num1 + 1) as f64
                            / features_polylines.len() as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            ShapeType::Polygon => {
                // The polyline splitline method makes keeping track of
                // measure and z data for split lines difficult. Regardless
                // of the input shapefile dimension, the output will be XY only.
                output.header.shape_type = ShapeType::Polygon;

                // output.header.shape_type = ShapeType::PolyLine;
                let mut fid = 1i32;

                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    let mut polygons: Vec<Polyline> = vec![];
                    let mut is_part_a_hole: Vec<bool> = vec![];
                    let mut features_bb: Vec<BoundingBox> = vec![];
                    let mut clip_feature_overlaps = vec![false; clip_polylines.len()];
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
                            part,
                        );
                        pl.source_file = 2;
                        let plbb = pl.get_bounding_box();
                        let mut overlaps_with_clip = false;
                        for i in 0..clip_polylines.len() {
                            if plbb.overlaps(clip_bb[i]) {
                                if poly_overlaps_poly(&(pl.vertices), &(clip_polylines[i].vertices))
                                {
                                    overlaps_with_clip = true;
                                    clip_feature_overlaps[i] = true;
                                    // break;
                                }
                            }
                        }
                        if overlaps_with_clip {
                            features_bb.push(pl.get_bounding_box());
                            polygons.push(pl);

                            if record.is_hole(part as i32) {
                                is_part_a_hole.push(true);
                            } else {
                                is_part_a_hole.push(false);
                            }
                        }
                    }
                    if polygons.len() > 0 {
                        // it overlaps with at least one clipping feature
                        for i in 0..clip_polylines.len() {
                            if clip_feature_overlaps[i] {
                                polygons.push(clip_polylines[i].clone());
                                is_part_a_hole.push(is_clip_part_a_hole[i]);
                                features_bb.push(clip_bb[i].clone());
                            }
                        }

                        // convert to fixed precision
                        let num_decimals = 6;
                        let precision = EPSILON; // 1f64 / num_decimals as f64;

                        let mut p: Point2D;
                        for i in 0..polygons.len() {
                            for j in 0..polygons[i].len() {
                                p = polygons[i][j];
                                polygons[i].vertices[j] = p.fix_precision(num_decimals);
                            }
                        }

                        // Break the polygons up into lines at junction points.
                        let dimensions = 2;
                        let capacity_per_node = 64;
                        let mut snap_tree =
                            KdTree::new_with_capacity(dimensions, capacity_per_node);
                        let mut p: Point2D;
                        for i in 0..polygons.len() {
                            for j in 0..polygons[i].len() {
                                p = polygons[i][j];
                                snap_tree.add([p.x, p.y], (i, j)).unwrap();
                            }
                        }

                        let mut num_neighbours: Vec<Vec<u8>> = Vec::with_capacity(polygons.len());
                        for i in 0..polygons.len() {
                            let mut line_num_neighbours = Vec::with_capacity(polygons[i].len());
                            for j in 0..polygons[i].len() {
                                p = polygons[i][j];
                                let ret = snap_tree
                                    .within(&[p.x, p.y], precision, &squared_euclidean)
                                    .unwrap();

                                let mut n = 0u8;
                                for a in 0..ret.len() {
                                    let k = ret[a].1;
                                    if k.0 != i {
                                        n += 1u8;
                                    }
                                }
                                line_num_neighbours.push(n);
                            }

                            num_neighbours.push(line_num_neighbours);
                        }

                        let mut features_polylines: Vec<Polyline> = vec![];
                        let mut id: usize;
                        for i in 0..polygons.len() {
                            id = polygons[i].id;
                            let mut pl = Polyline::new_empty(id);
                            pl.vertices.push(polygons[i][0]);
                            pl.source_file = polygons[i].source_file;
                            for j in 1..polygons[i].len() {
                                if num_neighbours[i][j] > 1
                                    || num_neighbours[i][j] == 1 && num_neighbours[i][j - 1] == 0
                                    || num_neighbours[i][j] == 0 && num_neighbours[i][j - 1] == 1
                                {
                                    // it's a junction, split the poly
                                    pl.vertices.push(polygons[i][j]);
                                    features_polylines.push(pl.clone());
                                    // id += 1;
                                    pl = Polyline::new_empty(id);
                                    pl.vertices.push(polygons[i][j]);
                                    pl.source_file = polygons[i].source_file;
                                } else {
                                    pl.vertices.push(polygons[i][j]);
                                }
                            }
                            features_polylines.push(pl.clone());
                        }

                        // Find duplicate polylines and remove them
                        let mut duplicate = vec![false; features_polylines.len()];
                        for i in 0..features_polylines.len() {
                            if !duplicate[i] {
                                for j in (i + 1)..features_polylines.len() {
                                    if features_polylines[i] == features_polylines[j] {
                                        duplicate[j] = true;
                                    }
                                }
                            }
                        }
                        for i in (0..features_polylines.len()).rev() {
                            if duplicate[i] {
                                features_polylines.remove(i);
                            }
                        }

                        // hunt for intersections
                        features_bb = Vec::with_capacity(features_polylines.len());
                        for i in 0..features_polylines.len() {
                            features_bb.push(features_polylines[i].get_bounding_box());
                        }

                        let mut polylines = vec![];
                        let mut lengths = vec![];
                        let mut line_length: f64;
                        for i in 0..features_polylines.len() {
                            let mut pl = features_polylines[i].clone();
                            for j in i + 1..features_polylines.len() {
                                if features_bb[i].overlaps(features_bb[j]) {
                                    // find any intersections between the polylines
                                    find_split_points_at_line_intersections(
                                        &mut pl,
                                        &mut (features_polylines[j]),
                                    );
                                }
                            }
                            let split_lines = pl.split();
                            for j in 0..split_lines.len() {
                                line_length = split_lines[j].length();
                                if line_length > precision {
                                    polylines.push(split_lines[j].clone());
                                    lengths.push(line_length);
                                }
                            }
                        }

                        // convert to fixed precision
                        for i in 0..polylines.len() {
                            for j in 0..polylines[i].len() {
                                p = polylines[i][j];
                                polylines[i].vertices[j] = p.fix_precision(num_decimals);
                            }
                        }

                        // Find duplicate polylines and remove them
                        let mut duplicate = vec![false; polylines.len()];
                        for i in 0..polylines.len() {
                            if !duplicate[i] {
                                for j in (i + 1)..polylines.len() {
                                    if polylines[i] == polylines[j] {
                                        duplicate[j] = true;
                                    }
                                }
                            }
                        }
                        for i in (0..polylines.len()).rev() {
                            if duplicate[i] {
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

                        // Find the neighbours of each endnode and check for dangling arcs
                        // and self-closing arcs which form single-line polys.
                        let mut is_acyclic_arc = vec![false; polylines.len()];
                        let mut node_angles: Vec<Vec<f64>> = vec![vec![]; num_endnodes];
                        let mut heading: f64;
                        for i in 0..polylines.len() {
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
                                    let index = *ret[a].1;
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
                                    .within(&[p2.x, p2.y], EPSILON, &squared_euclidean)
                                    .unwrap();
                                if ret.len() == 1 {
                                    is_acyclic_arc[i] = true;
                                } else {
                                    p3 = polylines[i][polylines[i].len() - 2];
                                    for a in 0..ret.len() {
                                        let index = *ret[a].1;
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

                        let mut node1: usize;
                        let mut node2: usize;
                        for i in 0..polylines.len() {
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

                        /*
                        ////////////////////////////////////////////////////////////////////
                        if record_num == 3248 {
                            let mut output2 = Shapefile::initialize_using_file(
                                &output_file,
                                &input,
                                input.header.shape_type,
                                true,
                            )?;
                            output2.header.shape_type = ShapeType::PolyLine;
                            for i in 0..polylines.len() {
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
                        }
                        ////////////////////////////////////////////////////////////////////
                        */

                        // if record_num == 3248 {
                        //     println!(
                        //         "First node neighbours for 2: {:?}",
                        //         endnodes[first_node_id(2)]
                        //     );
                        //     println!(
                        //         "First node neighbour angles for 2: {:?}",
                        //         node_angles[first_node_id(2)]
                        //     );
                        //     println!(
                        //         "Last node neighbours for 2: {:?}",
                        //         endnodes[last_node_id(2)]
                        //     );
                        //     println!(
                        //         "Last node neighbour angles for 2: {:?}",
                        //         node_angles[last_node_id(2)]
                        //     );

                        //     println!(
                        //         "First node neighbours for 6: {:?}",
                        //         endnodes[first_node_id(6)]
                        //     );
                        //     println!(
                        //         "First node neighbour angles for 6: {:?}",
                        //         node_angles[first_node_id(6)]
                        //     );
                        //     println!(
                        //         "Last node neighbours for 6: {:?}",
                        //         endnodes[last_node_id(6)]
                        //     );
                        //     println!(
                        //         "Last node neighbour angles for 6: {:?}",
                        //         node_angles[last_node_id(6)]
                        //     );
                        // }

                        //////////////////////////////////////////////////////////////////////////////////////////
                        // This is the main part of the anaysis. It is responsible for rebuilding the polygons. //
                        //////////////////////////////////////////////////////////////////////////////////////////
                        let mut bb: Vec<BoundingBox> = vec![];
                        let mut current_node: usize;
                        let mut neighbour_node: usize;
                        let mut num_neighbours: usize;
                        let mut existing_polygons = HashSet::new();
                        let mut existing_hull = HashSet::new();
                        let mut feature_geometries: Vec<ShapefileGeometry> = vec![];
                        let mut hull_geometries: Vec<ShapefileGeometry> = vec![];
                        let mut p: Point2D;
                        let mut max_val: f64;
                        let mut max_val_index: usize;
                        let mut k: usize;
                        let mut num_vertices: usize;
                        let mut other_side: usize;
                        let mut target_found: bool;
                        let mut assigned = vec![0usize; polylines.len()];
                        let mut is_clockwise: bool;
                        let mut overlaps_with_clip: bool;
                        let mut overlaps_with_poly: bool;
                        let mut last_index: usize;
                        for i in 0..polylines.len() {
                            // println!("i={}", i);
                            if !is_acyclic_arc[i] && assigned[i] < 2 {
                                // && polylines[i].source_file == 2 {

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
                                        if prev[other_side] != num_endnodes
                                            && other_side != target_node
                                        {
                                            break;
                                        }
                                        prev[neighbour_node] = current_node;
                                        prev[other_side] = neighbour_node;
                                        if neighbour_node == target_node
                                            || other_side == target_node
                                        {
                                            target_found = true;
                                            break;
                                        }
                                        current_node = other_side;
                                    } else if num_neighbours == 1 {
                                        // There's only one way forward, so take it.
                                        neighbour_node = endnodes[current_node][0];
                                        other_side = get_other_endnode(neighbour_node);
                                        if prev[other_side] != num_endnodes
                                            && other_side != target_node
                                        {
                                            break;
                                        }
                                        prev[neighbour_node] = current_node;
                                        prev[other_side] = neighbour_node;
                                        if neighbour_node == target_node
                                            || other_side == target_node
                                        {
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
                                    let mut backlinks: Vec<
                                        usize,
                                    > = vec![];
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
                                    let mut vertices: Vec<
                                        Point2D,
                                    > = Vec::with_capacity(num_vertices);
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

                                    // if record_num == 3248 {
                                    //     if i == 2 {
                                    //         println!("i={}", i);
                                    //         println!("lines={:?}", lines);
                                    //         println!("backlinks={:?}", backlinks);
                                    //         println!("is clockwise={}", is_clockwise);
                                    //     } else if i == 6 {
                                    //         println!("i={}", i);
                                    //         println!("lines={:?}", lines);
                                    //         println!("backlinks={:?}", backlinks);
                                    //         println!("is clockwise={}", is_clockwise);
                                    //     }
                                    // }

                                    // don't add the same poly more than once
                                    let mut test_poly = lines.clone();
                                    test_poly.sort();
                                    if !existing_polygons.contains(&test_poly) {
                                        if is_clockwise {
                                            existing_polygons.insert(test_poly);
                                            for a in 0..lines.len() {
                                                assigned[lines[a]] += 1;
                                            }
                                            if vertices.len() > 3 {
                                                // a minimum of four points are needed to form a closed polygon (triangle)
                                                if !vertices[0]
                                                    .nearly_equals(&vertices[vertices.len() - 1])
                                                {
                                                    p = vertices[0];
                                                    if vertices[0]
                                                        .distance(&vertices[vertices.len() - 1])
                                                        < precision
                                                    {
                                                        last_index = vertices.len() - 1;
                                                        vertices[last_index] = p;
                                                    } else {
                                                        vertices.push(p);
                                                    }
                                                }
                                                p = interior_point(&vertices);
                                                // if record_num == 3248 && i == 2 {
                                                //     println!("p={:?}", p);
                                                //     println!("vertices={:?}", vertices);
                                                // }
                                                overlaps_with_clip = false;
                                                overlaps_with_poly = false;
                                                for j in 0..polygons.len() {
                                                    if point_in_poly(&p, &(polygons[j].vertices)) {
                                                        if polygons[j].source_file == 1 {
                                                            if !is_part_a_hole[j] {
                                                                overlaps_with_clip = true;
                                                            } else {
                                                                overlaps_with_clip = false;
                                                                break;
                                                            }
                                                        } else {
                                                            overlaps_with_poly = true;
                                                        }
                                                    }
                                                }
                                                if overlaps_with_clip && overlaps_with_poly {
                                                    // output the polygon
                                                    let mut sfg =
                                                        ShapefileGeometry::new(ShapeType::Polygon);
                                                    sfg.add_part(&vertices);
                                                    bb.push(sfg.get_bounding_box());
                                                    // output.add_record(sfg);

                                                    // if table_contains_fid {
                                                    //     let mut att = input
                                                    //         .attributes
                                                    //         .get_record(record_num)
                                                    //         .clone();
                                                    //     att[fid_field_num] = FieldData::Int(fid);
                                                    //     fid += 1;
                                                    //     output.attributes.add_record(att, false);
                                                    // } else {
                                                    //     output.attributes.add_record(
                                                    //         input
                                                    //             .attributes
                                                    //             .get_record(record_num)
                                                    //             .clone(),
                                                    //         false,
                                                    //     )
                                                    // }
                                                    feature_geometries.push(sfg);
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
                                                // a minimum of four points are needed to form a closed polygon (triangle)
                                                if !vertices[0]
                                                    .nearly_equals(&vertices[vertices.len() - 1])
                                                {
                                                    p = vertices[0];
                                                    if vertices[0]
                                                        .distance(&vertices[vertices.len() - 1])
                                                        < precision
                                                    {
                                                        last_index = vertices.len() - 1;
                                                        vertices[last_index] = p;
                                                    } else {
                                                        vertices.push(p);
                                                    }
                                                }
                                                p = interior_point(&vertices);
                                                overlaps_with_clip = false;
                                                overlaps_with_poly = false;
                                                for j in 0..polygons.len() {
                                                    if point_in_poly(&p, &(polygons[j].vertices)) {
                                                        if polygons[j].source_file == 1 {
                                                            if !is_part_a_hole[j] {
                                                                overlaps_with_clip = true;
                                                            } else {
                                                                overlaps_with_clip = false;
                                                                break;
                                                            }
                                                        } else {
                                                            overlaps_with_poly = true;
                                                        }
                                                    }
                                                }
                                                if overlaps_with_clip && overlaps_with_poly {
                                                    let mut sfg =
                                                        ShapefileGeometry::new(ShapeType::Polygon);
                                                    sfg.add_part(&vertices);
                                                    hull_geometries.push(sfg);
                                                }
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
                                            if prev[other_side] != num_endnodes
                                                && other_side != target_node
                                            {
                                                break;
                                            }
                                            prev[neighbour_node] = current_node;
                                            prev[other_side] = neighbour_node;
                                            if neighbour_node == target_node
                                                || other_side == target_node
                                            {
                                                target_found = true;
                                                break;
                                            }
                                            current_node = other_side;
                                        } else if num_neighbours == 1 {
                                            // There's only one way forward, so take it.
                                            neighbour_node = endnodes[current_node][0];
                                            other_side = get_other_endnode(neighbour_node);
                                            if prev[other_side] != num_endnodes
                                                && other_side != target_node
                                            {
                                                break;
                                            }
                                            prev[neighbour_node] = current_node;
                                            prev[other_side] = neighbour_node;
                                            if neighbour_node == target_node
                                                || other_side == target_node
                                            {
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
                                        let mut lines: Vec<
                                            usize,
                                        > = vec![];
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
                                                if vertices.len() > 3 {
                                                    // a minimum of four points are needed to form a closed polygon (triangle)
                                                    if !vertices[0].nearly_equals(
                                                        &vertices[vertices.len() - 1],
                                                    ) {
                                                        p = vertices[0];
                                                        if vertices[0]
                                                            .distance(&vertices[vertices.len() - 1])
                                                            < precision
                                                        {
                                                            last_index = vertices.len() - 1;
                                                            vertices[last_index] = p;
                                                        } else {
                                                            vertices.push(p);
                                                        }
                                                    }
                                                    p = interior_point(&vertices);
                                                    overlaps_with_clip = false;
                                                    overlaps_with_poly = false;
                                                    for j in 0..polygons.len() {
                                                        if point_in_poly(
                                                            &p,
                                                            &(polygons[j].vertices),
                                                        ) {
                                                            if polygons[j].source_file == 1 {
                                                                if !is_part_a_hole[j] {
                                                                    overlaps_with_clip = true;
                                                                } else {
                                                                    overlaps_with_clip = false;
                                                                    break;
                                                                }
                                                            } else {
                                                                overlaps_with_poly = true;
                                                            }
                                                        }
                                                    }
                                                    if overlaps_with_clip && overlaps_with_poly {
                                                        // output the polygon
                                                        let mut sfg = ShapefileGeometry::new(
                                                            ShapeType::Polygon,
                                                        );
                                                        sfg.add_part(&vertices);
                                                        bb.push(sfg.get_bounding_box());
                                                        // output.add_record(sfg);

                                                        // if table_contains_fid {
                                                        //     let mut att = input
                                                        //         .attributes
                                                        //         .get_record(record_num)
                                                        //         .clone();
                                                        //     att[fid_field_num] =
                                                        //         FieldData::Int(fid);
                                                        //     fid += 1;
                                                        //     output
                                                        //         .attributes
                                                        //         .add_record(att, false);
                                                        // } else {
                                                        //     output.attributes.add_record(
                                                        //         input
                                                        //             .attributes
                                                        //             .get_record(record_num)
                                                        //             .clone(),
                                                        //         false,
                                                        //     )
                                                        // }
                                                        feature_geometries.push(sfg);
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
                                                    // a minimum of four points are needed to form a closed polygon (triangle)
                                                    if !vertices[0].nearly_equals(
                                                        &vertices[vertices.len() - 1],
                                                    ) {
                                                        p = vertices[0];
                                                        if vertices[0]
                                                            .distance(&vertices[vertices.len() - 1])
                                                            < precision
                                                        {
                                                            last_index = vertices.len() - 1;
                                                            vertices[last_index] = p;
                                                        } else {
                                                            vertices.push(p);
                                                        }
                                                    }
                                                    p = interior_point(&vertices);
                                                    overlaps_with_clip = false;
                                                    overlaps_with_poly = false;
                                                    for j in 0..polygons.len() {
                                                        if point_in_poly(
                                                            &p,
                                                            &(polygons[j].vertices),
                                                        ) {
                                                            if polygons[j].source_file == 1 {
                                                                if !is_part_a_hole[j] {
                                                                    overlaps_with_clip = true;
                                                                } else {
                                                                    overlaps_with_clip = false;
                                                                    break;
                                                                }
                                                            } else {
                                                                overlaps_with_poly = true;
                                                            }
                                                        }
                                                    }
                                                    if overlaps_with_clip && overlaps_with_poly {
                                                        let mut sfg = ShapefileGeometry::new(
                                                            ShapeType::Polygon,
                                                        );
                                                        sfg.add_part(&vertices);
                                                        hull_geometries.push(sfg);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // can any of the hulls be added as holes in other polygons?
                        for a in 0..hull_geometries.len() {
                            let hull_bb = hull_geometries[a].get_bounding_box();
                            for b in 0..bb.len() {
                                if hull_bb.entirely_contained_within(bb[b]) {
                                    if is_clockwise_order(&hull_geometries[a].points) {
                                        hull_geometries[a].points.reverse();
                                    }
                                    if feature_geometries[b].num_parts > 1 {
                                        if poly_in_poly(
                                            &(hull_geometries[a].points),
                                            &(feature_geometries[b].points
                                                [0..feature_geometries[b].parts[1] as usize]),
                                        ) {
                                            feature_geometries[b]
                                                .add_part(&(hull_geometries[a].points));
                                        }
                                    } else {
                                        if poly_in_poly(
                                            &(hull_geometries[a].points),
                                            &(feature_geometries[b].points),
                                        ) {
                                            feature_geometries[b]
                                                .add_part(&(hull_geometries[a].points));
                                        }
                                    }
                                }
                            }
                        }

                        for a in 0..feature_geometries.len() {
                            output.add_record(feature_geometries[a].clone());
                            if table_contains_fid {
                                let mut att = input.attributes.get_record(record_num).clone();
                                att[fid_field_num] = FieldData::Int(fid);
                                fid += 1;
                                output.attributes.add_record(att, false);
                            } else {
                                output.attributes.add_record(
                                    input.attributes.get_record(record_num).clone(),
                                    false,
                                )
                            }
                        }

                        // // can any of the hulls be added as holes in other polygons?
                        // for a in 0..hull_geometries.len() {
                        //     let hull_bb = hull_geometries[a].get_bounding_box();
                        //     for b in 0..bb.len() {
                        //         if hull_bb.entirely_contained_within(bb[b]) {
                        //             if output.records[b].num_parts > 1 {
                        //                 if poly_in_poly(
                        //                     &(hull_geometries[a].points),
                        //                     &(output.records[b].points
                        //                         [0..output.records[b].parts[1] as usize]),
                        //                 ) {
                        //                     output.records[b]
                        //                         .add_part(&(hull_geometries[a].points));
                        //                 }
                        //             } else {
                        //                 if poly_in_poly(
                        //                     &(hull_geometries[a].points),
                        //                     &(output.records[b].points),
                        //                 ) {
                        //                     output.records[b]
                        //                         .add_part(&(hull_geometries[a].points));
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    }

                    if verbose {
                        progress = (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid input data ShapeType.",
                ));
            }
        }

        /*
                let mut polygons: Vec<Polyline> = vec![];
                let mut features_bb: Vec<BoundingBox> = vec![];
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
                        let plbb = pl.get_bounding_box();
                        let mut overlaps_with_clip = false;
                        for record_num2 in 0..clip_polylines.len() {
                            if plbb.overlaps(clip_bb[record_num2]) {
                                if poly_overlaps_poly(
                                    &(pl.vertices),
                                    &(clip_polylines[record_num2].vertices),
                                ) {
                                    overlaps_with_clip = true;
                                    break;
                                }
                            }
                        }
                        if overlaps_with_clip {
                            features_bb.push(pl.get_bounding_box());
                            pl.source_file = 2;
                            polygons.push(pl);
                        }
                    }
                }

                for record_num in 0..clip_polylines.len() {
                    polygons.push(clip_polylines[record_num].clone());
                    features_bb.push(clip_bb[record_num].clone());
                }

                // convert to fixed precision
                let num_decimals = 6;
                let precision = 1f64 / num_decimals as f64;
                // let mut p: Point2D;
                // for i in 0..polygons.len() {
                //     for j in 0..polygons[i].len() {
                //         p = polygons[i][j];
                //         polygons[i].vertices[j] = p.fix_precision(num_decimals);
                //     }
                // }

                // Break the polygons up into lines at junction points.
                let dimensions = 2;
                let capacity_per_node = 64;
                let mut snap_tree = KdTree::new_with_capacity(dimensions, capacity_per_node);
                let mut p: Point2D;
                println!("Creating tree...");
                for i in 0..polygons.len() {
                    for j in 0..polygons[i].len() {
                        p = polygons[i][j];
                        snap_tree.add([p.x, p.y], (i, j)).unwrap();
                    }
                }

                let mut num_neighbours: Vec<Vec<u8>> = Vec::with_capacity(polygons.len());
                for i in 0..polygons.len() {
                    let mut line_num_neighbours = Vec::with_capacity(polygons[i].len());
                    for j in 0..polygons[i].len() {
                        p = polygons[i][j];
                        let ret = snap_tree
                            .within(&[p.x, p.y], EPSILON, &squared_euclidean)
                            .unwrap();

                        let mut n = 0u8;
                        for a in 0..ret.len() {
                            let k = ret[a].1;
                            if k.0 != i {
                                n += 1u8;
                            }
                        }
                        line_num_neighbours.push(n);
                    }

                    num_neighbours.push(line_num_neighbours);
                }

                let mut features_polylines2: Vec<Polyline> = vec![];
                let mut id: usize;
                for i in 0..polygons.len() {
                    id = polygons[i].id;
                    let mut pl = Polyline::new_empty(id);
                    pl.vertices.push(polygons[i][0]);
                    pl.source_file = polygons[i].source_file;
                    for j in 1..polygons[i].len() {
                        if num_neighbours[i][j] > 1
                            || num_neighbours[i][j] == 1 && num_neighbours[i][j - 1] == 0
                            || num_neighbours[i][j] == 0 && num_neighbours[i][j - 1] == 1
                        {
                            // it's a junction, split the poly
                            pl.vertices.push(polygons[i][j]);
                            features_polylines2.push(pl.clone());
                            // id += 1;
                            pl = Polyline::new_empty(id);
                            pl.vertices.push(polygons[i][j]);
                            pl.source_file = polygons[i].source_file;
                        } else {
                            pl.vertices.push(polygons[i][j]);
                        }
                    }
                    features_polylines2.push(pl.clone());
                }

                // Find duplicate polylines and remove them
                let mut duplicate = vec![false; features_polylines2.len()];
                for i in 0..features_polylines2.len() {
                    if !duplicate[i] {
                        for j in (i + 1)..features_polylines2.len() {
                            if features_polylines2[i] == features_polylines2[j] {
                                duplicate[j] = true;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * (i + 1) as f64 / features_polylines2.len() as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Searching for duplicate lines: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
                for i in (0..features_polylines2.len()).rev() {
                    if duplicate[i] {
                        features_polylines2.remove(i);
                    }
                }

                /*
                ////////////////////////////////////////////////////////////////////
                output.header.shape_type = ShapeType::PolyLine;
                let mut fid = 1i32;
                for i in 0..features_polylines2.len() {
                    // output the polygon
                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&(features_polylines2[i].vertices));
                    output.add_record(sfg);

                    output
                        .attributes
                        .add_record(vec![FieldData::Int(fid)], false);
                    fid += 1;
                }

                if verbose {
                    println!("Saving data...")
                };
                let _ = match output.write() {
                    Ok(_) => if verbose {
                        println!("Output file written")
                    },
                    Err(e) => return Err(e),
                };
                return Ok(());
                ////////////////////////////////////////////////////////////////////
                */

                features_bb = Vec::with_capacity(features_polylines2.len());
                for i in 0..features_polylines2.len() {
                    features_bb.push(features_polylines2[i].get_bounding_box());
                }

                // hunt for intersections in the overlapping bounding boxes
                let mut polylines = vec![];
                let mut lengths = vec![];
                let mut line_length: f64;
                for record_num1 in 0..features_polylines2.len() {
                    let mut pl = features_polylines2[record_num1].clone();
                    for record_num2 in record_num1 + 1..features_polylines2.len() {
                        if features_bb[record_num1].overlaps(features_bb[record_num2]) {
                            // find any intersections between the polylines
                            find_split_points_at_line_intersections(
                                &mut pl,
                                &mut (features_polylines2[record_num2]),
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
                        progress = (100.0_f64 * (record_num1 + 1) as f64
                            / features_polylines2.len() as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Finding line intersections: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                // Find duplicate polylines and remove them
                let mut duplicate = vec![false; polylines.len()];
                for i in 0..polylines.len() {
                    if !duplicate[i] {
                        for j in (i + 1)..polylines.len() {
                            if polylines[i] == polylines[j] {
                                duplicate[j] = true;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                        if progress != old_progress {
                            println!("Searching for duplicate lines: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
                for i in (0..polylines.len()).rev() {
                    if duplicate[i] {
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
                let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);
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
                            .within(&[p2.x, p2.y], EPSILON, &squared_euclidean)
                            .unwrap();
                        if ret.len() == 1 {
                            is_acyclic_arc[i] = true;
                        } else {
                            p3 = polylines[i][polylines[i].len() - 2];
                            for a in 0..ret.len() {
                                let index = *ret[a].1;
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

                    if verbose {
                        progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                        if progress != old_progress {
                            println!("Finding node vertices: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                // Find connecting arcs. These are arcs that don't form loops. The only way to
                // travel from one endnode to the other is to travel through the polyline. They
                // can be safely removed from the graph.
                if verbose {
                    println!("Finding acyclic arcs");
                }
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

                let mut bb: Vec<BoundingBox> = vec![];
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
                let mut overlaps_with_clip: bool;
                let mut last_index: usize;
                let mut fid = 1;
                for i in 0..polylines.len() {
                    // println!("i={}", i);
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
                                    if vertices.len() > 3 {
                                        // a minimum of four points are needed to form a closed polygon (triangle)
                                        if !vertices[0].nearly_equals(&vertices[vertices.len() - 1])
                                        {
                                            p = vertices[0];
                                            if vertices[0].distance(&vertices[vertices.len() - 1])
                                                < precision
                                            {
                                                last_index = vertices.len() - 1;
                                                vertices[last_index] = p;
                                            } else {
                                                vertices.push(p);
                                                // println!("1 Problem at {} {:?}", i, vertices);
                                            }
                                        }
                                        p = interior_point(&vertices);
                                        overlaps_with_clip = false;
                                        for record_num2 in 0..clip_polylines.len() {
                                            if point_in_poly(
                                                &p,
                                                &(clip_polylines[record_num2].vertices),
                                            ) {
                                                overlaps_with_clip = true;
                                                break;
                                            }
                                        }
                                        if overlaps_with_clip {
                                            // output the polygon
                                            let mut sfg =
                                                ShapefileGeometry::new(ShapeType::Polygon);
                                            sfg.add_part(&vertices);
                                            bb.push(sfg.get_bounding_box());
                                            output.add_record(sfg);

                                            output
                                                .attributes
                                                .add_record(vec![FieldData::Int(fid)], false);
                                            fid += 1;
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
                                        // a minimum of four points are needed to form a closed polygon (triangle)
                                        if !vertices[0].nearly_equals(&vertices[vertices.len() - 1])
                                        {
                                            p = vertices[0];
                                            if vertices[0].distance(&vertices[vertices.len() - 1])
                                                < precision
                                            {
                                                last_index = vertices.len() - 1;
                                                vertices[last_index] = p;
                                            } else {
                                                vertices.push(p);
                                                // println!("1 Problem at hull {} {:?}", i, vertices);
                                            }
                                        }
                                        p = interior_point(&vertices);
                                        overlaps_with_clip = false;
                                        for record_num2 in 0..clip_polylines.len() {
                                            if point_in_poly(
                                                &p,
                                                &(clip_polylines[record_num2].vertices),
                                            ) {
                                                overlaps_with_clip = true;
                                                break;
                                            }
                                        }
                                        if overlaps_with_clip {
                                            let mut sfg =
                                                ShapefileGeometry::new(ShapeType::Polygon);
                                            sfg.add_part(&vertices);
                                            hull_geometries.push(sfg);
                                        }
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
                                    if prev[other_side] != num_endnodes && other_side != target_node
                                    {
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
                                    if prev[other_side] != num_endnodes && other_side != target_node
                                    {
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
                                        if vertices.len() > 3 {
                                            // a minimum of four points are needed to form a closed polygon (triangle)
                                            if !vertices[0]
                                                .nearly_equals(&vertices[vertices.len() - 1])
                                            {
                                                p = vertices[0];
                                                if vertices[0]
                                                    .distance(&vertices[vertices.len() - 1])
                                                    < precision
                                                {
                                                    last_index = vertices.len() - 1;
                                                    vertices[last_index] = p;
                                                } else {
                                                    vertices.push(p);
                                                    // println!("2 Problem at {} {:?}", i, vertices);
                                                }
                                            }
                                            overlaps_with_clip = false;
                                            p = interior_point(&vertices);
                                            for record_num2 in 0..clip_polylines.len() {
                                                if point_in_poly(
                                                    &p,
                                                    &(clip_polylines[record_num2].vertices),
                                                ) {
                                                    overlaps_with_clip = true;
                                                    break;
                                                }
                                            }
                                            if overlaps_with_clip {
                                                // output the polygon
                                                let mut sfg =
                                                    ShapefileGeometry::new(ShapeType::Polygon);
                                                sfg.add_part(&vertices);
                                                bb.push(sfg.get_bounding_box());
                                                output.add_record(sfg);

                                                output
                                                    .attributes
                                                    .add_record(vec![FieldData::Int(fid)], false);
                                                fid += 1;
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
                                            // a minimum of four points are needed to form a closed polygon (triangle)
                                            if !vertices[0]
                                                .nearly_equals(&vertices[vertices.len() - 1])
                                            {
                                                p = vertices[0];
                                                if vertices[0]
                                                    .distance(&vertices[vertices.len() - 1])
                                                    < precision
                                                {
                                                    last_index = vertices.len() - 1;
                                                    vertices[last_index] = p;
                                                } else {
                                                    vertices.push(p);
                                                    // println!(
                                                    //     "2 Problem at hull {} {:?}",
                                                    //     i, vertices
                                                    // );
                                                }
                                            }
                                            p = interior_point(&vertices);
                                            overlaps_with_clip = false;
                                            for record_num2 in 0..clip_polylines.len() {
                                                if point_in_poly(
                                                    &p,
                                                    &(clip_polylines[record_num2].vertices),
                                                ) {
                                                    overlaps_with_clip = true;
                                                    break;
                                                }
                                            }
                                            if overlaps_with_clip {
                                                let mut sfg =
                                                    ShapefileGeometry::new(ShapeType::Polygon);
                                                sfg.add_part(&vertices);
                                                hull_geometries.push(sfg);
                                            }
                                        }
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
                                    &(output.records[b].points
                                        [0..output.records[b].parts[1] as usize]),
                                ) {
                                    output.records[b].add_part(&(hull_geometries[a].points));
                                }
                            } else {
                                if poly_in_poly(
                                    &(hull_geometries[a].points),
                                    &(output.records[b].points),
                                ) {
                                    output.records[b].add_part(&(hull_geometries[a].points));
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid input data ShapeType.",
                ));
            }
        }
        */

        // // create output file
        // let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;
        // output.projection = projection;

        // // add the attributes
        // output
        //     .attributes
        //     .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
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
