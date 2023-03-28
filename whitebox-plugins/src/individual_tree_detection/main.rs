/* 
Authors:  Dr. John Lindsay
Created: 05/03/2023
Last Modified: 05/03/2023
License: MIT
*/

use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_lidar::*;
use whitebox_vector::*;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_common::structures::Point3D;
use kd_tree::{KdPoint, KdTree};
use num_cpus;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to identify points in a LiDAR point cloud that are associated with the tops of individual trees. The
/// tool takes a LiDAR point cloud as an input (`input_lidar`) and it is best if the input file has been normalized using the
/// `NormalizeLidar` or `LidarTophatTransform` tools, such that points record height above the ground surface. Note that the `input` 
/// parameter is optional and if left unspecified the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) files 
/// contained within the current working directory. This 'batch mode' operation is common among many of the LiDAR processing 
/// tools. Output vectors are saved to disc automatically for each processed LiDAR file when operating in batch mode.
/// 
/// The tool will evaluate the points within a local neighbourhood around each point in the input point cloud and determine
/// if it is the highest point within the neighbourhood. If a point is the highest local point, it will be entered into the
/// output vector file (`output`). The neighbourhood size can vary, with higher canopy positions generally associated with larger
/// neighbourhoods. The user specifies the `min_search_radius` and `min_height` parameters, which default to 1 m and 0 m 
/// respectively. If the `min_height` parameter is greater than zero, all points that are less than this value above the 
/// ground (assuming the input point cloud measures this height parameter) are ignored, which can be a useful mechanism
/// for removing shorter trees and other vegetation from the analysis. If the user specifies the `max_search_radius` and
/// `max_height` parameters, the search radius will be determined by linearly interpolation based on point height and the
/// min/max search radius and height parameter values. Points that are above the `max_height` parameter will be processed
/// with search neighbourhoods sized `max_search_radius`. If the max radius and height parameters are unspecified, they
/// are set to the same values as the minimum radius and height parameters, i.e., the neighbourhood size does not increase
/// with canopy height.
/// 
/// If the point cloud contains point classifications, it may be useful to exclude all non-vegetation points. To do this
/// simply set the `only_use_veg` parameter to True. This parameter should only be set to True when you know that the
/// input file contains point classifications, otherwise the tool may generate an empty output vector file.
/// 
/// ![](../../doc_img/IndividualTreeDetection.png)
/// 
/// # See Also
/// `NormalizeLidar`, `LidarTophatTransform`
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

    let exe_name = &format!("IndividualTreeDetection{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    individual_tree_detection Help

    This tool is used to identify points in a LiDAR point cloud that are associated with the tops of individual trees.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input          Name of the input LiDAR file.
    -o, --output         Name of the output vector points file.
    --min_search_radius  Minimum search radius (m).
    --min_height         Minimum height (m).
    --max_search_radius  Maximum search radius (m).
    --max_height         Maximum height (m).
    --only_use_veg       Only use veg. class points?
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=points.laz -o=tree_tops.shp --min_search_radius=1.5 --min_height=2.0 --max_search_radius=8.0 --max_height=30.0 --only_use_veg
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "IndividualTreeDetection v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("IndividualTreeDetection") // This should be camel case and is a reference to the tool name.
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

    let mut num_procs = num_cpus::get() as isize;
    if configurations.max_procs > 0 && configurations.max_procs < num_procs {
        num_procs = configurations.max_procs;
    }

    // read the arguments
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut min_search_radius = 1f64;
    let mut min_height = 0f64;
    let mut max_search_radius = -1f64;
    let mut max_height = -1f64;
    let mut only_use_veg = false;

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
        } else if flag_val == "-min_search_radius" {
            min_search_radius = if keyval {
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
        } else if flag_val == "-min_height" {
            min_height = if keyval {
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
        } else if flag_val == "-max_search_radius" {
            max_search_radius = if keyval {
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
        } else if flag_val == "-max_height" {
            max_height = if keyval {
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
        } else if flag_val == "-only_use_veg" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                only_use_veg = true;
            }
        }
    }

    let start = Instant::now();

    if configurations.verbose_mode {
        let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
        // 28 = length of the 'Powered by' by statement.
        println!("{}", "*".repeat(welcome_len));
        println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
        println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
        println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
        println!("{}", "*".repeat(welcome_len));
    }

    if max_search_radius < 0f64 { max_search_radius = min_search_radius; }
    if max_height < 0f64 { max_height = min_height; }

    let radius_range = max_search_radius - min_search_radius;
    let height_range = max_height - min_height;
    
    if min_search_radius <= 0f64 || max_search_radius <= 0f64 {
        return Err(Error::new(ErrorKind::InvalidInput, "The search radius parameters must be larger than zero."));
    }

    if input_file.trim().is_empty() {
        if working_directory.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "This tool must be run by specifying either an individual input file or a working directory."));
        }
        let mut inputs = vec![];
        let mut outputs = vec![];
        if std::path::Path::new(&working_directory).is_dir() {
            for entry in fs::read_dir(working_directory.clone())? {
                let s = entry?
                    .path()
                    .into_os_string()
                    .to_str()
                    .expect("Error reading path string")
                    .to_string();
                if s.to_lowercase().ends_with(".las") {
                    inputs.push(s);
                    outputs.push(
                        inputs[inputs.len() - 1]
                            .replace(".las", ".shp")
                            .replace(".LAS", ".shp"),
                    )
                } else if s.to_lowercase().ends_with(".laz") {
                    inputs.push(s);
                    outputs.push(
                        inputs[inputs.len() - 1]
                            .replace(".laz", ".shp")
                            .replace(".LAZ", ".shp"),
                    )
                } else if s.to_lowercase().ends_with(".zlidar") {
                    inputs.push(s);
                    outputs.push(
                        inputs[inputs.len() - 1]
                            .replace(".zlidar", ".shp")
                            .replace(".ZLIDAR", ".shp"),
                    )
                } else if s.to_lowercase().ends_with(".zip") {
                    inputs.push(s);
                    outputs.push(
                        inputs[inputs.len() - 1]
                            .replace(".zip", ".shp")
                            .replace(".ZIP", ".shp"),
                    )
                }
            }
        } else {
            return Err(Error::new(ErrorKind::InvalidInput, format!("The input directory ({}) is incorrect.", working_directory)));
        }

        let num_tiles = inputs.len();
        if num_tiles == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,"No input files could be located."));
        }

        let inputs = Arc::new(inputs);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs as usize {
            let inputs = inputs.clone();
            let outputs = outputs.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                for tile in (0..num_tiles).filter(|t| t % num_procs as usize == tid) {
                    let mut input = LasFile::new(&inputs[tile], "r").expect("Error reading input file");
                    let n_points = input.header.number_of_points as usize;
                    
                    let mut output = Shapefile::new(&outputs[tile], ShapeType::Point).expect("Error creating vector file.");
                    output.projection = input.get_wkt();

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        7u8,
                        0u8,
                    ));

                    output.attributes.add_field(&AttributeField::new(
                        "Z",
                        FieldDataType::Real,
                        12u8,
                        5u8,
                    ));

                    if configurations.verbose_mode && num_tiles == 1 {
                        println!("Building kd-tree...");
                    }
                    let mut points: Vec<TreeItem> = Vec::with_capacity(n_points);
                    let mut p: Point3D;
                    let mut pd: PointData;
                    for i in 0..n_points {
                        pd = input[i];
                        if !pd.withheld() && !pd.is_classified_noise() && (!only_use_veg || pd.is_classified_vegetation()) {
                            p = input.get_transformed_coords(i);
                            points.push( TreeItem { point: [p.x, p.y], id: i } );
                        }
                    }

                    if points.len() == 0 {
                        if only_use_veg {
                            println!("No points were added to the kd-tree. It is possible that the points are unclassified; use only_use_veg = False instead.")
                        } else {
                            println!("No points were added to the kd-tree.");
                        }
                    }

                    // build the tree
                    let kdtree: KdTree<TreeItem> = KdTree::par_build_by_ordered_float(points);

                    let mut p2: Point3D;
                    let mut radius: f64;
                    let mut is_highest_pt: bool;
                    let mut index_n: usize;
                    for point_num in 0..n_points {
                        pd = input[point_num];
                        if !pd.withheld() && !pd.is_classified_noise() && (!only_use_veg || pd.is_classified_vegetation()) {
                            p = input.get_transformed_coords(point_num);
                            radius = if p.z < min_height {
                                min_search_radius
                            } else if p.z > max_height {
                                max_search_radius
                            } else {
                                min_search_radius + (p.z - min_height) / height_range * radius_range
                            };
                            let found = kdtree.within_radius(&[p.x, p.y], radius);
                            is_highest_pt = true;
                            for i in 0..found.len() {
                                index_n = found[i].id;
                                p2 = input.get_transformed_coords(index_n);
                                if p2.z > p.z {
                                    is_highest_pt = false;
                                    break;
                                }
                            }

                            if is_highest_pt {
                                output.add_point_record(p.x, p.y);
                                output.attributes.add_record(
                                    vec![
                                        FieldData::Int(point_num as i32 + 1i32),
                                        FieldData::Real(p.z)
                                    ],
                                    false,
                                );
                            }
                        }
                    }

                    output.write().expect("Error writing vector file.");

                    tx.send(tile).unwrap();
                }
            });
        }
        
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..num_tiles {
            let tile_completed: usize = rx.recv().unwrap();
            if configurations.verbose_mode {
                if tile <= 99 && num_tiles > 1 {
                    println!(
                        "Finished {} ({} of {})",
                        inputs[tile_completed]
                            .replace("\"", "")
                            .replace(&working_directory, "")
                            .replace(".las", ""),
                        tile + 1,
                        inputs.len()
                    );
                }

                if tile == 99 {
                    println!("...");
                }
                progress = (100.0_f64 * tile as f64 / (inputs.len() - 1) as f64) as i32;
                if progress != old_progress && num_tiles > 1 {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if configurations.verbose_mode {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!("Elapsed Time: {}", elapsed_time);
        }
    } else {
        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut input = LasFile::new(&input_file, "r")?;
        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut output = Shapefile::new(&output_file, ShapeType::Point).expect("Error creating vector file.");
        output.projection = input.get_wkt();

        // add the attributes
        output.attributes.add_field(&AttributeField::new(
            "FID",
            FieldDataType::Int,
            7u8,
            0u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "Z",
            FieldDataType::Real,
            12u8,
            5u8,
        ));

        if configurations.verbose_mode {
            println!("Reading lidar points...");
        }
        let mut points: Vec<TreeItem> = Vec::with_capacity(n_points);
        let mut p: Point3D;
        let mut pd: PointData;
        for i in 0..n_points {
            pd = input[i];
            if !pd.withheld() && !pd.is_classified_noise() && (!only_use_veg || pd.is_classified_vegetation()) {
                p = input.get_transformed_coords(i);
                points.push( TreeItem { point: [p.x, p.y], id: i } );
            }
        }

        if points.len() == 0 {
            if only_use_veg {
                return Err(Error::new(ErrorKind::InvalidInput, "No points were added to the kd-tree. It is possible that the points are unclassified; use only_use_veg = False instead.".to_string()));
            } else {
                return Err(Error::new(ErrorKind::InvalidInput, "No points were added to the kd-tree.".to_string()));
            }
        }

        // build the tree
        if configurations.verbose_mode {
            println!("Building kd-tree...");
        }
        let kdtree: KdTree<TreeItem> = KdTree::par_build_by_ordered_float(points);

        if configurations.verbose_mode {
            println!("Locating tree tops...");
        }
        let input = Arc::new(input);
        let kdtree = Arc::new(kdtree);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs as usize {
            let input = input.clone();
            let kdtree = kdtree.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut p: Point3D;
                let mut pd: PointData;
                let mut p2: Point3D;
                let mut radius: f64;
                let mut is_highest_pt: bool;
                let mut index_n: usize;
                for point_num in (0..n_points).filter(|t| t % num_procs as usize == tid) {
                    pd = input[point_num];
                    if !pd.withheld() && !pd.is_classified_noise() && (!only_use_veg || pd.is_classified_vegetation()) {
                        p = input.get_transformed_coords(point_num);
                        if p.z >= min_height {
                            radius = if p.z < min_height {
                                min_search_radius
                            } else if p.z > max_height {
                                max_search_radius
                            } else {
                                min_search_radius + (p.z - min_height) / height_range * radius_range
                            };
                            let found = kdtree.within_radius(&[p.x, p.y], radius);
                            is_highest_pt = true;
                            for i in 0..found.len() {
                                index_n = found[i].id;
                                p2 = input.get_transformed_coords(index_n);
                                if p2.z > p.z {
                                    is_highest_pt = false;
                                    break;
                                }
                            }

                            if is_highest_pt {
                                tx.send((point_num, true)).unwrap();
                            } else {
                                tx.send((point_num, false)).unwrap();
                            }
                        } else {
                            tx.send((point_num, false)).unwrap();
                        }
                    } else {
                        tx.send((point_num, false)).unwrap();
                    }
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for point_num in 0..n_points {
            let data = rx.recv().unwrap();
            if data.1 {
                p = input.get_transformed_coords(data.0);
                output.add_point_record(p.x, p.y);
                output.attributes.add_record(
                    vec![
                        FieldData::Int(data.0 as i32 + 1i32),
                        FieldData::Real(p.z)
                    ],
                    false,
                );
            }
            if configurations.verbose_mode {
                progress = (100f64 * point_num as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if configurations.verbose_mode {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!("Elapsed Time: {}", elapsed_time);
        }

        output.write().expect("Error writing vector file.");
    }
        
    Ok(())
}

struct TreeItem {
    point: [f64; 2],
    id: usize,
}

impl KdPoint for TreeItem {
    type Scalar = f64;
    type Dim = typenum::U2; // 2 dimensional tree.
    fn at(&self, k: usize) -> f64 { self.point[k] }
}