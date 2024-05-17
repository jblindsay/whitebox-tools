/* 
Authors: Prof. John Lindsay
Created: 03/08/2021 (oringinally in Whitebox Toolset Extension)
Last Modified: 04/11/2023
License: MIT
*/

use rayon::prelude::*;
use rstar::primitives::{GeomWithData, Line};
use rstar::RTree;
use std::sync::{Arc, Mutex};
use std::env;
use std::fs;
use std::f64;
use std::io::{Error, ErrorKind};
use std::ops::Index;
use std::path;
use std::str;
use std::time::Instant;
use whitebox_common::structures::{
    LineSegment, 
    Point2D
};
use whitebox_common::utils::{
    get_formatted_elapsed_time, 
    wrapped_print
};
use whitebox_vector::{
    AttributeField, 
    FieldData, 
    FieldDataType, 
    Shapefile, 
    ShapefileGeometry, 
    ShapeType
};
const EPSILON: f64 = std::f64::EPSILON;

/// This tool can be used to resolve many of the topological errors and inconsistencies associated with
/// manually digitized vector stream networks, i.e. hydrography data. A properly structured stream network
/// should consist of a series of stream segments that connect a channel head to a downstream confluence, 
/// or an upstream confluence to a downstream confluence/outlet. This tool will join vector arcs that
/// connect at arbitrary, non-confluence points along stream segments. It also splits an arc where
/// a tributary stream connects at a mid-point, thereby creating a proper confluence where two upstream
/// triburaries converge into a downstream segment. The tool also handles non-connecting tributaries
/// caused by dangling arcs, i.e. overshoots and undershoots.
///
/// ![](../../doc_img/RepairStreamVectorTopology.png)
///
/// The user may optionally specify the name of the input vector stream network (`--input`) and the output file 
/// (`--output`). Note that if an input file is not specified by the user, the tool will search for all vector
/// files (*.shp) files contained within the current working directory. This feature can be very useful when 
/// you need to process a large number of stream files contained within a single directory. The tool will 
/// process the files in parallel in this batch mode. 
/// 
/// A distance threshold for snapping dangling arcs (`--snap`) must be specified by the user. This distance 
/// is in the input layer's x-y units. The tool works best on projected input 
/// data, however, if the input are in geographic coordinates (latitude and longitude), then specifying a
/// small valued snap distance is advisable. 
/// 
/// Additionally, the tool possesses two Boolean flags, `--reverse_backward_arcs` and `--correct_nonconfluence_joins`
/// which determine whether the tool will correct backward arcs (i.e., line segements that are oriented
/// in the reverse direction to the streamflow) and non-confluence joins (i.e., upstream/downstream line
/// segments that are not joined at confluence locations).
/// 
/// Notice that the attributes of the input layer will not be
/// carried over to the output file because there is not a one-for-one feature correspondence between the
/// two files due to the joins and splits of stream segments. Instead the output attribute table will
/// only contain a feature ID (FID) entry. 
///
/// > Note: this tool should be used to pre-process vector streams that are input to the
/// > `VectorStreamNetworkAnalysis` tool.
///
/// # See Also
/// `VectorStreamNetworkAnalysis`, `FixDanglingArcs`
fn main() {
    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}
            Err(e) => panic!("{:?}", e),
        }
    }

    if args.len() <= 1 || args[1].trim() == "help" {
        // print help
        help();
    }

    if args[1].trim() == "version" {
        // print version information
        version();
    }
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("repair_stream_vector_topology{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    This tool resolves topological errors and inconsistencies associated with digitized vector streams.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    --routes                  Name of the input routes vector file.
    -o, --output              Name of the output HTML file.
    --length                  Maximum segment length (m).
    --dist                    Search distance, in grid cells, used in visibility analysis.
    --reverse_backward_arcs   Boolean flag determines whether backward arcs are corrected.
    --correct_nonconfluence_joins  Boolean flag determines whether non-confluence joins are corrected.
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --routes=footpath.shp --dem=DEM.tif -o=assessedRoutes.shp --length=50.0 --dist=200

    Note: Use of this tool requires a valid license. To obtain a license,
    contact Whitebox Geospatial Inc. (support@whiteboxgeo.com).
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "repair_stream_vector_topology v{} by Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("RepairStreamVectorTopology") // This should be camel case and is a reference to the tool name.
}

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    let tool_name = get_tool_name();

    let sep: String = path::MAIN_SEPARATOR.to_string();

    // Read in the environment variables and get the necessary values
    let configurations = whitebox_common::configs::get_configs()?;
    let mut working_directory = configurations.working_directory.clone();
    if !working_directory.is_empty() && !working_directory.ends_with(&sep) {
        working_directory += &sep;
    }

    // read the arguments
    let mut input_file = String::new();
    let mut output_file: String = String::new();
    let mut snap_dist = 1.0; 
    let mut reverse_backward_arcs = false;
    let mut correct_nonconfluence_joins = false;
    if args.len() <= 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Tool run with too few parameters.",
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
        } else if flag_val == "-snap" || flag_val == "-dist" {
            snap_dist = if keyval {
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
        } else if flag_val == "-reverse_backward_arcs" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                reverse_backward_arcs = true;
            }
        } else if flag_val == "-correct_nonconfluence_joins" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                correct_nonconfluence_joins = true;
            }
        }
    }

    if configurations.verbose_mode {
        let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
        // 28 = length of the 'Powered by' by statement.
        println!("{}", "*".repeat(welcome_len));
        println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
        println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
        println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
        println!("{}", "*".repeat(welcome_len));
    }

    // let mut progress: usize;
    // let mut old_progress: usize = 1;
    let old_progress = Arc::new(Mutex::new(1usize));
    let tiles_completed = Arc::new(Mutex::new(0usize));

    let start = Instant::now();

    let mut inputs = vec![];
    let mut outputs = vec![];
    if input_file.is_empty() {
        if working_directory.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool must be run by specifying either an individual input file or a working directory."));
        }
        if std::path::Path::new(&working_directory).is_dir() {
            for entry in fs::read_dir(working_directory.clone())? {
                let s = entry?
                    .path()
                    .into_os_string()
                    .to_str()
                    .expect("Error reading path string")
                    .to_string();
                if s.to_lowercase().ends_with(".shp") {
                    inputs.push(s);
                    outputs.push(
                        inputs[inputs.len() - 1]
                            .replace(".shp", "_repaired.shp")
                            .replace(".SHP", "_repaired.shp"),
                    )
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("The input directory ({}) is incorrect.", working_directory),
            ));
        }
    } else {
        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        inputs.push(input_file.clone());
        if output_file.is_empty() {
            output_file = input_file
                .clone()
                .replace(".shp", "_corrected.shp")
                .replace(".SHP", "_corrected.shp");
        }
        if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        outputs.push(output_file);
    }
    

    // if !input_file.contains(&sep) && !input_file.contains("/") {
    //     input_file = format!("{}{}", working_directory, input_file);
    // }

    // if !output_file.contains(&sep) && !output_file.contains("/") {
    //     output_file = format!("{}{}", working_directory, output_file);
    // }

    if snap_dist <= 0f64 {
        if configurations.verbose_mode {
            wrapped_print("Error: The snap distance must be greater than 0.0.", 50);
        }
    }

    let input_num_features = Arc::new(Mutex::new(0usize));
    let output_num_features = Arc::new(Mutex::new(0usize));

    (0..inputs.len()).into_par_iter().for_each(|k| {
        let input_file = inputs[k].clone();
        let output_file = outputs[k].clone();

        let input = Shapefile::read(&input_file).expect("Error reading file"); //?;

        {
            let mut input_num_features = input_num_features.lock().expect("Error unlocking input_num_features.");
            *input_num_features += input.num_records;
        }
        
        // Make sure the input vector file is of polyline type
        if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            // return Err(Error::new(
            //     ErrorKind::InvalidInput,
            //     "The vector data must be of PolyLine base shape type.",
            // ));
            panic!("The vector data must be of PolyLine base shape type.");
        }

        let mut progress: usize;

        // Read each line segment into an rtree.
        type Location = GeomWithData<[f64; 2], usize>;
        let mut line_segments = vec![];
        let mut end_nodes = vec![];
        let (mut part_start, mut part_end): (usize, usize);
        let mut fid = 0usize; // fid is unique to each part in the vector
        let mut segment_num = 0usize;
        let mut polylines = vec![];
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);        
            for part in 0..record.num_parts as usize {
                part_start = record.parts[part] as usize;
                part_end = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };

                polylines.push(
                    Polyline::new(
                        &record.points[part_start..=part_end], 
                        fid
                    )
                );

                // segment_num = 0;
                // for i in part_start+1..=part_end {
                //     line_segments.push(
                //         LineWithData::new(
                //             (fid, segment_num),
                //             // fid,
                //             [record.points[i-1].x, record.points[i-1].y],
                //             [record.points[i].x, record.points[i].y]
                //         )
                //     );
                //     segment_num += 1;
                // }

                end_nodes.push(Location::new(
                    [record.points[part_start].x, record.points[part_start].y],
                    fid
                ));

                end_nodes.push(Location::new(
                    [record.points[part_end].x, record.points[part_end].y],
                    fid
                ));

                fid += 1;
            }

            if configurations.verbose_mode && inputs.len() == 1 {
                progress = (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Reading vector: {}%", progress);
                    *old_progress = progress;
                }
            }
        }

        let mut num_polylines = polylines.len(); // will be updated after the joins.


        let endnode_tree = RTree::bulk_load(end_nodes);
        let precision = EPSILON * 10f64;
        let mut p1: Point2D;
        let mut connections = vec![[num_polylines, num_polylines]; num_polylines];
        let mut connected_polyline: usize;
        let mut num_neighbours: usize;
            
        if correct_nonconfluence_joins {
            // Find all of the segments that can be joined because they link at non-confluences.
            for fid in 0..num_polylines {
                // fid = polylines[poly_id].id1;
                p1 = polylines[fid].get_first_node();
                let ret = endnode_tree.locate_within_distance([p1.x, p1.y], precision);

                connected_polyline = num_polylines;
                num_neighbours = 0;
                for p in ret {
                    if p.data != fid {
                        connected_polyline = p.data;
                        num_neighbours += 1;
                    }
                }
                if num_neighbours == 1 {
                    connections[fid][0] = connected_polyline;
                }

                p1 = polylines[fid].get_last_node();
                let ret = endnode_tree.locate_within_distance([p1.x, p1.y], precision);

                connected_polyline = num_polylines;
                num_neighbours = 0;
                for p in ret {
                    if p.data != fid {
                        connected_polyline = p.data;
                        num_neighbours += 1;
                    }
                }
                if num_neighbours == 1 {
                    connections[fid][1] = connected_polyline;
                }

                if configurations.verbose_mode && inputs.len() == 1 {
                    progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
                    let mut old_progress = old_progress.lock().unwrap();
                    if progress != *old_progress {
                        println!("Looking for joins in arcs: {}%", progress);
                        *old_progress = progress;
                    }
                }
            }
        }

        // now perform the actual joins
        let mut marked_for_deletion = vec![false; num_polylines];
        for fid in 0..num_polylines {
            // We're looking for segments where one end is joined and the other end is not. These are 
            // valid starting segements for chains of joined segments.
            // if fid == 21414 || fid == 16471 || fid == 3703 || fid == 3683 {
            //     println!("{} {} {} {} {}", fid, connections[fid][0], connections[fid][1], marked_for_deletion[fid], num_polylines);
            // }
            if !marked_for_deletion[fid] {
                let is_joined_at_start = connections[fid][0] < num_polylines && connections[fid][1] == num_polylines;
                let mut is_joined_at_end = connections[fid][0] == num_polylines && connections[fid][1] < num_polylines;
                // if fid == 21414 || fid == 16471 || fid == 3703 || fid == 3683 {
                //     println!("{} {} {} {} {}", fid, is_joined_at_start, connections[fid][0], is_joined_at_end, connections[fid][1]);
                // }
                if is_joined_at_start || is_joined_at_end {
                    // let flag_high = fid == 3683;
                    marked_for_deletion[fid] = true;
                    // It's a start to a connected chain.
                    let mut pl = Polyline::new_empty(fid);
                    if is_joined_at_end {
                        pl.vertices.extend_from_slice(&polylines[fid].vertices.clone());
                    } else {
                        let mut rev = polylines[fid].vertices.clone();
                        rev.reverse();
                        pl.vertices.extend_from_slice(&rev);
                    }
                    // let mut current_fid = if connections[fid][0] < num_polylines {
                    //     connections[fid][0]
                    // } else {
                    //     connections[fid][1]
                    // };
                    let mut current_fid = fid;
                    loop {
                        // if flag_high {
                        //     let t1 = if connections[current_fid][0] < num_polylines {
                        //         marked_for_deletion[connections[current_fid][0]]
                        //     } else {
                        //         true
                        //     };

                        //     let t2 = if connections[current_fid][1] < num_polylines {
                        //         marked_for_deletion[connections[current_fid][1]]
                        //     } else {
                        //         true
                        //     };

                        //     println!("{} {} {} {} {}", current_fid, connections[current_fid][0], t1, connections[current_fid][1], t2);
                        // }
                        marked_for_deletion[current_fid] = true;
                        // is_joined_at_end = false;
                        current_fid = if connections[current_fid][0] < num_polylines && !marked_for_deletion[connections[current_fid][0]] {
                            connections[current_fid][0]
                        } else if connections[current_fid][1] < num_polylines && !marked_for_deletion[connections[current_fid][1]] {
                            // is_joined_at_end = true;
                            connections[current_fid][1]
                        } else {
                            break;
                        };
                        
                        // which way is it joined?
                        is_joined_at_end = false;
                        if pl.get_last_node().distance(&polylines[current_fid].get_first_node()) <= precision {
                            is_joined_at_end = true;
                        }

                        if is_joined_at_end {
                            pl.vertices.extend_from_slice(&polylines[current_fid].vertices.clone());
                        } else {
                            let mut rev = polylines[current_fid].vertices.clone();
                            rev.reverse();
                            pl.vertices.extend_from_slice(&rev);
                        }
                    }

                    polylines.push(pl);
                }
            }
        }

        for i in (0..num_polylines).rev() {
            if marked_for_deletion[i] {
                polylines.remove(i);
            }
        }

        num_polylines = polylines.len();

        // remove any zero-length segments.
        for fid in 0..num_polylines {
            for i in (1..polylines[fid].len()).rev() {
                if polylines[fid][i].distance(&polylines[fid][i-1]) <= precision {
                    polylines[fid].vertices.remove(i);
                }
            }
        }









        end_nodes = vec![];
        for fid in 0..num_polylines {
            polylines[fid].id = fid;

            segment_num = 0;
            for i in 1..polylines[fid].vertices.len() {
                line_segments.push(
                    GeomWithData::new(
                        Line::new(
                            [polylines[fid].vertices[i-1].x, polylines[fid].vertices[i-1].y],
                            [polylines[fid].vertices[i].x, polylines[fid].vertices[i].y]
                        ),
                        (fid, segment_num)
                    )
                );
                segment_num += 1;
            }

            p1 = polylines[fid].get_first_node();
            end_nodes.push(Location::new(
                [p1.x, p1.y],
                fid
            ));

            p1 = polylines[fid].get_last_node();
            end_nodes.push(Location::new(
                [p1.x, p1.y],
                fid
            ));

            if configurations.verbose_mode && inputs.len() == 1 {
                progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Looking for dangling arcs: {}%", progress);
                    *old_progress = progress;
                }
            }
        }




        let endnode_tree = RTree::bulk_load(end_nodes);
        let line_segments_tree = RTree::bulk_load(line_segments);
        let snap_dist_sq = snap_dist * snap_dist;
        // let mut points: Vec<Point2D>;
        let mut num_vertices: usize;
        let mut min_dist: f64;
        let mut dist: f64;
        let mut point = Point2D::new(0f64, 0f64); // just to satisfy the need to initialize.
        let mut p2: Point2D;
        let mut line_seg: LineSegment;
        let mut line_seg2: LineSegment = LineSegment::new(Point2D::new(0f64, 0f64), Point2D::new(0f64, 0f64));
        let mut joined_feature: usize = 0;
        for poly_id in 0..polylines.len() {
            fid = polylines[poly_id].id;
            p1 = polylines[fid].get_first_node();
            let ret = line_segments_tree.locate_within_distance([p1.x, p1.y], snap_dist_sq);
            // See if any of the line segments within the snap distance are from a different polyline. 
            // If so, find the nearest point.
            min_dist = f64::INFINITY;
            for line in ret {
                if line.data.0 != fid {
                    let geom = line.geom();
                    let p = geom.nearest_point(&[p1.x, p1.y]);
                    p2 = Point2D::new(p[0], p[1]);
                    dist = p1.distance(&p2);
                    if dist < min_dist {
                        min_dist = dist;
                        point = p2;
                        segment_num = line.data.1;
                        joined_feature = line.data.0;
                        line_seg2 = LineSegment::new(
                            Point2D::new(geom.from[0], geom.from[1]),
                            Point2D::new(geom.to[0], geom.to[1])
                        );
                    }
                }
            }

            // how many endnodes is this endnode in contact with? This is for y-junctions
            let ret_endnodes = endnode_tree.locate_within_distance([p1.x, p1.y], precision);
            num_neighbours = 0;
            for p in ret_endnodes {
                if p.data != fid {
                    num_neighbours += 1;
                }
            }

            if (min_dist.is_finite() && min_dist > precision) || (min_dist <= precision && num_neighbours == 0) {
                // Is it an undershoot or an overshoot?
                // if it is an overshoot, then the nearest point will have a distance of zero with 
                // the current line segment too. That is, it will be coincident.
                line_seg = LineSegment::new(p1, polylines[fid][1]);
                
                if (line_seg.dist_to_segment(point) - min_dist).abs() <= precision {
                    // It's an undershoot, add the point to the start of the polyline.
                    polylines[fid].insert(0, point);
                    // all the split indices will be one less than they should be now that we've
                    // inserted a vertex at the start.
                    polylines[fid].splits_offset_by_one = true;
                    polylines[joined_feature].insert_split_point(segment_num, point);
                    // points.push(point);
                } else { // It's an overshoot.
                    point = match line_seg.get_intersection(&line_seg2) {
                        Some(ls) => ls.p1,
                        None => point // do nothing
                    };
                    if polylines[fid][1].distance(&point) > precision {
                        polylines[fid].insert(0, point);
                        polylines[fid].remove(1);
                    }
                    polylines[joined_feature].insert_split_point(segment_num, point);
                }
            }

            p1 = polylines[fid].get_last_node();
            let ret = line_segments_tree.locate_within_distance([p1.x, p1.y], snap_dist_sq);
            min_dist = f64::INFINITY;
            for line in ret {
                if line.data.0 != fid {
                    let geom = line.geom();
                    let p = geom.nearest_point(&[p1.x, p1.y]);
                    p2 = Point2D::new(p[0], p[1]);
                    dist = p1.distance(&p2);
                    if dist < min_dist {
                        min_dist = dist;
                        point = p2;
                        segment_num = line.data.1;
                        joined_feature = line.data.0;
                        line_seg2 = LineSegment::new(
                            Point2D::new(geom.from[0], geom.from[1]),
                            Point2D::new(geom.to[0], geom.to[1])
                        );
                    }
                }
            }

            // how many endnodes is this endnode in contact with? This is for y-junctions
            let ret_endnodes = endnode_tree.locate_within_distance([p1.x, p1.y], precision);
            num_neighbours = 0;
            for p in ret_endnodes {
                if p.data != fid {
                    num_neighbours += 1;
                }
            }

            if (min_dist.is_finite() && min_dist > precision) || (min_dist <= precision && num_neighbours == 0) {
            // if min_dist.is_finite() && min_dist >= precision {
                // Is it an undershoot or an overshoot?
                // if it is an overshoot, then the nearest point will have a distance of zero with 
                // the current line segment too. That is, it will be coincident.
                line_seg = LineSegment::new(polylines[fid][polylines[fid].len()-2], p1);
                if (line_seg.dist_to_segment(point) - min_dist).abs() <= precision {
                    // It's an undershoot, add the line end point.
                    // points.push(record.points[part_end].clone());
                    polylines[fid].push(point);
                    polylines[joined_feature].insert_split_point(segment_num, point);
                } else { // It's an overshoot
                    num_vertices = polylines[fid].len();
                    polylines[fid].remove(num_vertices-1);

                    point = match line_seg.get_intersection(&line_seg2) {
                        Some(ls) => ls.p1,
                        None => point // do nothing
                    };
                    polylines[fid].push(point);
                    polylines[joined_feature].insert_split_point(segment_num, point);
                }
            }

            if configurations.verbose_mode && inputs.len() == 1 {
                progress = (100.0_f64 * (poly_id + 1) as f64 / polylines.len() as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Looking for dangling arcs: {}%", progress);
                    *old_progress = progress;
                }
            }
        }

        // Deal with the splits.
        let mut polylines2 = vec![];
        for poly_id in 0..polylines.len() {
            if polylines[poly_id].split_points.len() == 0 {
                polylines2.push(polylines[poly_id].clone());
            } else {
                let splits = polylines[poly_id].split();
                for pl in splits {
                    polylines2.push(pl.clone());
                }
            }
        }


        // remove any zero-length segments.
        for fid in 0..polylines2.len() {
            for i in (1..polylines2[fid].len()).rev() {
                if polylines2[fid][i].distance(&polylines2[fid][i-1]) <= precision {
                    polylines2[fid].vertices.remove(i);
                }
            }
        }


        // Find segments that have a gap at their endnodes and can be joined.



        // We want line segements to have the same orientation as the input lines. This may not always be
        // possible because two lines may have been joined at their ends (meaning at least one must be reversed)
        // but the majority should follow the same direction
        // Read each line segment into an rtree.
        type Location2 = GeomWithData<[f64; 2], (usize, usize)>;
        let mut vertices = vec![];
        let (mut part_start, mut part_end): (usize, usize);
        let mut polylines = vec![];
        let mut fid = 0;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);        
            for part in 0..record.num_parts as usize {
                part_start = record.parts[part] as usize;
                part_end = if part < record.num_parts as usize - 1 {
                    record.parts[part + 1] as usize - 1
                } else {
                    record.num_points as usize - 1
                };

                polylines.push(
                    Polyline::new(
                        &record.points[part_start..=part_end], 
                        fid
                    )
                );

                for i in part_start..part_end {
                    vertices.push(Location2::new(
                        [record.points[i].x, record.points[i].y],
                        (fid, i)
                    ));
                }

                fid += 1;
            }

            if configurations.verbose_mode && inputs.len() == 1 {
                progress = (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Creating vertex tree: {}%", progress);
                    *old_progress = progress;
                }
            }
        }

        // Find all of the segments that can be joined because they link at non-confluences.
        let vertex_tree = RTree::bulk_load(vertices);
        let mut p1: Point2D;
        let mut p2: Point2D;
        let mut p3: Point2D;
        let mut percent_reverse = vec![0.0; polylines2.len()];
        for fid in 0..polylines2.len() {
            for i in 0..polylines2[fid].len()-1 {
                // get the id of the cooresponding vertex in the original file
                p1 = polylines2[fid].vertices[i];
                let ret = vertex_tree.locate_within_distance([p1.x, p1.y], precision);

                p2 = polylines2[fid].vertices[i+1]; // The next vertex in the output line

                for p in ret { // there should only ever be one
                    let (in_fid, in_vertex) = p.data; // The corresponding point in the input line

                    if in_vertex < polylines[in_fid].len() - 1 {
                        p3 = polylines[in_fid].vertices[in_vertex+1]; // The next vertex in the input line

                        // Are p2 and p3 the same location? If so, then the line order is unchanged, if no,
                        // then it's been reversed.
                        if p2.distance(&p3) > precision {
                            percent_reverse[fid] += 1.0;
                        }
                    }
                }
            }
            
            if configurations.verbose_mode && inputs.len() == 1 {
                progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Looking for joins in arcs: {}%", progress);
                    *old_progress = progress;
                }
            }
        }


        // If more than half of the vertices in a line have been reverse, reverse it back to the orginal order.
        // Remember, output lines may be composed of multiple input lines some of which may have been reversed,
        // while others were not. This voting scheme represents a 'majority' line order.
        for fid in 0..polylines2.len() {
            percent_reverse[fid] = 100.0 * percent_reverse[fid] / (polylines2[fid].len() - 1) as f64;
            if percent_reverse[fid] > 50.0 {
                let mut line = polylines2[fid].vertices.clone();
                line.reverse();
                polylines2[fid].vertices = line;
            }
        }





        if reverse_backward_arcs {
            let mut vertices = vec![];
            let mut p1: Point2D;
            for fid in 0..polylines2.len() {
                p1 = polylines2[fid].get_first_node();
                vertices.push(Location2::new(
                    [p1.x, p1.y],
                    (fid, 1)
                ));

                p1 = polylines2[fid].get_last_node();
                vertices.push(Location2::new(
                    [p1.x, p1.y],
                    (fid, 0)
                ));

                if configurations.verbose_mode && inputs.len() == 1 {
                    progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
                    let mut old_progress = old_progress.lock().unwrap();
                    if progress != *old_progress {
                        println!("Creating endnode tree: {}%", progress);
                        *old_progress = progress;
                    }
                }
            }

            // Find all of the segments that can be joined because they link at non-confluences.
            let tree = RTree::bulk_load(vertices);
            let mut reverse = vec![false; polylines2.len()];
            for fid in 0..polylines2.len() {
                // look for lines that have a neighbouring starting node at the line start and none at the line end.
                let mut has_neighbouring_start_at_start = false;
                p1 = polylines2[fid].get_first_node();
                let ret = tree.locate_within_distance([p1.x, p1.y], precision);
                for p in ret {
                    let (in_fid, is_start) = p.data;
                    if in_fid != fid {
                        if is_start == 1 {
                            has_neighbouring_start_at_start = true;
                            break;
                        }
                    }
                }

                if has_neighbouring_start_at_start {
                    let mut has_neighbouring_start_at_end = false;
                    p1 = polylines2[fid].get_last_node();
                    let ret = tree.locate_within_distance([p1.x, p1.y], precision);
                    for p in ret {
                        let (in_fid, is_start) = p.data;
                        if in_fid != fid {
                            if is_start == 1 {
                                has_neighbouring_start_at_end = true;
                                break;
                            }
                        }
                    }

                    if !has_neighbouring_start_at_end {
                        reverse[fid] = true;
                    }
                }
                
                if configurations.verbose_mode && inputs.len() == 1 {
                    progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
                    let mut old_progress = old_progress.lock().unwrap();
                    if progress != *old_progress {
                        println!("Looking backwards arcs: {}%", progress);
                        *old_progress = progress;
                    }
                }
            }

            let mut num_reversed = 0;
            for fid in 0..polylines2.len() {
                if reverse[fid] {
                    let mut line = polylines2[fid].vertices.clone();
                    line.reverse();
                    polylines2[fid].vertices = line;
                    num_reversed += 1;
                }
            }
            println!("num. reversed arcs: {num_reversed}");

            // create output file
            let mut output = Shapefile::initialize_using_file(&output_file.replace(".shp", "_reversed_arcs.shp"), &input, ShapeType::PolyLine, false).expect("Error creating output file");

            // add the attributes
            output.attributes.add_field(
                &AttributeField::new(
                    "FID", 
                    FieldDataType::Int, 
                    7u8, 
                    0u8
                )
            );

            let mut sfg: ShapefileGeometry;
            for fid in 0..polylines2.len() {
                if reverse[fid] {
                    sfg = ShapefileGeometry::new(ShapeType::PolyLine); 
                    sfg.add_part(&polylines2[fid].vertices);
                    output.add_record(sfg);
                    output.attributes.add_record(vec![FieldData::Int((fid + 1) as i32)], false);
                }
            }

            output.write().expect("Error writing file.");
        }

        // create output file
        let mut output = Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, false).expect("Error creating output file"); //?;

        // add the attributes
        // let in_atts = input.attributes.get_fields();

        // output.attributes.add_fields(&in_atts);
        output.attributes.add_field(
            &AttributeField::new(
                "FID", 
                FieldDataType::Int, 
                7u8, 
                0u8
            )
        );
        
        let mut sfg: ShapefileGeometry;
        // let mut record_num: usize;
        for poly_id in 0..polylines2.len() {
            sfg = ShapefileGeometry::new(ShapeType::PolyLine); 
            sfg.add_part(&polylines2[poly_id].vertices);
            output.add_record(sfg);

            // record_num = polylines2[poly_id].id2;
            // let att_data = input.attributes.get_record(record_num);
            // output.attributes.add_record(att_data.clone(), false);
            output.attributes.add_record(vec![FieldData::Int((poly_id + 1) as i32)], false);

            if configurations.verbose_mode && inputs.len() == 1 {
                progress =  (100.0_f64 * (poly_id + 1) as f64 / polylines2.len() as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Looking for dangling arcs: {}%", progress);
                    *old_progress = progress;
                }
            }
        }

        {
            let mut output_num_features = output_num_features.lock().expect("Error unlocking output_num_features.");
            *output_num_features += polylines2.len();
        }
        

        if configurations.verbose_mode && inputs.len() == 1 {
            println!("Saving data...")
        };

        output.write().expect("Error writing file.");


        if inputs.len() > 1 {
            {
                let mut tiles_completed = tiles_completed.lock().unwrap();
                *tiles_completed += 1;
                progress =  (100.0_f64 * *tiles_completed as f64 / inputs.len() as f64) as usize;
                let mut old_progress = old_progress.lock().unwrap();
                if progress != *old_progress {
                    println!("Progress: {}%", progress);
                    *old_progress = progress;
                }
            }
        }
    });

    let input_num_features = input_num_features.lock().expect("Error locking output_num_features.");
    println!("Num. input line features: {}", *input_num_features);

    let output_num_features = output_num_features.lock().expect("Error locking output_num_features.");
    println!("Num. output line features: {}", *output_num_features);


    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "\n{}",
            &format!("Elapsed Time (Including I/O): {}", elapsed_time)
        );
    }

    
    Ok(())
}

