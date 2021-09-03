/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 8/11/2018
Last Modified: 8/11/2018
License: MIT
*/
extern crate kdtree;

use whitebox_common::algorithms::{
    find_split_points_at_line_intersections, interior_point, is_clockwise_order, point_in_poly,
    poly_in_poly, poly_overlaps_poly,
};
use whitebox_common::structures::{BoundingBox, MultiPolyline, Point2D, Polyline};
use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON;

/// This tool will remove all the overlapping features, or parts of overlapping features, between
/// input and overlay vector files, outputting only the features that occur in one of the two
/// inputs but not both. The *Symmetrical Difference* is related to the Boolean
/// exclusive-or (**XOR**) operation in  set theory and is one of the common vector overlay
/// operations in GIS. The user must specify  the names of the input and overlay vector files
/// as well as the output vector file name. The tool operates on vector points,
/// lines, or polygon, but both the input and overlay files must contain the same ShapeType.
///
/// The *Symmetrical Difference* can also be derived using a combination of other vector
/// overlay operations, as either `(A union B) difference (A intersect B)`, or
/// `(A difference B) union (B difference A)`.
///
/// The attributes of the two input vectors will be merged in the output attribute table.
/// Fields that are duplicated between the inputs will share a single attribute in the
/// output. Fields that only exist in one of the two inputs will be populated by `null`
/// in the output table. Multipoint ShapeTypes however will simply contain a single
/// output feature identifier (`FID`) attribute. Also, note that depending on the
/// ShapeType (polylines and polygons), `Measure` and `Z` ShapeDimension data will not
/// be transferred to the output geometries. If the input attribute table contains fields
/// that measure the geometric properties of their associated features (e.g. length or area),
/// these fields will not be updated to reflect changes in geometry shape and size
/// resulting from the overlay operation.
///
/// # See Also
/// `Intersect`, `Difference`, `Union`, `Clip`, `Erase`
pub struct Difference {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Difference {
    pub fn new() -> Difference {
        // public constructor
        let name = "Difference".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description =
            "Outputs the features that occur in one of the two vector inputs but not both, i.e. no overlapping features."
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp --overlay=layer2.shp -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        Difference {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Difference {
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
            Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;
        output.projection = projection;

        // add the attributes
        // output
        //     .attributes
        //     .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        // let mut input_field_mapping = vec![0; input.attributes.get_num_fields()];
        // for i in 0..input.attributes.get_num_fields() {
        //     let att = input.attributes.get_field(i);
        //     if att.name != "FID" {
        //         if !output.attributes.contains_field(att) {
        //             output.attributes.add_field(&(att.clone()));
        //             input_field_mapping[i] = output.attributes.get_num_fields() - 1;
        //         } else {
        //             input_field_mapping[i] = output.attributes.get_field_num(&att.name).unwrap();
        //         }
        //     }
        // }

        // let mut overlay_field_mapping = vec![0; overlay.attributes.get_num_fields()];
        // for i in 0..overlay.attributes.get_num_fields() {
        //     let att = overlay.attributes.get_field(i);
        //     if att.name != "FID" {
        //         if !output.attributes.contains_field(att) {
        //             output.attributes.add_field(&(att.clone()));
        //             overlay_field_mapping[i] = output.attributes.get_num_fields() - 1;
        //         } else {
        //             overlay_field_mapping[i] = output.attributes.get_field_num(&att.name).unwrap();
        //         }
        //     }
        // }

        let num_attributes = output.attributes.get_num_fields();

        // let num_decimals = 6;
        let precision = EPSILON; //1f64 / num_decimals as f64;

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
                    tree.add([p.x, p.y], 1).unwrap();
                }
                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    p = record.points[0];
                    tree.add([p.x, p.y], 2).unwrap();
                }

                // now see which ones overlap
                let mut fid = 1;
                let num_total_points = (input.num_records + overlay.num_records - 1) as f64;
                let mut output_point: bool;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    p = record.points[0];
                    let ret = tree
                        .within(&[p.x, p.y], precision, &squared_euclidean)
                        .unwrap();

                    output_point = true;
                    for a in 0..ret.len() {
                        if *(ret[a].1) == 2 {
                            output_point = false;
                            break;
                        }
                    }
                    if output_point {
                        // it is not overlapped by another point in the overlay file.
                        output.add_record(record.clone());
                        let mut out_atts = vec![FieldData::Null; num_attributes];
                        out_atts[0] = FieldData::Int(fid);
                        fid += 1;
                        let atts = input.attributes.get_record(record_num);
                        output.attributes.add_record(atts, false);
                    }
                    if verbose {
                        progress = (100.0_f64 * record_num as f64 / num_total_points) as usize;
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
                for record_num in 0..overlay.num_records {
                    let record = overlay.get_record(record_num);
                    for p in &record.points {
                        tree.add([p.x, p.y], 2).unwrap();
                    }
                }

                // now see which ones overlap
                let num_total_points = (total_points - 1) as f64;
                let mut output_point = vec![true; total_points];
                let mut num_out_pnts = total_points;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    for i in 0..record.points.len() {
                        p = record.points[i];
                        let ret = tree
                            .within(&[p.x, p.y], precision, &squared_euclidean)
                            .unwrap();

                        for j in 0..ret.len() {
                            if *(ret[j].1) == 2 {
                                num_out_pnts -= 1;
                                output_point[i] = false;
                                break;
                            }
                        }
                        if verbose {
                            progress = (100.0_f64 * i as f64 / num_total_points) as usize;
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
                                    if output_point[i] {
                                        sfg.add_point((record.points[i]).clone());
                                    }
                                }
                            }
                        }
                        ShapeTypeDimension::Measure => {
                            for record_num in 0..input.num_records {
                                let record = input.get_record(record_num);
                                for i in 0..record.points.len() {
                                    if output_point[i] {
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
                                    if output_point[i] {
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
                    println!("WARNING: no features were output from the tool.");
                }
            }
            ShapeType::PolyLine => {
                output.header.shape_type = ShapeType::PolyLine;
                // let mut features_bb: Vec<BoundingBox> = vec![];
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
                        // features_bb.push(BoundingBox::from_points(
                        //     &(record.points[first_point_in_part..=last_point_in_part]),
                        // ));

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

                // let mut p: Point2D;
                // for i in 0..polylines.len() {
                //     for j in 0..polylines[i].len() {
                //         p = polylines[i][j];
                //         polylines[i].vertices[j] = p.fix_precision(num_decimals);
                //     }
                // }

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

                // Find duplicate polylines and remove them
                let mut duplicate = vec![false; features_polylines.len()];
                for i in 0..features_polylines.len() {
                    if !duplicate[i] {
                        for j in (i + 1)..features_polylines.len() {
                            if features_polylines[i] == features_polylines[j] {
                                if features_polylines[i].source_file
                                    != features_polylines[j].source_file
                                {
                                    duplicate[i] = true;
                                    duplicate[j] = true;
                                }
                            }
                        }
                    }
                }
                for i in (0..features_polylines.len()).rev() {
                    if duplicate[i] {
                        features_polylines.remove(i);
                    }
                }

                // let mut fid = 1i32;
                for i in 0..features_polylines.len() {
                    if features_polylines[i].source_file == 1 {
                        let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                        sfg.add_part(&features_polylines[i].vertices);
                        output.add_record(sfg);
                        let atts = input.attributes.get_record(features_polylines[i].id);
                        output.attributes.add_record(atts, false);
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
                // let mut fid = 1i32;
                for record_num in 0..multipolylines.len() {
                    if multipolylines[record_num][0].source_file == 2 {
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

                            // // convert to fixed precision
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
                            let mut snap_tree =
                                KdTree::with_capacity(dimensions, capacity_per_node);
                            let mut p: Point2D;
                            for i in 0..polygons.len() {
                                for j in 0..polygons[i].len() {
                                    p = polygons[i][j];
                                    snap_tree.add([p.x, p.y], (i, j)).unwrap();
                                }
                            }

                            let mut num_neighbours: Vec<Vec<u8>> =
                                Vec::with_capacity(polygons.len());
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
                                        || num_neighbours[i][j] == 1
                                            && num_neighbours[i][j - 1] == 0
                                        || num_neighbours[i][j] == 0
                                            && num_neighbours[i][j - 1] == 1
                                    {
                                        // it's a junction, split the poly
                                        pl.vertices.push(polygons[i][j]);
                                        features_polylines.push(pl.clone());
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

                            // // convert to fixed precision
                            // for i in 0..polylines.len() {
                            //     for j in 0..polylines[i].len() {
                            //         p = polylines[i][j];
                            //         polylines[i].vertices[j] = p.fix_precision(num_decimals);
                            //     }
                            // }

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
                                        .within(&[p2.x, p2.y], EPSILON, &squared_euclidean)
                                        .unwrap();
                                    if ret.len() == 1 {
                                        is_acyclic_arc[i] = true;
                                    } else {
                                        p3 = polylines[i][polylines[i].len() - 2];
                                        for a in 0..ret.len() {
                                            let index = *ret[a].1;
                                            if index != last_node_id(i)
                                                && !is_acyclic_arc[index / 2]
                                            {
                                                p4 = if is_first_node(index) {
                                                    polylines[index / 2][1]
                                                } else {
                                                    polylines[index / 2]
                                                        [polylines[index / 2].len() - 2]
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
                                        let link =
                                            queue.pop().expect("Error during pop operation.");
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
                                                        priority: link.priority
                                                            + lengths[link.id / 2],
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
                                                        // output the polygon
                                                        let mut sfg = ShapefileGeometry::new(
                                                            ShapeType::Polygon,
                                                        );
                                                        sfg.add_part(&vertices);
                                                        bb.push(sfg.get_bounding_box());
                                                        feature_geometries.push(sfg);
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
                                                existing_hull.insert(test_poly);
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
                                                neighbour_node =
                                                    endnodes[current_node][max_val_index];
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
                                                            if vertices[0].distance(
                                                                &vertices[vertices.len() - 1],
                                                            ) < precision
                                                            {
                                                                last_index = vertices.len() - 1;
                                                                vertices[last_index] = p;
                                                            } else {
                                                                vertices.push(p);
                                                            }
                                                        }
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
                                                        if !overlaps_with_other
                                                            && overlaps_with_poly
                                                        {
                                                            // output the polygon
                                                            let mut sfg = ShapefileGeometry::new(
                                                                ShapeType::Polygon,
                                                            );
                                                            sfg.add_part(&vertices);
                                                            bb.push(sfg.get_bounding_box());
                                                            feature_geometries.push(sfg);
                                                        } else if overlaps_with_other
                                                            && overlaps_with_poly
                                                        {
                                                            let mut test_poly = lines.clone();
                                                            test_poly.sort();
                                                            if !existing_hull.contains(&test_poly) {
                                                                existing_hull.insert(test_poly);
                                                                // it's a potential hull
                                                                let mut sfg =
                                                                    ShapefileGeometry::new(
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
                                                        // a minimum of four points are needed to form a closed polygon (triangle)
                                                        if !vertices[0].nearly_equals(
                                                            &vertices[vertices.len() - 1],
                                                        ) {
                                                            p = vertices[0];
                                                            if vertices[0].distance(
                                                                &vertices[vertices.len() - 1],
                                                            ) < precision
                                                            {
                                                                last_index = vertices.len() - 1;
                                                                vertices[last_index] = p;
                                                            } else {
                                                                vertices.push(p);
                                                            }
                                                        }
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
                                                        if !overlaps_with_other
                                                            && overlaps_with_poly
                                                        {
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
                                let atts = input
                                    .attributes
                                    .get_record(multipolylines[record_num][0].id);
                                output.attributes.add_record(atts, false);
                            }
                        } else {
                            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                            for i in 0..multipolylines[record_num].len() {
                                sfg.add_part(&multipolylines[record_num][i].vertices);
                            }
                            output.add_record(sfg);

                            let atts = input
                                .attributes
                                .get_record(multipolylines[record_num][0].id);
                            output.attributes.add_record(atts, false);
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
