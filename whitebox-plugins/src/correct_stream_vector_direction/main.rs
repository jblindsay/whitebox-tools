/* 
Authors: Prof. John Lindsay
Created: 15/08/2023 (oringinally in Whitebox Toolset Extension)
Last Modified: 15/08/2023
License: MIT
*/

use rstar::primitives::GeomWithData;
use rstar::RTree;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::ops::Index;
use std::path;
use std::str;
use std::time::Instant;
use std::collections::VecDeque;
use whitebox_common::structures::Point2D;
use whitebox_common::utils::{
    get_formatted_elapsed_time, 
    wrapped_print
};
use whitebox_vector::{
    // AttributeField, 
    // FieldData, 
    // FieldDataType, 
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

    let exe_name = &format!("correct_stream_vector_direction{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    This tool resolves topological errors and inconsistencies associated with digitized vector streams.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    --routes       Name of the input routes vector file.
    -o, --output   Name of the output HTML file.
    --length       Maximum segment length (m).
    --dist         Search distance, in grid cells, used in visibility analysis.
    
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
        "correct_stream_vector_direction v{} by Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("CorrectStreamVectorDirection") // This should be camel case and is a reference to the tool name.
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
    let mut outlet_file = String::new();
    let mut output_file: String = String::new();
    let mut snap_dist = 1.0;
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
        } else if flag_val == "-outlet" {
            outlet_file = if keyval {
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
    let mut old_progress: usize = 1;

    let start = Instant::now();

    if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !outlet_file.contains(path::MAIN_SEPARATOR) && !outlet_file.contains("/") {
        outlet_file = format!("{}{}", working_directory, outlet_file);
    }

    if output_file.is_empty() {
        output_file = input_file
            .clone()
            .replace(".shp", "_corrected.shp")
            .replace(".SHP", "_corrected.shp");
    }
    if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }
    

    if snap_dist <= 0f64 {
        if configurations.verbose_mode {
            wrapped_print("Error: The snap distance must be greater than 0.0.", 50);
        }
    }


    let input = Shapefile::read(&input_file).expect("Error reading file"); //?;
    
    // Make sure the input vector file is of polyline type
    if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
        // return Err(Error::new(
        //     ErrorKind::InvalidInput,
        //     "The vector data must be of PolyLine base shape type.",
        // ));
        panic!("The vector stream data must be of PolyLine base shape type.");
    }

    let outlets = Shapefile::read(&outlet_file).expect("Error reading file"); //?;
    
    // Make sure the input vector file is of polyline type
    if outlets.header.shape_type.base_shape_type() != ShapeType::Point {
        panic!("The vector outlets data must be of POINT base shape type.");
    }

    let mut progress: usize;

    // Read each line segment into an rtree.
    type Location = GeomWithData<[f64; 2], (usize, bool)>;
    let mut end_nodes = vec![];
    let (mut part_start, mut part_end): (usize, usize);
    let mut fid = 0usize; // fid is unique to each part in the vector
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
                    record_num
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

    let num_polylines = polylines.len(); // will be updated after the joins.

    let snap_dist_sq = snap_dist * snap_dist;
    let endnode_tree = RTree::bulk_load(end_nodes);
    let precision = EPSILON * 10f64;
    let mut outlet_pt: Point2D;
    let mut p1: Point2D;
    let mut visited = vec![false; num_polylines];
    let mut reverse = vec![false; num_polylines];
    for record_num in 0..outlets.num_records {
        let record = outlets.get_record(record_num);  
        if record.shape_type != ShapeType::Null {
            for p in 0..record.points.len() {
                outlet_pt = record.points[p];
                let ret = endnode_tree.locate_within_distance([outlet_pt.x, outlet_pt.y], snap_dist_sq);
                
                for pt in ret {
                    let (fid, is_start) = pt.data;
                    if !visited[fid] {
                        visited[fid] = true;

                        // now find all connected line segments
                        let mut queue: VecDeque<(usize, bool)> = VecDeque::with_capacity(num_polylines);
                        queue.push_back((fid, is_start));
                        while !queue.is_empty() {
                            let (fid2, is_start2) = queue.pop_front().unwrap();

                            // Find the point associated with the other end of this polyline
                            p1 = if !is_start2 {
                                polylines[fid2].get_first_node()
                            } else {
                                // To get here means that you first encountered the beginning of the polyline, which 
                                // shouldn't happen if it is correctly directed, since we are doing a bottom-up
                                // scan of the network. Therefore, reverse the line in the output.
                                reverse[fid2] = true; 
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
                                }
                            }
                        }
                    }
                }
            }
        } 

        if configurations.verbose_mode {
            progress = (100.0_f64 * (fid + 1) as f64 / num_polylines as f64) as usize;
            if progress != old_progress {
                println!("Looking for reverse-oriented arcs: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let mut num_reversed = 0;
    for fid in 0..polylines.len() {
        if reverse[fid] {
            let mut line = polylines[fid].vertices.clone();
            line.reverse();
            polylines[fid].vertices = line;
            num_reversed += 1;
        }
    }
    println!("num. reversed arcs: {num_reversed}");

    // create output file
    let mut output = Shapefile::initialize_using_file(&output_file.replace(".shp", "_reversed_arcs.shp"), &input, ShapeType::PolyLine, true).expect("Error creating output file");

    let mut sfg: ShapefileGeometry;
    for fid in 0..polylines.len() {
        if reverse[fid] {
            sfg = ShapefileGeometry::new(ShapeType::PolyLine); 
            sfg.add_part(&polylines[fid].vertices);
            output.add_record(sfg);

            let record_num = polylines[fid].id;
            let att_data = input.attributes.get_record(record_num);
            output.attributes.add_record(att_data.clone(), false);
        }
    }

    output.write().expect("Error writing file.");


    // create output file
    let mut output = Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, true).expect("Error creating output file"); //?;

    // add the attributes
    // let in_atts = input.attributes.get_fields();
    
    let mut sfg: ShapefileGeometry;
    for poly_id in 0..polylines.len() {
        sfg = ShapefileGeometry::new(ShapeType::PolyLine); 
        sfg.add_part(&polylines[poly_id].vertices);
        output.add_record(sfg);

        let record_num = polylines[poly_id].id;
        let att_data = input.attributes.get_record(record_num);
        output.attributes.add_record(att_data.clone(), false);

        if configurations.verbose_mode {
            progress =  (100.0_f64 * (poly_id + 1) as f64 / polylines.len() as f64) as usize;
            if progress != old_progress {
                println!("Looking for dangling arcs: {}%", progress);
                old_progress = progress;
            }
        }
    }
    

    if configurations.verbose_mode {
        println!("Saving data...")
    };

    output.write().expect("Error writing file.");


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
        }
    }

    fn get_first_node(&self) -> Point2D {
        self[0]
    }

    fn get_last_node(&self) -> Point2D {
        self[self.vertices.len() - 1]
    }
}