#[derive(Default, Clone, Debug)]
struct Polyline {
    vertices: Vec<Point2D>,
    id: usize,
    pub split_points: Vec<(usize, Point2D, f64)>,
    splits_offset_by_one: bool,
}

impl Index<usize> for Polyline {
    type Output = Point2D;

    fn index<'a>(&'a self, index: usize) -> &'a Point2D {
        &self.vertices[index]
    }
}
 
impl Polyline {
    // Creates a new Polyline from vertices
    fn new(vertices: &[Point2D], id: usize) -> Self {
        Polyline {
            vertices: vertices.to_vec(),
            id,
            split_points: vec![],
            splits_offset_by_one: false,
        }
    }

    // Creates a new empty Polyline
    fn new_empty(id: usize) -> Polyline {
        Polyline {
            vertices: vec![], 
            id,
            split_points: vec![],
            splits_offset_by_one: false,
        }
    }

    // returns the number of vertices
    fn len(&self) -> usize {
        self.vertices.len()
    }

    // Inserts a point vertex at the end of the line.
    fn push(&mut self, v: Point2D) {
        self.vertices.push(v);
    }

    // Inserts a point vertex at a specific index.
    fn insert(&mut self, index: usize, v: Point2D) {
        if index <= self.len() {
            self.vertices.insert(index, v);
        }
    }

