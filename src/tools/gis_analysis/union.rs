/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 05/11/2018
Last Modified: 08/04/2019
License: MIT
*/
extern crate kdtree;

use crate::algorithms::{
    find_split_points_at_line_intersections, interior_point, is_clockwise_order, point_in_poly,
    poly_in_poly, poly_overlaps_poly,
};
use crate::structures::{BoundingBox, MultiPolyline, Polyline};
use crate::tools::*;
use crate::vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON;

/// This tool splits vector layers at their overlaps, creating a layer containing all the portions from both
/// input and overlay layers. The *Union* is related to the Boolean
/// **OR** operation in  set theory and is one of the common vector overlay
/// operations in GIS. The user must specify  the names of the input and overlay vector files
/// as well as the output vector file name. The tool operates on vector points,
/// lines, or polygon, but both the input and overlay files must contain the same ShapeType.
///
/// The attributes of the two input vectors will be merged in the output attribute table.
/// Fields that are duplicated between the inputs will share a single attribute in the
/// output. Fields that only exist in one of the two inputs will be populated by `null`
/// in the output table. Multipoint ShapeTypes however will simply contain a single
/// ouptut feature indentifier (`FID`) attribute. Also, note that depending on the
/// ShapeType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not
/// be transfered to the output geometries. If the input attribute table contains fields
/// that measure the geometric properties of their associated features (e.g. length or area),
/// these fields will not be updated to reflect changes in geometry shape and size
/// resulting from the overlay operation.
///
/// # See Also
/// `Intersect`, `Difference`, `SymmetricalDifference`, `Clip`, `Erase`
pub struct Union {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Union {
    pub fn new() -> Union {
        // public constructor
        let name = "Union".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Splits vector layers at their overlaps, creating a layer containing all the portions from both input and overlay layers.".to_string();

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
            name: "Input Overlay Vector File".to_owned(),
            flags: vec!["--overlay".to_owned()],
            description: "Input overlay vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp --overlay=layer2.shp -o=out_file.shp --snap=0.0000001",
            short_exe, name
        ).replace("*", &sep);

