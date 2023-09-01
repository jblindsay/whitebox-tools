/* 
Authors: Prof. John Lindsay
Created: 28/07/2021 (oringinally in Whitebox Toolset Extension)
Last Modified: 23/03/2023
License: MIT
*/

use rstar::primitives::GeomWithData;
use rstar::RTree;
use std::io::{Error, ErrorKind};
use std::{env, path, str};
use std::time::Instant;
use std::ops::Index;
use std::collections::VecDeque;
const EPSILON: f64 = std::f64::EPSILON;
use whitebox_common::utils::{
    get_formatted_elapsed_time,
    haversine_distance,
    wrapped_print
};
use whitebox_common::structures::Point2D;
use whitebox_vector::*;

/// This tool performs common stream network analysis operations on an input vector stream file (`--streams`).
/// The network indices produced by this analysis are contained within the output vector's (`--output`)
/// attribute table. The following table shows each of the network indices that are calculated.
/// 
/// | Index Name | Description |
/// | :- | :- |
/// | OUTLET | Unique outlet identifying value, used as basin identifier | 
/// | TRIB_ID | Unique tributary identifying value | 
/// | DIST2MOUTH | Distance to outlet (i.e., mouth node) | 
/// | DS_NODES | Number of downstream nodes | 
/// | TUCL | Total upstream channel length; the channel equivalent to catchment area | 
/// | MAXUPSDIST | Maximum upstream distance | 
/// | HORTON | Horton stream order | 
/// | STRAHLER | Strahler stream order | 
/// | SHREVE | Shreve stream magnitude | 
/// | HACK | Hack stream order | 
/// | MAINSTREAM | Boolean value indicating whether link is the main stream trunk of its basin | 
/// | MIN_ELEV | Minimum link elevation (from DEM) | 
/// | MAX_ELEV | Maximum link elevation (from DEM) | 
/// | IS_OUTLET | Boolean value indicating whether link is an outlet link |  
///
/// In addition to the input and output files, the user must also specify the name of an input DEM file 
/// (`--dem`), the maximum ridge-cutting height, in DEM z units (`--cutting_height`), and the snap distance
/// used for identifying any topological errors in the stream file (`--snap`).  The main function of the 
/// input DEM is to distinguish between outlet and headwater links in the network, which
/// can be differentiated by their elevations during the priority-flood operation used in the algorithm 
/// (see Lindsay et al. 2019). The maximum ridge-cutting height parameter is useful for preventing 
/// erroneous stream capture in the headwaters when channel heads are very near (within the sanp distance),
/// which is usually very rare. The snap distance parameter is used to deal with certain common topological
/// errors. However, it is advisable that the input streams file be pre-processed prior to analysis.
///
/// > Note: The input streams file for this tool should be pre-processed using the `RepairStreamVectorTopology`
/// > tool. **This is an important step**.
///
/// OUTLET:
/// ![](../../doc_img/StreamVectorAnalysis1.png)
///
/// HORTON:
/// ![](../../doc_img/StreamVectorAnalysis2.png)
///
/// SHREVE:
/// ![](../../doc_img/StreamVectorAnalysis4.png)
///
/// TRIB_ID:
/// ![](../../doc_img/StreamVectorAnalysis3.png)
///
///Many of the network indices output by this tool for vector streams have raster equivalents in WhiteboxTools.
/// For example, see the `StrahlerStreamOrder`, `ShreveStreamMagnitude` tools.
///
/// # Reference
/// Lindsay, JB, Yang, W, Hornby, DD. 2019. Drainage network analysis and structuring of topologically 
/// noisy vector stream data. ISPRS International Journal of Geo-Information. 8(9), 422; DOI: 
/// 10.3390/ijgi8090422
///
/// # See Also
/// `RepairStreamVectorTopology`, `StrahlerStreamOrder`, `ShreveStreamMagnitude`
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

    let exe_name = &format!("vector_stream_network_analysis{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    vector_stream_network_analysis Help

    This tool can be used to 

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    --streams          Name of the input streams vector.
    --dem              Name of the input DEM raster file.
    -o, --output       Name of the output lines shapefile.
    --cutting_height   Maximum ridge-cutting height (z units).
    --snap             Snap distance, in xy units (metres).
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --streams=rivers.shp --dem=DEM.tif -o=network_analysis.shp --cutting_height=10.0 --snap=1.0

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
        "vector_stream_network_analysis v{} by Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("VectorStreamNetworkAnalysis") // This should be camel case and is a reference to the tool name.
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

    let mut streams_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut snap_distance = 0.001;

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
        if flag_val == "-streams" {
            streams_file = if keyval {
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
            snap_distance = if keyval {
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

    let mut progress: usize;
    let mut old_progress: usize = 1;

    let start = Instant::now();

    let snap_distance = snap_distance * snap_distance;
    
    
    if !streams_file.contains(&sep) && !streams_file.contains("/") {
        streams_file = format!("{}{}", working_directory, streams_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    let input = Shapefile::read(&streams_file)?;
    
    // Make sure the input vector file is of polygon type
    if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The input vector data must be of PolyLine base shape type.",
        ));
    }


    // create output file
    let mut output = Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, false)?;
    
    // add the attributes

    let fields_vec: Vec<AttributeField> = vec![
        AttributeField::new(
            "FID", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "TUCL", 
            FieldDataType::Real, 
            10u8, 
            4u8
        ),
        AttributeField::new(
            "MAXUPSDIST", 
            FieldDataType::Real, 
            10u8, 
            4u8
        ),
        AttributeField::new(
            "OUTLET", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "HORTON", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "STRAHLER", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "SHREVE", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "HACK", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "DIST2MOUTH", 
            FieldDataType::Real, 
            10u8, 
            4u8
        ),
        AttributeField::new(
            "DS_NODES", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "IS_OUTLET", 
            FieldDataType::Int, 
            1u8, 
            0u8
        ),
        AttributeField::new(
            "DS_LINK_ID", 
            FieldDataType::Int, 
            7u8, 
            0u8
        ),
        AttributeField::new(
            "MAINSTEM", 
            FieldDataType::Int, 
            1u8, 
            0u8
        ),
        AttributeField::new(
            "TRIB_ID", 
            FieldDataType::Int, 
            7u8, 
            0u8
        )
    ];



    // let in_atts = input.attributes.clone();
    // let mut parent_fid_att = 999;
    // for i in 0..in_atts.fields.len() {
    //     let field = in_atts.get_field(i);
    //     if field.name == "FID" {
    //         parent_fid_att = i;
    //     } else {
    //         fields_vec.push(field.clone());
    //     }
    // }

    output.attributes.add_fields(&fields_vec);

    let mut output_confluences = Shapefile::initialize_using_file(&output_file.replace(".shp", "_confluences.shp"), &input, ShapeType::Point, false)?;
    output_confluences
        .attributes
        .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));


    let mut output_channel_heads = Shapefile::initialize_using_file(&output_file.replace(".shp", "_channelHeads.shp"), &input, ShapeType::Point, false)?;
    output_channel_heads
        .attributes
        .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));

    let mut output_outlets = Shapefile::initialize_using_file(&output_file.replace(".shp", "_outlets.shp"), &input, ShapeType::Point, false)?;
    output_outlets
        .attributes
        .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));

    // count the number of parts
    let mut total_num_parts = 0;
    for record_num in 0..input.num_records {
        let record = input.get_record(record_num);        
        total_num_parts += record.num_parts as usize;
    }

    let mut link_mag = vec![0f64; total_num_parts];
    
    let mut link_lengths = vec![0f64; total_num_parts];
    let mut outlet_nums = vec![0; total_num_parts];
    let mut num_downstream_nodes = vec![0; total_num_parts];

    let mut downstream_link = vec![-99; total_num_parts];

    let is_geographic_proj = if input.header.x_min.abs() <= 180.0 && input.header.x_max.abs() <= 180.0 && input.header.y_min.abs() < 90.0 && input.header.y_max.abs() <= 90.0 {
        // it's likely in geographic coordinates.
        wrapped_print("Warning: It appears that the input data is in geographic coordinates. This tool will run better on projected point data.", 50);
        true
    } else {
        false
    };

    // Start by finding the outlet point(s).

    // Read each line segment into an rtree.
    type Location = GeomWithData<[f64; 2], (usize, bool)>;
    let mut end_nodes = vec![];
    let (mut part_start, mut part_end): (usize, usize);
    let mut fid = 0usize; // fid is unique to each part in the vector
    let mut polylines = vec![];
    let mut length: f64;
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
                    &record.points[part_start..=part_end]
                )
            );

            end_nodes.push(Location::new(
                [record.points[part_start].x, record.points[part_start].y],
                (fid, true)
            ));

            end_nodes.push(Location::new(
                [record.points[part_end].x, record.points[part_end].y],
                (fid, false)
            ));

            // calculate the length of this line
            length = 0.0;
            for i in part_start+1..=part_end {
                length += if !is_geographic_proj {
                    record.points[i].distance(&record.points[i-1])
                } else {
                    let phi1 = record.points[i].y;
                    let lambda1 = record.points[i].x;

                    let phi2 = record.points[i-1].y;
                    let lambda2 = record.points[i-1].x;

                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                };
            }
            link_lengths[fid] = length;

            fid += 1;
        }

        if configurations.verbose_mode {
            progress = (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
            if progress != old_progress {
                println!("Reading vector: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let snap_dist_sq = snap_distance * snap_distance;
    let endnode_tree = RTree::bulk_load(end_nodes);
    let precision = EPSILON * 10f64;
    let mut head_pt: Point2D;
    let mut outlet_pts = vec![];
    // let mut channel_head_pts = vec![];
    let mut channel_head_list = vec![];
    let mut outlet_list = vec![];
    let mut p1: Point2D;

    // let mut is_exterior_link = vec![false; total_num_parts];
    // let mut is_exterior: bool;
    let mut is_outlet_link = vec![false; total_num_parts];
    // let mut num_infowing_links = vec![0; total_num_parts];
    let mut dist_to_outlet = vec![0f64; total_num_parts];
    let mut trib_num = vec![0; total_num_parts];
    let mut strahler_order = vec![0usize; total_num_parts];
    let mut shreve_order = vec![0usize; total_num_parts];
    let mut max_upstream_length = vec![0f64; total_num_parts];
    let mut is_main_stem = vec![false; total_num_parts];
    let mut horton_order = vec![0usize; total_num_parts];
    let mut hack_order = vec![0usize; total_num_parts];

    for fid in 0..total_num_parts {
        p1 = polylines[fid].get_last_node();
        let ret = endnode_tree.locate_within_distance([p1.x, p1.y], snap_dist_sq);
        let mut num_downstream_arcs = 0;
        for pt in ret {
            let (fid_n, is_start) = pt.data;
            if fid_n != fid && is_start {
                num_downstream_arcs += 1;
            }
        }

        if num_downstream_arcs == 0 {
            outlet_pts.push(p1);
            // outlet_list.push(fid);

            output_outlets.add_point_record(p1.x, p1.y);
            output_outlets.attributes.add_record(vec![FieldData::Int(fid as i32)], false);
        }

        p1 = polylines[fid].get_first_node();
        let ret = endnode_tree.locate_within_distance([p1.x, p1.y], snap_dist_sq);
        let mut num_neighbours = 0;
        for pt in ret {
            let (fid_n, _is_start) = pt.data;
            if fid_n != fid {
                num_neighbours += 1;
            }
        }
        
        if num_neighbours == 0 {
            strahler_order[fid] = 1;
            shreve_order[fid] = 1;
            output_channel_heads.add_point_record(p1.x, p1.y);
            output_channel_heads.attributes.add_record(vec![FieldData::Int(fid as i32)], false);
        } else {
            output_confluences.add_point_record(p1.x, p1.y);
            output_confluences.attributes.add_record(vec![FieldData::Int(fid as i32)], false);
        }

        if configurations.verbose_mode {
            progress = (100.0_f64 * (fid + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Finding outlets: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // Perform a network traverse on the streams connected to each outlet.
    let mut visited = vec![false; total_num_parts];
    for outlet in 0..outlet_pts.len() {
        head_pt = outlet_pts[outlet];
        let ret = endnode_tree.locate_within_distance([head_pt.x, head_pt.y], snap_dist_sq);
        
        for pt in ret {
            let (fid, is_start) = pt.data;
            
            if !visited[fid] {
                visited[fid] = true;

                outlet_list.push(fid);
                is_outlet_link[fid] = true;

                outlet_nums[fid] = outlet + 1;
                num_downstream_nodes[fid] = 1;

                // now find all connected line segments
                let mut queue: VecDeque<(usize, bool)> = VecDeque::with_capacity(total_num_parts);
                queue.push_back((fid, is_start));
                while !queue.is_empty() {
                    let (fid2, is_start2) = queue.pop_front().unwrap();

                    // Find the point associated with the other end of this polyline
                    p1 = if !is_start2 {
                        polylines[fid2].get_first_node()
                    } else {
                        polylines[fid2].get_last_node()
                    };

                    // Find the neighbouring endnodes of p1
                    let ret2 = endnode_tree.locate_within_distance([p1.x, p1.y], precision);

                    for pt2 in ret2 {
                        let (fid_n, is_start_n) = pt2.data;
                        if fid_n != fid2 && !visited[fid_n] {
                            // Add this newly encountered polyline to the queue
                            queue.push_back((fid_n, is_start_n));
                            visited[fid_n] = true;
                            dist_to_outlet[fid_n] = dist_to_outlet[fid2] + link_lengths[fid_n];
                            outlet_nums[fid_n] = outlet + 1;
                            downstream_link[fid_n] = fid2 as isize;
                            num_downstream_nodes[fid_n] += num_downstream_nodes[fid2] + 1;
                        }
                    }
                }
            }
        }
    }


    // How many inflowing links?
    let mut num_inflowing = vec![0; total_num_parts];
    for n in 0..total_num_parts {
        if downstream_link[n] >= 0 {
            num_inflowing[downstream_link[n] as usize] += 1;
        }
    }

    for n in 0..total_num_parts {
        link_mag[n] = link_lengths[n];
        if num_inflowing[n] == 0 {
            channel_head_list.push(n);
        }
    }


    let mut ds_queue: VecDeque<isize> = VecDeque::with_capacity(total_num_parts);
    let mut dsl: isize;
    for head in 0..channel_head_list.len() {
        fid = channel_head_list[head];
        ds_queue.push_back(fid as isize);
        shreve_order[fid] = 1;
        strahler_order[fid] = 1;
        max_upstream_length[fid] = link_lengths[fid];
        trib_num[fid] = fid;
    }

    while !ds_queue.is_empty() {
        let fid = ds_queue.pop_front().unwrap();
        max_upstream_length[fid as usize] += link_lengths[fid as usize];
        dsl = downstream_link[fid as usize];
        if dsl >= 0 {
            shreve_order[dsl as usize] += shreve_order[fid as usize];
            if strahler_order[dsl as usize] < strahler_order[fid as usize] {
                strahler_order[dsl as usize] = strahler_order[fid as usize];
            } else if strahler_order[dsl as usize] == strahler_order[fid as usize] {
                strahler_order[dsl as usize] += 1;
            }

            link_mag[dsl as usize] += link_mag[fid as usize];

            if max_upstream_length[dsl as usize] < max_upstream_length[fid as usize] {
                max_upstream_length[dsl as usize] = max_upstream_length[fid as usize];
                trib_num[dsl as usize] = trib_num[fid as usize];
            }
            
            num_inflowing[dsl as usize] -= 1;
            if num_inflowing[dsl as usize] == 0 {
                ds_queue.push_back(dsl);
            }
        }
    }


    // Is main stem
    for n in 0..total_num_parts {
        if outlet_nums[n] > 0 {
            let outlet_link = outlet_list[outlet_nums[n]-1];
            if trib_num[n] == trib_num[outlet_link] {
                is_main_stem[n] = true;
            }
        }
    }

    // Calculate Horton order
    // First find the maximum Strahler order for each tributary
    let mut trib_max_strahler = vec![0; total_num_parts];
    for n in 0..total_num_parts {
        let so = strahler_order[n];
        let trib = trib_num[n];
        if so > trib_max_strahler[trib] {
            trib_max_strahler[trib] = so;
        }
    }

    for n in 0..total_num_parts {
        let trib = trib_num[n];
        horton_order[n] = trib_max_strahler[trib];
    }
    drop(trib_max_strahler);
    
    // Calculate Hack order
    let mut visited = vec![false; total_num_parts];
    for outlet in 0..outlet_pts.len() {
        head_pt = outlet_pts[outlet];
        let ret = endnode_tree.locate_within_distance([head_pt.x, head_pt.y], snap_dist_sq);
        
        for pt in ret {
            let (fid, is_start) = pt.data;
            
            if !visited[fid] {
                visited[fid] = true;

                hack_order[fid] = 1;

                // now find all connected line segments
                let mut queue: VecDeque<(usize, bool)> = VecDeque::with_capacity(total_num_parts);
                queue.push_back((fid, is_start));
                while !queue.is_empty() {
                    let (fid2, is_start2) = queue.pop_front().unwrap();
                    let trib1 = trib_num[fid2];

                    // Find the point associated with the other end of this polyline
                    p1 = if !is_start2 {
                        polylines[fid2].get_first_node()
                    } else {
                        polylines[fid2].get_last_node()
                    };

                    // Find the neighbouring endnodes of p1
                    let ret2 = endnode_tree.locate_within_distance([p1.x, p1.y], precision);

                    for pt2 in ret2 {
                        let (fid_n, is_start_n) = pt2.data;
                        if fid_n != fid2 && !visited[fid_n] {
                            // Add this newly encountered polyline to the queue
                            queue.push_back((fid_n, is_start_n));
                            visited[fid_n] = true;
                            let trib2 = trib_num[fid_n];
                            if trib2 == trib1 {
                                hack_order[fid_n] = hack_order[fid2];
                            } else {
                                hack_order[fid_n] = hack_order[fid2] + 1;
                            }
                        }
                    }
                }
            }
        }
    }


    // Output the data into the attribute table.
    let mut feature_num = 0;
    let mut count = 0;
    let mut att_data: Vec<FieldData>;
    let mut fid = 1;
    for rec_num in 0..input.num_records {
        let record = input.get_record(rec_num);
        for part in 0..record.num_parts as usize {
            part_start = record.parts[part] as usize;
            part_end = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };
            let mut points: Vec<Point2D> = vec![];
            for i in part_start..=part_end {
                points.push(record.points[i].clone());
            }
            let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
            sfg.add_part(&points);
            output.add_record(sfg);

            att_data = Vec::with_capacity(fields_vec.len());
            att_data.push(FieldData::Int(fid as i32));
            att_data.push(FieldData::Real(link_mag[feature_num]));
            att_data.push(FieldData::Real(max_upstream_length[feature_num]));
            att_data.push(FieldData::Int(outlet_nums[feature_num] as i32));
            att_data.push(FieldData::Int(horton_order[feature_num] as i32));
            att_data.push(FieldData::Int(strahler_order[feature_num] as i32));
            att_data.push(FieldData::Int(shreve_order[feature_num] as i32));
            att_data.push(FieldData::Int(hack_order[feature_num] as i32));
            att_data.push(FieldData::Real(dist_to_outlet[feature_num]));
            att_data.push(FieldData::Int(num_downstream_nodes[feature_num] as i32));
            if is_outlet_link[feature_num] {
                att_data.push(FieldData::Int(1));
            } else {
                att_data.push(FieldData::Int(0));
            }
            att_data.push(FieldData::Int(downstream_link[feature_num] as i32 + 1));
            if is_main_stem[feature_num] {
                att_data.push(FieldData::Int(1));
            } else {
                att_data.push(FieldData::Int(0));
            }
            att_data.push(FieldData::Int(trib_num[feature_num] as i32));

            output.attributes.add_record(att_data.clone(), false);
            fid += 1;
            feature_num += 1;
        }

        count += 1;
        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (count + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Writing data: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if configurations.verbose_mode {
        println!("Saving data...")
    };
    let _ = match output.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output stream file written")
            }
        }
        Err(e) => return Err(e),
    };

    let _ = match output_confluences.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output confluences file written")
            }
        }
        Err(e) => return Err(e),
    };

    let _ = match output_outlets.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output outlets file written")
            }
        }
        Err(e) => return Err(e),
    };
    
    let _ = match output_channel_heads.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output channel heads file written")
            }
        }
        Err(e) => return Err(e),
    };
    
    
    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "{}",
            &format!("Elapsed Time (including I/O): {}", elapsed_time)
        );
    }

    Ok(())
}


#[derive(Default, Clone, Debug)]
struct Polyline {
    vertices: Vec<Point2D>
    // id: usize,
}

impl Index<usize> for Polyline {
    type Output = Point2D;

    fn index<'a>(&'a self, index: usize) -> &'a Point2D {
        &self.vertices[index]
    }
}
 
impl Polyline {
    // Creates a new Polyline from vertices
    fn new(vertices: &[Point2D]) -> Self {
        Polyline {
            vertices: vertices.clone().to_vec()
        }
    }

    fn get_first_node(&self) -> Point2D {
        self[0]
    }

    fn get_last_node(&self) -> Point2D {
        self[self.vertices.len() - 1]
    }
}