    // Removes a point vertex at a specified index.
    fn remove(&mut self, index: usize) {
        if index <= self.len() {
            self.vertices.remove(index);
        }
    }

    fn insert_split_point(&mut self, position: usize, point: Point2D) {
        if position < self.len() - 1 { // position >= 0 && 
            self.split_points.push((position, point, 0f64));
        }
    }

    fn split(&mut self) -> Vec<Self> {
        // if there is an offset value it is because a vertex was added to the start of the polyline
        if self.splits_offset_by_one {
            for split in 0..self.split_points.len() {
                self.split_points[split].0 += 1;
            }
        }

        // make sure there are no duplicate splits
        for split in (1..self.split_points.len()).rev() {
            if self.split_points[split].0 == self.split_points[split - 1].0 && 
            self.split_points[split].1 == self.split_points[split - 1].1 {
                self.split_points.remove(split);
            }
        }

        // calculate cumulative segment distances at the start of the segment
        let mut segment_distances = Vec::with_capacity(self.len());
        segment_distances.push(0f64);
        for i in 1..self.len() {
            segment_distances.push(segment_distances[i-1] + self[i-1].distance(&self[i]));
        }

        // now calculate the cumulative distance from the start of the polyline of the split points.
        let mut dist: f64;
        for split in 0..self.split_points.len() {
            dist = segment_distances[self.split_points[split].0] + self[self.split_points[split].0].distance(&self.split_points[split].1);
            self.split_points[split].2 = dist;
        }

        // This is a problem because we also need to sort the points by distance.
        self.split_points
            .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap()); 

        
        // perform the split
        let mut ret: Vec<Polyline> = Vec::with_capacity(self.split_points.len() + 1);
        if self.split_points.len() > 0 {
            let mut current_split = 0;
            let mut line = Polyline::new_empty(self.id);
            for node in 0..self.len() {
                if current_split == self.split_points.len() || node < self.split_points[current_split].0 {
                    line.push(self[node]);
                } else {
                    line.push(self[node]);
                    line.push(self.split_points[current_split].1);
                    ret.push(line);
                    line = Polyline::new_empty(self.id);
                    line.push(self.split_points[current_split].1);

                    // current_split += 1;

                    // Deal with segments with multiple splits.
                    let former_node = self.split_points[current_split].0;
                    loop {
                        current_split += 1;

                        if current_split < self.split_points.len() && former_node == self.split_points[current_split].0 {
                            line.push(self.split_points[current_split].1);
                            ret.push(line);
                            line = Polyline::new_empty(self.id);
                            line.push(self.split_points[current_split].1);
                        } else {
                            break;
                        }
                    }
                }
            }
            ret.push(line);
        }

        ret
    }

    fn get_first_node(&self) -> Point2D {
        self[0]
    }

    fn get_last_node(&self) -> Point2D {
        self[self.vertices.len() - 1]
    }
}