        Union {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Union {
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
        let mut overlay_file = String::new();
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
            } else if flag_val == "-overlay" {
                overlay_file = if keyval {
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
        if !overlay_file.contains(&sep) && !overlay_file.contains("/") {
            overlay_file = format!("{}{}", working_directory, overlay_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let overlay = Shapefile::read(&overlay_file)?;

        let input = Shapefile::read(&input_file)?;
        let projection = input.projection.clone();

        // The overlay file must be of the same ShapeType as the input file
        if overlay.header.shape_type != input.header.shape_type {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input and overlay vector inputs must be of the same shape type.",
            ));
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, false)?;
        output.projection = projection;

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

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

        let mut overlay_field_mapping = vec![0; overlay.attributes.get_num_fields()];
        for i in 0..overlay.attributes.get_num_fields() {
            let att = overlay.attributes.get_field(i);
            if att.name != "FID" {
                if !output.attributes.contains_field(att) {
                    output.attributes.add_field(&(att.clone()));
                    overlay_field_mapping[i] = output.attributes.get_num_fields() - 1;
                } else {
                    overlay_field_mapping[i] = output.attributes.get_field_num(&att.name).unwrap();
                }
            }
        }

        let num_attributes = output.attributes.get_num_fields();

        match input.header.shape_type.base_shape_type() {
            ShapeType::Point => {
                // place the points from both files into a KD-tree
                let dimensions = 2;
                let capacity_per_node = 64;
                let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
                let mut p: Point2D;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    p = record.points[0];
                    tree.add([p.x, p.y], (1, record_num)).unwrap();
                }
                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    p = record.points[0];
                    tree.add([p.x, p.y], (2, record_num)).unwrap();
                }

                // now see which ones overlap
                let mut fid = 1;
                let mut overlay_id: usize = 0;
                let num_total_points = (input.num_records + overlay.num_records - 1) as f64;
                let mut overlapped_point: bool;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    p = record.points[0];
                    let ret = tree
                        .within(&[p.x, p.y], precision, &squared_euclidean)
                        .unwrap();

                    overlapped_point = false;
                    for a in 0..ret.len() {
                        if (ret[a].1).0 == 2 {
                            overlay_id = (ret[a].1).1;
                            overlapped_point = true;
                            break;
                        }
                    }
                    if !overlapped_point {
                        // it is not overlapped by another point in the overlay file.
                        output.add_record(record.clone());
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = input.attributes.get_record(record_num);
                        for att_num in 0..atts.len() {
                            if input_field_mapping[att_num] != 0 {
                                out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                    } else {
                        // it is overlapped by another point in the overlay file.
                        output.add_record(record.clone());
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = input.attributes.get_record(record_num);
                        for att_num in 0..atts.len() {
                            if input_field_mapping[att_num] != 0 {
                                out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }
                        let atts = overlay.attributes.get_record(overlay_id);
                        for att_num in 0..atts.len() {
                            if overlay_field_mapping[att_num] != 0 {
                                out_atts[overlay_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                    }
                    if verbose {
                        progress = (100.0_f64 * record_num as f64 / num_total_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    p = record.points[0];
                    let ret = tree
                        .within(&[p.x, p.y], precision, &squared_euclidean)
                        .unwrap();

                    overlapped_point = false;
                    for a in 0..ret.len() {
                        if (ret[a].1).0 == 1 {
                            overlapped_point = true;
                            break;
                        }
                    }
                    if !overlapped_point {
                        // it is not overlapped by another point in the overlay file.
                        output.add_record(record.clone());
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = overlay.attributes.get_record(record_num);
                        for att_num in 0..atts.len() {
                            if overlay_field_mapping[att_num] != 0 {
                                out_atts[overlay_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                    }
                    if verbose {
                        progress = (100.0_f64 * (record_num + input.num_records) as f64
                            / num_total_points) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            ShapeType::MultiPoint => {
                // place the points from both files into a KD-tree
                let dimensions = 2;
                let capacity_per_node = 64;
                let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
                let mut p: Point2D;
                let mut total_points = 0;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    for p in &record.points {
                        tree.add([p.x, p.y], 1).unwrap();
                        total_points += 1;
                    }
                }
                let num_points_input = total_points;
                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    for p in &record.points {
                        tree.add([p.x, p.y], 2).unwrap();
                        total_points += 1;
                    }
                }

                // now see which ones overlap
                let num_total_points = (total_points - 1) as f64;
                let mut overlapped_point = vec![false; total_points];
                let mut num_out_pnts = total_points;

                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    for i in 0..record.points.len() {
                        p = record.points[i];
                        let ret = tree
                            .within(&[p.x, p.y], precision, &squared_euclidean)
                            .unwrap();

                        for j in 0..ret.len() {
                            if *(ret[j].1) == 1 {
                                num_out_pnts -= 1;
                                overlapped_point[i + num_points_input] = true;
                                break;
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * (i + num_points_input) as f64
                                / num_total_points) as usize;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }
                }

                if num_out_pnts > 0 {
                    // attributes aren't provided for multipoints overlay.
                    output.attributes.reinitialize();
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        7u8,
                        0u8,
                    ));
                    let mut sfg = ShapefileGeometry::new(input.header.shape_type);
                    match input.header.shape_type.dimension() {
                        ShapeTypeDimension::XY => {
                            for record_num in 0..input.num_records {
                                let record = input.get_record(record_num);
                                for i in 0..record.points.len() {
                                    sfg.add_point((record.points[i]).clone());
                                }
                            }

                            for record_num in 0..overlay.num_records {
                                let record = overlay.get_record(record_num);
                                for i in 0..record.points.len() {
                                    if !overlapped_point[i + num_points_input] {
                                        sfg.add_point((record.points[i]).clone());
                                    }
                                }
                            }
                        }
                        ShapeTypeDimension::Measure => {
                            for record_num in 0..input.num_records {
                                let record = input.get_record(record_num);
                                for i in 0..record.points.len() {
                                    sfg.add_pointm((record.points[i]).clone(), record.m_array[i]);
                                }
                            }

                            for record_num in 0..overlay.num_records {
                                let record = overlay.get_record(record_num);
                                for i in 0..record.points.len() {
                                    if !overlapped_point[i + num_points_input] {
                                        sfg.add_pointm(
                                            (record.points[i]).clone(),
                                            record.m_array[i],
                                        );
                                    }
                                }
                            }
                        }
                        ShapeTypeDimension::Z => {
                            for record_num in 0..input.num_records {
                                let record = input.get_record(record_num);
                                for i in 0..record.points.len() {
                                    sfg.add_pointz(
                                        (record.points[i]).clone(),
                                        record.m_array[i],
                                        record.z_array[i],
                                    );
                                }
                            }

                            for record_num in 0..overlay.num_records {
                                let record = overlay.get_record(record_num);
                                for i in 0..record.points.len() {
                                    if !overlapped_point[i + num_points_input] {
                                        sfg.add_pointz(
                                            (record.points[i]).clone(),
                                            record.m_array[i],
                                            record.z_array[i],
                                        );
                                    }
                                }
                            }
                        }
                    }
                    output.add_record(sfg);
                    output
                        .attributes
                        .add_record(vec![FieldData::Int(1i32)], false);
                } else {
                    println!("WARNING: no features were ouput from the tool.");
                }
            }
            ShapeType::PolyLine => {
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

                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
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
                        pl.source_file = 2;
                        polylines.push(pl);
                    }
                }

                // Break the polylines up into shorter lines at junction points.
                let dimensions = 2;
                let capacity_per_node = 64;
                let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
                let mut p: Point2D;
                for i in 0..polylines.len() {
                    for j in 0..polylines[i].len() {
                        p = polylines[i][j];
                        tree.add([p.x, p.y], (i, j)).unwrap();
                    }
                }

                let mut num_neighbours: Vec<Vec<u8>> = Vec::with_capacity(polylines.len());
                for i in 0..polylines.len() {
                    let mut line_num_neighbours = Vec::with_capacity(polylines[i].len());
                    for j in 0..polylines[i].len() {
                        p = polylines[i][j];
                        let ret = tree
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

                    if verbose {
                        progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                let mut features_polylines: Vec<Polyline> = vec![];
                let mut id: usize;
                for i in 0..polylines.len() {
                    id = polylines[i].id;
                    let mut pl = Polyline::new_empty(id);
                    pl.vertices.push(polylines[i][0]);
                    pl.source_file = polylines[i].source_file;
                    for j in 1..polylines[i].len() {
                        if num_neighbours[i][j] > 1
                            || num_neighbours[i][j] == 1 && num_neighbours[i][j - 1] == 0
                            || num_neighbours[i][j] == 0 && num_neighbours[i][j - 1] == 1
                        {
                            // it's a junction, split the poly
                            pl.vertices.push(polylines[i][j]);
                            features_polylines.push(pl.clone());
                            pl = Polyline::new_empty(id);
                            pl.vertices.push(polylines[i][j]);
                            pl.source_file = polylines[i].source_file;
                        } else {
                            pl.vertices.push(polylines[i][j]);
                        }
                    }
                    features_polylines.push(pl.clone());

                    if verbose {
                        progress = (100.0_f64 * (i + 1) as f64 / polylines.len() as f64) as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                // Remove any zero-length line segments
                for i in 0..features_polylines.len() {
                    for j in (1..features_polylines[i].len()).rev() {
                        if features_polylines[i][j].nearly_equals(&features_polylines[i][j - 1]) {
                            features_polylines[i].remove(j);
                        }
                    }
                }

                // Remove any single-point lines result from above.
                let mut features_polylines2: Vec<Polyline> = vec![];
                for i in (0..features_polylines.len()).rev() {
                    if features_polylines[i].len() > 1 {
                        features_polylines2.push(features_polylines[i].clone());
                    }
                }
                features_polylines = features_polylines2.clone();
                drop(features_polylines2);

                // Find duplicate polylines and remove them
                let mut duplicate = vec![false; features_polylines.len()];
                let mut duplicate_partner = vec![0; features_polylines.len()];
                let mut num_duplicates = 0;
                // let mut source_file: usize;
                // let mut first_vertex1: Point2D;
                // let mut first_vertex2: Point2D;
                // for i in 0..features_polylines.len() {
                //     if !duplicate[i] {
                //         source_file = features_polylines[i].source_file;
                //         // first_vertex1 = features_polylines[i].first_vertex();
                //         for j in (i + 1)..features_polylines.len() {
                //             if source_file != features_polylines[j].source_file {
                //                 if lines_are_equal(&features_polylines[i].vertices, &features_polylines[j].vertices) {
                //                     num_duplicates += 1;
                //                 }
                //                 // first_vertex2 = features_polylines[j].first_vertex();
                //                 // if first_vertex1.x == first_vertex2.x && first_vertex1.y == first_vertex2.y {
                //                 //     num_duplicates += 1;
                //                 // }
                //                 // if features_polylines[i] == features_polylines[j] {
                //                 // // if features_polylines[i].equals(&features_polylines[j]) {
                //                 //     duplicate[i] = true;
                //                 //     duplicate[j] = true;
                //                 //     duplicate_partner[i] = j;
                //                 //     duplicate_partner[j] = i;
                //                 //     num_duplicates += 1;
                //                 // }
                //             }
                //         }
                //     }

                //     if verbose {
                //         progress = (100.0_f64 * (i + 1) as f64 / features_polylines.len() as f64) as usize;
                //         if progress != old_progress {
                //             println!("Finding duplicate polylines ({}): {}%", num_duplicates, progress);
                //             old_progress = progress;
                //         }
                //     }
                // }

                let mut tree = KdTree::with_capacity(dimensions, capacity_per_node);
                let mut p1: Point2D;
                let mut p2: Point2D;
                for i in 0..features_polylines.len() {
                    p1 = features_polylines[i].first_vertex();
                    tree.add([p1.x, p1.y], first_node_id(i)).unwrap();

                    p2 = features_polylines[i].last_vertex();
                    tree.add([p2.x, p2.y], last_node_id(i)).unwrap();
                }

                let mut j: usize;
                let mut index: usize;
                for i in 0..features_polylines.len() {
                    if !duplicate[i] {
                        p = features_polylines[i].first_vertex();

                        let ret = tree
                            .within(&[p.x, p.y], precision, &squared_euclidean)
                            .unwrap();
                        if ret.len() > 1 {
                            for a in 0..ret.len() {
                                index = *ret[a].1;
                                j = index / 2;
                                if j != i
                                    && features_polylines[j].source_file
                                        != features_polylines[i].source_file
                                {
                                    if features_polylines[i]
                                        .nearly_equals(&features_polylines[j], precision)
                                    {
                                        duplicate[i] = true;
                                        duplicate[j] = true;
                                        duplicate_partner[i] = j;
                                        duplicate_partner[j] = i;
                                        num_duplicates += 1;
                                    }
                                }
                            }
                        }
                    }
                    if verbose {
                        progress =
                            (100.0_f64 * (i + 1) as f64 / features_polylines.len() as f64) as usize;
                        if progress != old_progress {
                            println!(
                                "Finding duplicate polylines ({}): {}%",
                                num_duplicates, progress
                            );
                            old_progress = progress;
                        }
                    }
                }

                let mut fid = 1i32;
                for i in 0..features_polylines.len() {
                    if duplicate[i] && features_polylines[i].source_file == 1 {
                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&features_polylines[i].vertices);
                        output.add_record(sfg);
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = input.attributes.get_record(features_polylines[i].id);
                        for att_num in 0..atts.len() {
                            if input_field_mapping[att_num] != 0 {
                                out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }

                        let atts = overlay
                            .attributes
                            .get_record(features_polylines[duplicate_partner[i]].id);
                        for att_num in 0..atts.len() {
                            if overlay_field_mapping[att_num] != 0 {
                                out_atts[overlay_field_mapping[att_num]] = atts[att_num].clone();
                            }
                        }

                        output.attributes.add_record(out_atts, false);
                    } else if !duplicate[i] {
                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&features_polylines[i].vertices);
                        output.add_record(sfg);
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        if features_polylines[i].source_file == 2 {
                            let atts = overlay.attributes.get_record(features_polylines[i].id);
                            for att_num in 0..atts.len() {
                                if overlay_field_mapping[att_num] != 0 {
                                    out_atts[overlay_field_mapping[att_num]] =
                                        atts[att_num].clone();
                                }
                            }
                        } else {
                            let atts = input.attributes.get_record(features_polylines[i].id);
                            for att_num in 0..atts.len() {
                                if input_field_mapping[att_num] != 0 {
                                    out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                                }
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                    }
                }
            }
            ShapeType::Polygon => {
                // The polyline splitline method makes keeping track of
                // measure and z data for split lines difficult. Regardless
                // of the input shapefile dimension, the output will be XY only.
                output.header.shape_type = ShapeType::Polygon;

                let mut multipolylines: Vec<MultiPolyline> = vec![];
                let mut is_part_a_hole: Vec<Vec<bool>> = vec![];
                let mut first_point_in_part: usize;
                let mut last_point_in_part: usize;

                // Read in the features
                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    let mut mpl = MultiPolyline::new(record_num);
                    let mut holes = vec![false; record.num_parts as usize];
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
                        mpl.push(&pl);

                        if record.is_hole(part as i32) {
                            holes[part] = true;
                        }
                    }
                    multipolylines.push(mpl);
                    is_part_a_hole.push(holes);
                }

                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    let mut mpl = MultiPolyline::new(record_num);
                    let mut holes = vec![false; record.num_parts as usize];
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
                        pl.source_file = 2;
                        mpl.push(&pl);

                        if record.is_hole(part as i32) {
                            holes[part] = true;
                        }
                    }
                    multipolylines.push(mpl);
                    is_part_a_hole.push(holes);
                }

                // Perform the overlay on individual features.
                let mut fid = 1i32;
                for record_num in 0..multipolylines.len() {
                    let mut polygons: Vec<Polyline> = vec![];
                    let mut is_part_a_hole2: Vec<bool> = vec![];

                    // find overlapping features in other file
                    let mut overlaps_with_feature: bool;
                    for i in 0..multipolylines.len() {
                        overlaps_with_feature = false;
                        if multipolylines[i][0].source_file
                            != multipolylines[record_num][0].source_file
                        {
                            if multipolylines[record_num]
                                .get_bounding_box()
                                .overlaps(multipolylines[i].get_bounding_box())
                            {
                                for j in 0..multipolylines[record_num].len() {
                                    for k in 0..multipolylines[i].len() {
                                        if poly_overlaps_poly(
                                            &(multipolylines[record_num][j].vertices),
                                            &(multipolylines[i][k].vertices),
                                        ) {
                                            overlaps_with_feature = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if overlaps_with_feature {
                            for j in 0..multipolylines[i].len() {
                                polygons.push(multipolylines[i][j].clone());
                                is_part_a_hole2.push(is_part_a_hole[i][j]);
                            }
                        }
                    }

                    if polygons.len() > 0 {
                        let feature_source_file = multipolylines[record_num][0].source_file;

                        for j in 0..multipolylines[record_num].len() {
                            polygons.push(multipolylines[record_num][j].clone());
                            is_part_a_hole2.push(is_part_a_hole[record_num][j]);
                        }

                        // Break the polygons up into line segments at junction points and endnodes.
                        let mut p: Point2D;
                        const DIMENSIONS: usize = 2;
                        const CAPACITY_PER_NODE: usize = 64;
                        let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
                        for i in 0..polygons.len() {
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

                        let mut features_polylines: Vec<Polyline> = vec![];
                        let mut id: usize;
                        let mut jn: usize;
                        let mut endnode_n: bool;
                        let mut dist1: f64;
                        let mut dist2: f64;
                        let mut num_neighbours: usize;
                        let mut neighbour_set = HashSet::new();
                        for i in 0..polygons.len() {
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
                                        dist1 = (polygons[i][j - 1]
                                            .distance(&polygons[id][jn - 1]))
                                        .min(polygons[i][j - 1].distance(&polygons[id][jn + 1]));
                                        dist2 = (polygons[i][j + 1]
                                            .distance(&polygons[id][jn - 1]))
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
                                    features_polylines.push(pl.clone());
                                    pl = Polyline::new_empty(i);
                                    pl.vertices.push(polygons[i][j]);
                                }
                            }
                        }

                        // Remove any zero-length line segments
                        for i in 0..features_polylines.len() {
                            for j in (1..features_polylines[i].len()).rev() {
                                if features_polylines[i][j] == features_polylines[i][j - 1] {
                                    features_polylines[i].remove(j);
                                }
                            }
                        }

                        // Remove any single-point lines result from above.
                        for i in (0..features_polylines.len()).rev() {
                            if features_polylines[i].len() < 2 {
                                features_polylines.remove(i);
                            }
                        }

                        // Find duplicate polylines and remove them
                        let mut duplicate = vec![false; features_polylines.len()];
                        for i in 0..features_polylines.len() {
                            if !duplicate[i] {
                                for j in (i + 1)..features_polylines.len() {
                                    if features_polylines[i]
                                        .nearly_equals(&features_polylines[j], precision)
                                    {
                                        duplicate[j] = true;
                                        break; // we don't really have more than two overlapping lines ever.
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
                        let mut features_bb = Vec::with_capacity(features_polylines.len());
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

                        // Remove any zero-length line segments
                        for i in 0..polygons.len() {
                            for j in (1..polygons[i].len()).rev() {
                                if polygons[i][j] == polygons[i][j - 1] {
                                    polygons[i].remove(j);
                                }
                            }
                        }
                        // Remove any single-point lines result from above.
                        for i in (0..polygons.len()).rev() {
                            if polygons[i].len() < 2 {
                                polygons.remove(i);
                            }
                        }

                        // Find duplicate polylines and remove them
                        let mut duplicate = vec![false; polylines.len()];
                        for i in 0..polylines.len() {
                            if !duplicate[i] {
                                for j in (i + 1)..polylines.len() {
                                    if polylines[i] == polylines[j] {
                                        duplicate[j] = true;
                                        break;
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
                        let mut kdtree = KdTree::with_capacity(dimensions, capacity_per_node);
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
                                    .within(&[p2.x, p2.y], precision, &squared_euclidean)
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

                        ///////////////////////////////////////////////////////////////////////////////////////////
                        // This is the main part of the analysis. It is responsible for rebuilding the polygons. //
                        ///////////////////////////////////////////////////////////////////////////////////////////
                        let mut bb: Vec<BoundingBox> = vec![];
                        let mut current_node: usize;
                        let mut neighbour_node: usize;
                        let mut num_neighbours: usize;
                        let mut existing_polygons = HashSet::new();
                        let mut existing_hull = HashSet::new();
                        let mut feature_geometries: Vec<ShapefileGeometry> = vec![];
                        let mut hull_geometries: Vec<ShapefileGeometry> = vec![];
                        let mut overlay_poly_id: Vec<usize> = vec![];
                        let mut p: Point2D;
                        let mut max_val: f64;
                        let mut max_val_index: usize;
                        let mut k: usize;
                        let mut num_vertices: usize;
                        let mut other_side: usize;
                        let mut target_found: bool;
                        let mut assigned = vec![0usize; polylines.len()];
                        let mut is_clockwise: bool;
                        let mut overlaps_with_other: bool;
                        let mut overlaps_with_poly: bool;
                        let mut poly_is_hole: bool;
                        let mut other_is_hole: bool;
                        let mut last_index: usize;
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
                                    let mut vertices: Vec<Point2D> =
                                        Vec::with_capacity(num_vertices);
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
                                        if vertices[0].distance(&vertices[vertices.len() - 1])
                                            < precision
                                        {
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
                                            existing_polygons.insert(test_poly);
                                            for a in 0..lines.len() {
                                                assigned[lines[a]] += 1;
                                            }
                                            if vertices.len() > 3 {
                                                p = interior_point(&vertices);
                                                overlaps_with_other = false;
                                                overlaps_with_poly = false;
                                                poly_is_hole = false;
                                                other_is_hole = false;
                                                let mut other_poly_id = 0;
                                                for j in 0..polygons.len() {
                                                    if point_in_poly(&p, &(polygons[j].vertices)) {
                                                        if polygons[j].source_file
                                                            != feature_source_file
                                                        {
                                                            other_poly_id = polygons[j].id;
                                                            if !is_part_a_hole2[j] && !other_is_hole
                                                            {
                                                                overlaps_with_other = true;
                                                            } else {
                                                                overlaps_with_other = false;
                                                                other_is_hole = true;
                                                            }
                                                        } else {
                                                            if !is_part_a_hole2[j] && !poly_is_hole
                                                            {
                                                                overlaps_with_poly = true;
                                                            } else {
                                                                overlaps_with_poly = false;
                                                                poly_is_hole = true;
                                                            }
                                                        }
                                                    }
                                                }

                                                if (feature_source_file == 2 && overlaps_with_poly)
                                                    || (feature_source_file == 1
                                                        && overlaps_with_poly
                                                        && !overlaps_with_other)
                                                {
                                                    // output the polygon
                                                    let mut sfg =
                                                        ShapefileGeometry::new(ShapeType::Polygon);
                                                    sfg.add_part(&vertices);
                                                    bb.push(sfg.get_bounding_box());
                                                    feature_geometries.push(sfg);
                                                    if feature_source_file == 2
                                                        && overlaps_with_other
                                                    {
                                                        overlay_poly_id.push(other_poly_id);
                                                    } else {
                                                        overlay_poly_id.push(polygons.len());
                                                    }
                                                } else if overlaps_with_other && overlaps_with_poly
                                                {
                                                    let mut test_poly = lines.clone();
                                                    test_poly.sort();
                                                    if !existing_hull.contains(&test_poly) {
                                                        existing_hull.insert(test_poly);
                                                        // it's a potential hull
                                                        let mut sfg = ShapefileGeometry::new(
                                                            ShapeType::Polygon,
                                                        );
                                                        vertices.reverse();
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
                                                p = interior_point(&vertices);
                                                overlaps_with_other = false;
                                                overlaps_with_poly = false;
                                                poly_is_hole = false;
                                                other_is_hole = false;
                                                for j in 0..polygons.len() {
                                                    if point_in_poly(&p, &(polygons[j].vertices)) {
                                                        if polygons[j].source_file
                                                            != feature_source_file
                                                        {
                                                            if !is_part_a_hole2[j] && !other_is_hole
                                                            {
                                                                overlaps_with_other = true;
                                                            } else {
                                                                overlaps_with_other = false;
                                                                other_is_hole = true;
                                                            }
                                                        } else {
                                                            if !is_part_a_hole2[j] && !poly_is_hole
                                                            {
                                                                overlaps_with_poly = true;
                                                            } else {
                                                                overlaps_with_poly = false;
                                                                poly_is_hole = true;
                                                            }
                                                        }
                                                    }
                                                }
                                                if !overlaps_with_other && overlaps_with_poly {
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
                                        let mut vertices: Vec<Point2D> =
                                            Vec::with_capacity(num_vertices);
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
                                            }
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
                                                    p = interior_point(&vertices);
                                                    overlaps_with_other = false;
                                                    overlaps_with_poly = false;
                                                    poly_is_hole = false;
                                                    other_is_hole = false;
                                                    let mut other_poly_id = 0;
                                                    for j in 0..polygons.len() {
                                                        if point_in_poly(
                                                            &p,
                                                            &(polygons[j].vertices),
                                                        ) {
                                                            if polygons[j].source_file
                                                                != feature_source_file
                                                            {
                                                                other_poly_id = polygons[j].id;
                                                                if !is_part_a_hole2[j]
                                                                    && !other_is_hole
                                                                {
                                                                    overlaps_with_other = true;
                                                                } else {
                                                                    overlaps_with_other = false;
                                                                    other_is_hole = true;
                                                                }
                                                            } else {
                                                                if !is_part_a_hole2[j]
                                                                    && !poly_is_hole
                                                                {
                                                                    overlaps_with_poly = true;
                                                                } else {
                                                                    overlaps_with_poly = false;
                                                                    poly_is_hole = true;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if (feature_source_file == 2
                                                        && overlaps_with_poly)
                                                        || (feature_source_file == 1
                                                            && overlaps_with_poly
                                                            && !overlaps_with_other)
                                                    {
                                                        // output the polygon
                                                        let mut sfg = ShapefileGeometry::new(
                                                            ShapeType::Polygon,
                                                        );
                                                        sfg.add_part(&vertices);
                                                        bb.push(sfg.get_bounding_box());
                                                        feature_geometries.push(sfg);
                                                        if feature_source_file == 2
                                                            && overlaps_with_other
                                                        {
                                                            overlay_poly_id.push(other_poly_id);
                                                        } else {
                                                            overlay_poly_id.push(polygons.len());
                                                        }
                                                    } else if overlaps_with_other
                                                        && overlaps_with_poly
                                                    {
                                                        let mut test_poly = lines.clone();
                                                        test_poly.sort();
                                                        if !existing_hull.contains(&test_poly) {
                                                            existing_hull.insert(test_poly);
                                                            // it's a potential hull
                                                            let mut sfg = ShapefileGeometry::new(
                                                                ShapeType::Polygon,
                                                            );
                                                            vertices.reverse();
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
                                                    p = interior_point(&vertices);
                                                    overlaps_with_other = false;
                                                    overlaps_with_poly = false;
                                                    poly_is_hole = false;
                                                    other_is_hole = false;
                                                    for j in 0..polygons.len() {
                                                        if point_in_poly(
                                                            &p,
                                                            &(polygons[j].vertices),
                                                        ) {
                                                            if polygons[j].source_file
                                                                != feature_source_file
                                                            {
                                                                if !is_part_a_hole2[j]
                                                                    && !other_is_hole
                                                                {
                                                                    overlaps_with_other = true;
                                                                } else {
                                                                    overlaps_with_other = false;
                                                                    other_is_hole = true;
                                                                }
                                                            } else {
                                                                if !is_part_a_hole2[j]
                                                                    && !poly_is_hole
                                                                {
                                                                    overlaps_with_poly = true;
                                                                } else {
                                                                    overlaps_with_poly = false;
                                                                    poly_is_hole = true;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if !overlaps_with_other && overlaps_with_poly {
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

                            let mut out_atts = vec![FieldData::Null; num_attributes];
                            out_atts[0] = FieldData::Int(fid);
                            fid += 1;
                            if multipolylines[record_num][0].source_file == 1 {
                                let atts = overlay
                                    .attributes
                                    .get_record(multipolylines[record_num][0].id);
                                for att_num in 0..atts.len() {
                                    if overlay_field_mapping[att_num] != 0 {
                                        out_atts[overlay_field_mapping[att_num]] =
                                            atts[att_num].clone();
                                    }
                                }
                            } else {
                                let atts = input
                                    .attributes
                                    .get_record(multipolylines[record_num][0].id);
                                for att_num in 0..atts.len() {
                                    if input_field_mapping[att_num] != 0 {
                                        out_atts[input_field_mapping[att_num]] =
                                            atts[att_num].clone();
                                    }
                                }
                                if multipolylines[record_num][0].source_file == 2
                                    && overlay_poly_id[a] < polygons.len()
                                {
                                    let atts = overlay.attributes.get_record(overlay_poly_id[a]);
                                    for att_num in 0..atts.len() {
                                        if overlay_field_mapping[att_num] != 0 {
                                            out_atts[overlay_field_mapping[att_num]] =
                                                atts[att_num].clone();
                                        }
                                    }
                                }
                            }
                            output.attributes.add_record(out_atts, false);
                        }
                    } else {
                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                        for i in 0..multipolylines[record_num].len() {
                            sfg.add_part(&multipolylines[record_num][i].vertices);
                        }
                        output.add_record(sfg);

                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        if multipolylines[record_num][0].source_file == 1 {
                            let atts = overlay
                                .attributes
                                .get_record(multipolylines[record_num][0].id);
                            for att_num in 0..atts.len() {
                                if overlay_field_mapping[att_num] != 0 {
                                    out_atts[overlay_field_mapping[att_num]] =
                                        atts[att_num].clone();
                                }
                            }
                        } else {
                            let atts = input
                                .attributes
                                .get_record(multipolylines[record_num][0].id);
                            for att_num in 0..atts.len() {
                                if input_field_mapping[att_num] != 0 {
                                    out_atts[input_field_mapping[att_num]] = atts[att_num].clone();
                                }
                            }
                        }
                        output.attributes.add_record(out_atts, false);
                    }

                    if verbose {
                        progress = (100.0_f64 * (record_num + 1) as f64
                            / multipolylines.len() as f64)
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
