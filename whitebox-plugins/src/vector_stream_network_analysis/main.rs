/* 
Authors: Prof. John Lindsay
Created: 28/07/2021 (oringinally in Whitebox Toolset Extension)
Last Modified: 23/03/2023
License: MIT
*/

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::io::{Error, ErrorKind};
use std::{env, path, str};
use std::time::Instant;
use std::sync::Arc;
const EPSILON: f64 = std::f64::EPSILON;
use whitebox_common::utils::{get_formatted_elapsed_time};
use whitebox_common::structures::Point2D;
use whitebox_raster::*;
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
    let mut dem_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut max_ridge_cutting_height = 10.0;
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
        } else if flag_val == "-dem" {
            dem_file = if keyval {
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
        } else if flag_val == "-cutting_height" {
            max_ridge_cutting_height = if keyval {
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

    let precision = EPSILON * 10f64;
    
    
    if !streams_file.contains(&sep) && !streams_file.contains("/") {
        streams_file = format!("{}{}", working_directory, streams_file);
    }
    if !dem_file.contains(&sep) && !dem_file.contains("/") {
        dem_file = format!("{}{}", working_directory, dem_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    // Read in the DEM file
    let dem = Arc::new(Raster::new(&dem_file, "r")?);
    // let rows = dem.configs.rows as isize;
    // let columns = dem.configs.columns as isize;
    let nodata = dem.configs.nodata;

    let mut dist_multiplier = 1.0;
    if dem.is_in_geographic_coordinates() {
        // calculate a new z-conversion factor
        let mut mid_lat = (dem.configs.north - dem.configs.south) / 2.0;
        if mid_lat <= 90.0 && mid_lat >= -90.0 {
            mid_lat = mid_lat.to_radians();
            // z_factor = 1.0 / (111320.0 * mid_lat.cos());
            let a = 6378137.0; 
            let b = 6356752.314;
            let e2 = (a * a - b * b) / (a * a);
            let num = std::f64::consts::PI * a * mid_lat.cos();
            let denum = 180.0 * ((1.0 - e2 * mid_lat.sin() * mid_lat.sin())).sqrt();
            let long_deg_dist = num / denum;
            let lat_deg_dist = 111132.954 - 559.822 * (2.0f64 * mid_lat).cos() + 1.175 * (4.0f64 * mid_lat).cos();
            dist_multiplier = (long_deg_dist + lat_deg_dist) / 2.0;
        }
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
            "MIN_ELEV", 
            FieldDataType::Real, 
            10u8, 
            4u8
        ),
        AttributeField::new(
            "MAX_ELEV", 
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

    // First enter the line end-nodes into a kd-tree
    let num_features = input.num_records;
    // let mut count = 0;
    let (mut part_start, mut part_end): (usize, usize);
    let mut outlet_num: usize;
    let mut total_num_parts = 0;
    let mut is_beyond_edge_line: bool;
    // let mut is_interior: bool;
    // let mut flag: bool;
    let (mut row, mut col): (isize, isize);
    let (mut z, mut z1, mut z2): (f64, f64, f64);
    let mut length: f64;

    if configurations.verbose_mode {
        println!("Pre-processing...");
    }

    // count the number of parts
    for record_num in 0..input.num_records {
        let record = input.get_record(record_num);        
        total_num_parts += record.num_parts as usize;
    }

    let mut crosses_nodata = vec![false; total_num_parts];
    let mut link_mag = vec![0f64; total_num_parts];
    let mut is_beyond_edge = vec![false; total_num_parts];

    let mut link_key_points = Vec::with_capacity(total_num_parts);
    
    let mut link_lengths = vec![0f64; total_num_parts];
    let mut outlet_nums = vec![0; total_num_parts];
    let mut num_downstream_nodes = vec![0; total_num_parts];
    let mut points_tree = KdTree::with_capacity(2, 64);

    let mut link_min_elev = vec![f64::INFINITY; total_num_parts];
    let mut link_max_elev = vec![f64::NEG_INFINITY; total_num_parts];
    let mut downstream_link = vec![-99; total_num_parts];

    let (mut x, mut x1, mut x2, mut y, mut y1, mut y2): (f64, f64, f64, f64, f64, f64);
    
    // Read the end-nodes into the KD-tree. 
    let mut feature_num = 0;
    for rec_num in 0..input.num_records {
        let record = input.get_record(rec_num);        
        for part in 0..record.num_parts as usize {
            part_start = record.parts[part] as usize;
            part_end = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };

            // Is this line off the edge of the DEM or within an area of nodata?
            is_beyond_edge_line = true;

            z1 = 0f64;
            z2 = 0f64;
            for i in part_start..=part_end {
                row = dem.get_row_from_y(record.points[i].y);
                col = dem.get_column_from_x(record.points[i].x);
                z = dem.get_value(row, col);
                if i == part_start { z1 = z; }
                if i == part_end { z2 = z; }
                
                if z != nodata {
                    is_beyond_edge_line = false;
                    if z < link_min_elev[feature_num] { link_min_elev[feature_num] = z}
                    if z > link_max_elev[feature_num] { link_max_elev[feature_num] = z}
                } else {
                    crosses_nodata[feature_num] = true;
                }
            }
            
            if is_beyond_edge_line {
                is_beyond_edge[feature_num] = true;
            } else {
                // calculate the length of this line
                length = 0.0;
                for i in part_start+1..=part_end {
                    length += dist_multiplier * record.points[i].distance(&record.points[i-1]); // Math.sqrt((points[i][0] - points[i - 1][0]) * (points[i][0] - points[i - 1][0]) + (points[i][1] - points[i - 1][1]) * (points[i][1] - points[i - 1][1]))
                }
                link_lengths[feature_num] = length;
            }

            x1 = record.points[part_start].x;
            y1 = record.points[part_start].y;
            points_tree.add([x1, y1], feature_num).unwrap();
            
            x2 = record.points[part_end].x;
            y2 = record.points[part_end].y;
            points_tree.add([x2, y2], feature_num).unwrap();

            link_key_points.push(StreamLinkKeyPoints::new(x1, y1, z1, x2, y2, z2));
            
            feature_num += 1;
        }

        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (rec_num + 1) as f64 / num_features as f64) as usize;
            if progress != old_progress {
                println!("Building search tree: {}%", progress);
                old_progress = progress;
            }
        }
    }


    /*
    * Now we must find y-junctions. This occurs where
    * a stream link's end node intersects with another 
    * stream link but not at one of its end-nodes. Instead,
    * it touches one of its intermediate nodes. We will
    * perform a NN search at the location of all 
    * intermediate nodes and wherever one is within the 
    * search distance of an end-node (already in the kd-tree)
    * then it will be added to the kd-tree as well.
    */
    // let mut num_y_junctions = 0;
    feature_num = 0;
    for rec_num in 0..input.num_records {
        let record = input.get_record(rec_num);        
        for part in 0..record.num_parts as usize {
            part_start = record.parts[part] as usize;
            part_end = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };
            for i in part_start+1..part_end {
                let ret = points_tree.within(&[record.points[i].x, record.points[i].y], precision, &squared_euclidean).unwrap();
                
                if ret.len() > 0 {
                    // add it to the tree
                    points_tree.add([record.points[i].x, record.points[i].y], feature_num).unwrap();
                    // num_y_junctions += 1;

                    link_key_points[feature_num].add_intermediate_point(record.points[i].x, record.points[i].y);

                    output_confluences.add_point_record(record.points[i].x, record.points[i].y);
                    output_confluences.attributes.add_record(vec![FieldData::Int(2i32)], false);
                }
            }

            feature_num += 1;
        }

        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (rec_num + 1) as f64 / num_features as f64) as usize;
            if progress != old_progress {
                println!("Building search tree: {}%", progress);
                old_progress = progress;
            }
        }
    }

    /*
    *  Exterior links can be identified 
    *  as lines that either do not connect to another
    *  or that have at least one end-node with a NoData
    *  elevation value. Exterior links include both 
    *  channel heads (first-order stream) and outlet links.
    *  Add each of these to a priority queue.
    */

    let mut queue = BinaryHeap::with_capacity(total_num_parts);

    // let mut is_channel_head = vec![false; total_num_parts];
    let mut is_exterior_link = vec![false; total_num_parts];
    let mut is_exterior: bool;
    let mut is_outlet_link = vec![false; total_num_parts];
    let mut id: usize;
    let mut j: usize;
    for i in 0..total_num_parts {
        if !is_beyond_edge[i] {
            z = f64::INFINITY;
            /*
            * To be an exterior link, it must have 
            * at least one end that either isn't connected
            * to any other link, has one link end that 
            * is nodata in the DEM, or
            */
            is_exterior = false;
            x = link_key_points[i].end_point1.x;
            y = link_key_points[i].end_point1.y;
            let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
            
            j = 0;
            for n in 0..ret.len() {
                id = *ret[n].1;
                if id != i && !is_beyond_edge[id] {
                    j += 1;
                    if link_min_elev[id] < z { z = link_min_elev[id]; }
                }
            }

            if j == 0 {
                is_exterior = true;
            }

            x = link_key_points[i].end_point2.x;
            y = link_key_points[i].end_point2.y;
            let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
            
            j = 0;
            for n in 0..ret.len() {
                id = *ret[n].1;
                if id != i && !is_beyond_edge[id] {
                    j += 1;
                    if link_min_elev[id] < z { z = link_min_elev[id]; }
                }
            }

            if j == 0 {
                is_exterior = true;
            }

            if is_exterior || crosses_nodata[i] {
                is_exterior_link[i] = true;
                if link_min_elev[i] <= z || crosses_nodata[i] {
                    queue.push(StreamLink{ index: i, min: link_min_elev[i] + max_ridge_cutting_height });
                }
            }
        }

        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (i + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Finding starting points: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // perform the priority-flood operation
    // let mut num_snapped_outlets = 0;
    let mut sl: StreamLink;
    let mut have_visited = vec![false; total_num_parts];
    let mut have_entered_queue = vec![false; total_num_parts];
    let mut num_infowing_links = vec![0; total_num_parts];
    let mut dist_to_outlet = vec![0f64; total_num_parts];
    let mut trib_num = vec![0; total_num_parts];
    let mut link: isize;
    let mut current_max_outlet_num = 0;
    let mut dsn: isize;
    let mut is_confluence: bool;
    let mut num_links: isize;
    // let mut total_num_links: isize;
    let mut num_links_visited = 0;
    let mut end_point: Point2D;
    
    while !queue.is_empty() {
        sl = queue.pop().expect("Error during pop operation.");
        link = sl.index as isize;
        if !have_visited[link as usize] {
            have_visited[link as usize] = true;
            have_entered_queue[link as usize] = true;

            dist_to_outlet[link as usize] += link_lengths[link as usize];

            // What is the downstream link?
            dsn = downstream_link[link as usize];

            // What outlet number does the DSN belong to?
            if dsn >= 0 {
                outlet_num = outlet_nums[dsn as usize];				
            } else {
                // which end point is the downstream outlet node?
                end_point = link_key_points[link as usize].end_point1;

                x = end_point.x;
                y = end_point.y;

                let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
                num_links = 0;
                for n in 0..ret.len() {
                    id = *ret[n].1;
                    if !is_beyond_edge[id] && !have_visited[id] && !is_outlet_link[id] {
                        num_links += 1;
                    }
                }

                if num_links > 0 {
                    // end point 2 is the downstream node
                    x = link_key_points[link as usize].end_point2.x;
                    y = link_key_points[link as usize].end_point2.y;
                } else {
                    // how many linking nodes are at end point 2?
                    end_point = link_key_points[link as usize].end_point2;

                    x = end_point.x;
                    y = end_point.y;
                    let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
                    num_links = 0;
                    for n in 0..ret.len() {
                        id = *ret[n].1;
                        if !is_beyond_edge[id] && !have_visited[id] && !is_outlet_link[id] {
                            num_links += 1;
                        }
                    }

                    if num_links > 0 {
                        // end point 1 is the downstream node
                        x = link_key_points[link as usize].end_point1.x;
                        y = link_key_points[link as usize].end_point1.y;
                    } else { // it's a single channel stream, which end is lower?
                        if link_key_points[link as usize].z1 < link_key_points[link as usize].z2 || 
                        (link_key_points[link as usize].z1 == nodata && link_key_points[link as usize].z2 != nodata) {
                            x = link_key_points[link as usize].end_point1.x;
                            y = link_key_points[link as usize].end_point1.y;
                        } else {
                            x = link_key_points[link as usize].end_point2.x;
                            y = link_key_points[link as usize].end_point2.y;
                        }
                    }
                }

                if !crosses_nodata[link as usize] {
                    /* This is a dangling stream. First let's make 
                    *  sure that there isn't a link end node from 
                    *  a previously discovered outlet nearby that 
                    *  we could connect to this outlet point
                    */ 
                    let ret = points_tree.nearest(&[x, y], 3, &squared_euclidean).unwrap();
                    let mut snapped_neighbour = -1isize;
                    for n in 0..ret.len() {
                        id = *ret[n].1;
                        if !is_beyond_edge[id] && have_visited[id] && is_exterior_link[id] && id as isize != link {
                            // Check to see if the distance is less than the specified
                            // snap distance.
                            if ret[n].0 < snap_distance {
                                snapped_neighbour = id as isize;
                                break;
                            }
                        }
                    }
                    
                    if snapped_neighbour >= 0 {
                        // we found a neighbour to snap to
                        dsn = snapped_neighbour;
                        outlet_num = outlet_nums[dsn as usize];
                        outlet_nums[link as usize] = outlet_num;
                        downstream_link[link as usize] = dsn;
                        num_infowing_links[dsn as usize] += 1;
                        num_downstream_nodes[link as usize] = num_downstream_nodes[dsn as usize] + 1;
                        dist_to_outlet[link as usize] += dist_to_outlet[dsn as usize];
                        // num_snapped_outlets += 1;
                    } else {
                        // it is a true outlet

                        // There isn't a DSN and we need a new outlet number
                        current_max_outlet_num += 1;
                        outlet_num = current_max_outlet_num;
                        outlet_nums[link as usize] = outlet_num;
                        is_outlet_link[link as usize] = true;
                        
                        // pointOfInterest = new whitebox.geospatialfiles.shapefile.Point(x, y);                  
                        // rowData = new Object[1];
                        // rowData[0] = new Double(outletNum);
                        // outputOutlets.addRecord(pointOfInterest, rowData);
                        output_outlets.add_point_record(x, y);
                        output_outlets.attributes.add_record(vec![FieldData::Int(outlet_num as i32)], false);
                    }
                } else {
                    // There isn't a DSN and we need a new outlet number
                    current_max_outlet_num += 1;
                    outlet_num = current_max_outlet_num;
                    outlet_nums[link as usize] = outlet_num;
                    is_outlet_link[link as usize] = true;
                
                    // point_of_interest = new whitebox.geospatialfiles.shapefile.Point(x, y);                  
                    // rowData = new Object[1];
                    // rowData[0] = new Double(outletNum);
                    // outputOutlets.addRecord(pointOfInterest, rowData);
                    output_outlets.add_point_record(x, y);
                    output_outlets.attributes.add_record(vec![FieldData::Int(outlet_num as i32)], false);
                }
            }

            for pt in link_key_points[link as usize].get_all_points() { // (XYPoint pt : linkKeyPoints[link].getAllPoints()) {
                x = pt.x;
                y = pt.y;
                let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
                num_links = 0;
                for n in 0..ret.len() {
                    id = *ret[n].1;
                    if !is_beyond_edge[id] && !have_entered_queue[id] {
                        num_links += 1;
                    }
                }

                is_confluence = if num_links > 1 { true } else { false }; 
                if is_confluence {
                    // pointOfInterest = new whitebox.geospatialfiles.shapefile.Point(x, y);                  
                    // rowData = new Object[1];
                    // rowData[0] = new Double(1);
                    // outputConfluences.addRecord(pointOfInterest, rowData);
                    output_confluences.add_point_record(x, y);
                    output_confluences.attributes.add_record(vec![FieldData::Int(1i32)], false);
                }
                for n in 0..ret.len() {
                    id = *ret[n].1;
                    if !is_beyond_edge[id] && !have_entered_queue[id] {
                        // add the link to the queue
                        z = link_min_elev[id];
                        queue.push(StreamLink{ index: id, min: z });

                        have_entered_queue[id] = true;

                        // update the DSN for this link
                        downstream_link[id] = link;
                        if is_confluence {
                            num_downstream_nodes[id] = num_downstream_nodes[link as usize] + 1;
                        } else {
                            num_downstream_nodes[id] = num_downstream_nodes[link as usize];
                        }

                        dist_to_outlet[id] += dist_to_outlet[link as usize];
                        
                        outlet_nums[id] = outlet_num;

                        num_infowing_links[link as usize] += 1;
                    }
                }
            }

            num_links_visited += 1;
            if configurations.verbose_mode {
                progress =
                    (100.0_f64 * (num_links_visited + 1) as f64 / total_num_parts as f64) as usize;
                if progress != old_progress {
                    println!("Priority-Flood Operation: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    // calculate the link mag variables
    let mut strahler_order = vec![0usize; total_num_parts];
    let mut shreve_order = vec![0usize; total_num_parts];
    let mut max_upstream_length = vec![0f64; total_num_parts];
    let mut stack = vec![];
    let mut found_downstream_end: bool;
    for i in 0..total_num_parts {
        if num_infowing_links[i] == 0 && !is_beyond_edge[i] {
            stack.push(i);
            strahler_order[i] = 1;
            shreve_order[i] = 1;

            // this is a headwater, find which end is the channel head
            found_downstream_end = false;
            dsn = downstream_link[i];
            end_point = link_key_points[i].end_point1;
            x = end_point.x;
            y = end_point.y;
            let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
            for j in 0..ret.len() {
                id = *ret[j].1;
                if id as isize == dsn {
                    found_downstream_end = true;
                }
            }
            
            if !found_downstream_end {
                // pointOfInterest = new whitebox.geospatialfiles.shapefile.Point(x, y);                  
                // rowData = new Object[1];
                // rowData[0] = new Double(1);
                // outputChannelHeads.addRecord(pointOfInterest, rowData);
                output_channel_heads.add_point_record(x, y);
                output_channel_heads.attributes.add_record(vec![FieldData::Int(1i32)], false);
            } else {
                end_point = link_key_points[i].end_point2;
                x = end_point.x;
                y = end_point.y;
                // pointOfInterest = new whitebox.geospatialfiles.shapefile.Point(x, y);                  
                // rowData = new Object[1];
                // rowData[0] = new Double(1);
                // outputChannelHeads.addRecord(pointOfInterest, rowData);
                output_channel_heads.add_point_record(x, y);
                output_channel_heads.attributes.add_record(vec![FieldData::Int(1i32)], false);
            }
        }
    }

    let mut count = 0;
    while !stack.is_empty() {
        let i = stack.pop().expect("Error during pop operation.");
        link_mag[i] += link_lengths[i];
        max_upstream_length[i] += link_lengths[i];
        dsn = downstream_link[i];
        if dsn >= 0isize {
            // pass this downstream
            link_mag[dsn as usize] += link_mag[i];
            num_infowing_links[dsn as usize] -= 1;
            if num_infowing_links[dsn as usize] == 0 {
                stack.push(dsn as usize);
            }

            if strahler_order[dsn as usize] == strahler_order[i] {
                strahler_order[dsn as usize] += 1;
            } else if strahler_order[i] > strahler_order[dsn as usize] {
                strahler_order[dsn as usize] = strahler_order[i];
            }

            if max_upstream_length[i] > max_upstream_length[dsn as usize] {
                max_upstream_length[dsn as usize] = max_upstream_length[i];
            }

            shreve_order[dsn as usize] += shreve_order[i];
        }
        count += 1;
        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (count + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Accumulation operations: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // perform the outlet-to-head ops like finding the main stem
    // and assign tributary numbers
    let mut is_main_stem = vec![false; total_num_parts];
    let mut horton_order = vec![0usize; total_num_parts];
    let mut hack_order = vec![0usize; total_num_parts];
    stack = vec![];
    let mut current_trib_num = 0;

    for i in 0..total_num_parts {
        if is_outlet_link[i] {
            is_main_stem[i] = true;
            horton_order[i] = strahler_order[i];
            hack_order[i] = 1;
            stack.push(i);
            current_trib_num += 1;
            trib_num[i] = current_trib_num;
        }

        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (i + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Assigning tributary IDs: {}%", progress);
                old_progress = progress;
            }
        }
    }

    
    let mut neighbour_list = vec![];
    count = 0;
    while !stack.is_empty() {
        let i = stack.pop().expect("Error during pop operation.");
        neighbour_list.clear();
        let mut max_tucl = 0f64;
        let mut max_tucl_link = -1isize;
        for pt in link_key_points[i].get_all_points() {
            x = pt.x;
            y = pt.y;
            let ret = points_tree.within(&[x, y], precision, &squared_euclidean).unwrap();
            // num_links = 0;
            for j in 0..ret.len() {
                id = *ret[j].1;
                if downstream_link[id] == i as isize {
                    neighbour_list.push(id);
                    // if link_mag[id] > max_tucl {
                    if max_upstream_length[id] > max_tucl {
                        max_tucl = max_upstream_length[id]; // link_mag[id];
                        max_tucl_link = id as isize;
                    }
                }
            }
        }
        if max_tucl_link >= 0 {
            //isMainStem[maxTUCLlink] = true;
            for q in 0..neighbour_list.len() {
                let n = neighbour_list[q];
                // add it to the stack
                stack.push(n);
                if n as isize != max_tucl_link {
                    current_trib_num += 1;
                    trib_num[n] = current_trib_num;
                    horton_order[n] = strahler_order[n];
                    hack_order[n] = hack_order[i] + 1;
                } else {
                    trib_num[n] = trib_num[i];
                    horton_order[n] = horton_order[i];
                    hack_order[n] = hack_order[i];
                    if is_main_stem[downstream_link[n] as usize] {
                        is_main_stem[n] = true;
                    }
                }
            }
        }
                
        count += 1;
        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (count + 1) as f64 / total_num_parts as f64) as usize;
            if progress != old_progress {
                println!("Assigning tributary IDs: {}%", progress);
                old_progress = progress;
            }
        }
    }

    // Output the data into the attribute table.
    feature_num = 0;
    count = 0;
    let mut att_data: Vec<FieldData>;
    let mut fid = 1;
    for rec_num in 0..input.num_records {
        let record = input.get_record(rec_num);  
        // num_points = record.points.len();
        for part in 0..record.num_parts as usize {
            part_start = record.parts[part] as usize;
            part_end = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };
            if !is_beyond_edge[feature_num as usize] {
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
                att_data.push(FieldData::Real(link_min_elev[feature_num]));
                att_data.push(FieldData::Real(link_max_elev[feature_num]));
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
                att_data.push(FieldData::Int(downstream_link[feature_num] as i32));
                if is_main_stem[feature_num] {
                    att_data.push(FieldData::Int(1));
                } else {
                    att_data.push(FieldData::Int(0));
                }
                att_data.push(FieldData::Int(trib_num[feature_num] as i32));

                // output.attributes.add_record(
                //     vec![FieldData::Int(fid as i32)],
                //     false,
                // );

                output.attributes.add_record(att_data.clone(), false);
                fid += 1;
            }
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


struct StreamLinkKeyPoints {
    pub end_point1: Point2D,
    pub end_point2: Point2D,
    pub z1: f64,
    pub z2: f64,
    pub intermediate_points: Vec<Point2D>,
}

impl StreamLinkKeyPoints {
    fn new(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> StreamLinkKeyPoints {
        StreamLinkKeyPoints {
            end_point1: Point2D::new(x1, y1),
            z1: z1,
            end_point2: Point2D::new(x2, y2),
            z2: z2,
            intermediate_points: vec![],
        }
    }

    fn add_intermediate_point(&mut self, x: f64, y: f64) {
        self.intermediate_points.push(Point2D::new(x, y));
    }

    fn get_all_points(&self) -> Vec<Point2D> {
        let mut points = vec![];
        points.push(self.end_point1);
        points.push(self.end_point2);
        for p in &self.intermediate_points {
            points.push(p.clone());
        }

        points
    }
}

#[derive(PartialEq, Debug)]
struct StreamLink {
    index: usize,
    min: f64,
}

impl Eq for StreamLink {}

impl PartialOrd for StreamLink {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.min.partial_cmp(&self.min)
    }
}

impl Ord for StreamLink {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}