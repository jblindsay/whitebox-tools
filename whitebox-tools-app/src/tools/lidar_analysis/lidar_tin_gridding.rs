/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/09/2018
Last Modified: 25/05/2020
License: MIT
*/

use self::na::Vector3;
use whitebox_common::algorithms::{point_in_poly, triangulate};
use whitebox_lidar::*;
use crate::na;
use whitebox_raster::*;
use whitebox_common::spatial_ref_system::esri_wkt_from_epsg;
use whitebox_common::structures::{BoundingBox, Point2D};
use crate::tools::*;
use num_cpus;
use std::io::{Error, ErrorKind};
use std::sync::mpsc;
// use std::sync::{Arc, Mutex};
use std::sync::Arc;
use std::{env, f64, fs, path, thread};

/// This tool creates a raster grid based on a Delaunay triangular irregular network (TIN) fitted to LiDAR points.
/// The output grid can be based on any of the stored LiDAR point parameters (`--parameter`), including elevation
/// (in which case the output grid is a digital elevation model, DEM), intensity, class, return number, number of
/// returns, scan angle, RGB (colour) values, and user data values. Similarly, the user may specify which point
/// return values (`--returns`) to include in the interpolation, including all points, last returns (including single
/// return points), and first returns (including single return points).
///
/// The user must specify the grid resolution of the output raster (`--resolution`), and optionally, the name of the
/// input LiDAR file (`--input`) and output raster (`--output`). Note that if an input LiDAR file (`--input`) is not
/// specified by the user, the tool will search for all valid LiDAR (*.las, *.laz, *.zlidar) contained within the current
/// working directory. This feature can be very useful when you need to interpolate a DEM for a large number of LiDAR
/// files. Not only does this batch processing mode enable the tool to run in a more optimized parallel manner, but it
/// will also allow the tool to include a small buffer of points extending into adjacent tiles when interpolating an
/// individual file. This can significantly reduce edge-effects when the output tiles are later mosaicked together.
/// When run in this batch mode, the output file (`--output`) also need not be specified; the tool will instead create
/// an output file with the same name as each input LiDAR file, but with the .tif extension. This can provide a very
/// efficient means for processing extremely large LiDAR data sets.
///
/// Users may excluded points from the interpolation based on point classification values, which follow the LAS
/// classification scheme. Excluded classes are specified using the `--exclude_cls` parameter. For example,
/// to exclude all vegetation and building classified points from the interpolation, use --exclude_cls='3,4,5,6'.
/// Users may also exclude points from the interpolation if they fall below or above the minimum (`--minz`) or
/// maximum (`--maxz`) thresholds respectively. This can be a useful means of excluding anomalously high or low
/// points. Note that points that are classified as low points (LAS class 7) or high noise (LAS class 18) are
/// automatically excluded from the interpolation operation.
///
/// Triangulation will generally completely fill the convex hull containing the input point data. This can sometimes
/// result in very long and narrow triangles at the edges of the data or connecting vertices on either side of void
/// areas. In LiDAR data, these void areas are often associated with larger waterbodies, and triangulation can result
/// in very unnatural interpolated patterns within these areas. To avoid this problem, the user may specify a the
/// maximum allowable triangle edge length (`max_triangle_edge_length`) and all grid cells within triangular facets
/// with edges larger than this threshold are simply assigned the NoData values in the output DSM. These NoData areas
/// can later be better dealt with using the `FillMissingData` tool after interpolation.
///
/// # See Also
/// `LidarIdwInterpolation`, `LidarNearestNeighbourGridding`, `LidarTINGridding`, `FilterLidarClasses`, `FillMissingData`
pub struct LidarTINGridding {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarTINGridding {
    pub fn new() -> LidarTINGridding {
        // public constructor
        let name = "LidarTINGridding".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates a raster grid based on a Delaunay triangular irregular network (TIN) fitted to LiDAR points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file (including extension).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file (including extension).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Interpolation Parameter".to_owned(), 
            flags: vec!["--parameter".to_owned()], 
            description: "Interpolation parameter; options are 'elevation' (default), 'intensity', 'class', 'return_number', 'number_of_returns', 'scan angle', 'rgb', 'user data'.".to_owned(),
            parameter_type: ParameterType::OptionList(
                vec![
                    "elevation".to_owned(), 
                    "intensity".to_owned(), 
                    "class".to_owned(), 
                    "return_number".to_owned(), 
                    "number_of_returns".to_owned(), 
                    "scan angle".to_owned(), 
                    "rgb".to_owned(),
                    "user data".to_owned()
                ]
            ),
            default_value: Some("elevation".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Point Returns Included".to_owned(),
            flags: vec!["--returns".to_owned()],
            description:
                "Point return types to include; options are 'all' (default), 'last', 'first'."
                    .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "all".to_owned(),
                "last".to_owned(),
                "first".to_owned(),
            ]),
            default_value: Some("all".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Exclusion Classes (0-18, based on LAS spec; e.g. 3,4,5,6,7)".to_owned(), 
            flags: vec!["--exclude_cls".to_owned()], 
            description: "Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: Some("7,18".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Minimum Elevation Value (optional)".to_owned(),
            flags: vec!["--minz".to_owned()],
            description: "Optional minimum elevation for inclusion in interpolation.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Elevation Value (optional)".to_owned(),
            flags: vec!["--maxz".to_owned()],
            description: "Optional maximum elevation for inclusion in interpolation.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Triangle Edge Length (optional)".to_owned(),
            flags: vec!["--max_triangle_edge_length".to_owned()],
            description: "Optional maximum triangle edge length; triangles larger than this size will not be gridded.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --returns=last --resolution=2.0 --exclude_cls='3,4,5,6,7,18' --max_triangle_edge_length=5.0", short_exe, name).replace("*", &sep);

        LidarTINGridding {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarTINGridding {
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
        let mut interp_parameter = "elevation".to_string();
        let mut interp_parameter_is_rgb = false;
        let mut return_type = "all".to_string();
        let mut grid_res: f64 = 1.0;
        let mut include_class_vals = vec![true; 256];
        let mut exclude_cls_str = String::new();
        let mut max_z = f64::INFINITY;
        let mut min_z = f64::NEG_INFINITY;
        let mut max_triangle_edge_length = f64::INFINITY;

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
            } else if flag_val == "-parameter" {
                interp_parameter = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };
                if interp_parameter == "rgb" {
                    interp_parameter_is_rgb = true;
                }
            } else if flag_val == "-returns" {
                return_type = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
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
            } else if flag_val == "-exclude_cls" {
                exclude_cls_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                let mut cmd = exclude_cls_str.split(",");
                let mut vec = cmd.collect::<Vec<&str>>();
                if vec.len() == 1 {
                    cmd = exclude_cls_str.split(";");
                    vec = cmd.collect::<Vec<&str>>();
                }
                for value in vec {
                    if !value.trim().is_empty() {
                        if value.contains("-") {
                            cmd = value.split("-");
                            vec = cmd.collect::<Vec<&str>>();
                            let c = vec[0].trim().parse::<usize>().unwrap();
                            let d = vec[1].trim().parse::<usize>().unwrap();
                            for e in c..=d {
                                include_class_vals[e] = false;
                            }
                        } else if value.contains("...") {
                            cmd = value.split("...");
                            vec = cmd.collect::<Vec<&str>>();
                            let c = vec[0].trim().parse::<usize>().unwrap();
                            let d = vec[1].trim().parse::<usize>().unwrap();
                            for e in c..=d {
                                include_class_vals[e] = false;
                            }
                        } else {
                            let c = value.trim().parse::<usize>().unwrap();
                            include_class_vals[c] = false;
                        }
                    }
                }
            } else if flag_val == "-minz" {
                min_z = if keyval {
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
            } else if flag_val == "-maxz" {
                max_z = if keyval {
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
            } else if flag_val == "-max_triangle_edge_length" {
                max_triangle_edge_length = if keyval {
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

                max_triangle_edge_length *= max_triangle_edge_length; // actually squared distance
            }
        }

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

        let start = Instant::now();

        let (all_returns, late_returns, early_returns): (bool, bool, bool);
        if return_type.contains("last") {
            all_returns = false;
            late_returns = true;
            early_returns = false;
        } else if return_type.contains("first") {
            all_returns = false;
            late_returns = false;
            early_returns = true;
        } else {
            // all
            all_returns = true;
            late_returns = false;
            early_returns = false;
        }

        // let search_radius = grid_res * 5f64; // this is an important parameter for minimizing edge effect when interpolating tiles.

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
                    if s.to_lowercase().ends_with(".las") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".las", ".tif")
                                .replace(".LAS", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".laz") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".laz", ".tif")
                                .replace(".LAZ", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zlidar", ".tif")
                                .replace(".ZLIDAR", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zip", ".tif")
                                .replace(".ZIP", ".tif"),
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
                    .replace(".las", ".tif")
                    .replace(".LAS", ".tif")
                    .replace(".laz", ".tif")
                    .replace(".LAZ", ".tif")
                    .replace(".zlidar", ".tif");
            }
            if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }
            outputs.push(output_file);
        }

        /*
        If multiple files are being interpolated, we will need to know their bounding boxes,
        in order to retrieve points from adjacent tiles. This is so that there are no edge
        effects.
        */
        let mut bounding_boxes = vec![];
        for in_file in &inputs {
            let header = LasHeader::read_las_header(&in_file.replace("\"", "")).expect(&format!(
                "Error while reading LiDAR file header ({}).",
                in_file
            ));
            bounding_boxes.push(BoundingBox {
                min_x: header.min_x,
                max_x: header.max_x,
                min_y: header.min_y,
                max_y: header.max_y,
            });
        }

        if verbose {
            println!("Performing interpolation...");
        }

        let num_tiles = inputs.len();
        // let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let num_procs = num_cpus::get();
        let inputs = Arc::new(inputs);
        let outputs = Arc::new(outputs);
        let bounding_boxes = Arc::new(bounding_boxes);
        // let num_procs2 = num_cpus::get() as isize;
        let (tx2, rx2) = mpsc::channel();
        for tid in 0..num_procs {
            let inputs = inputs.clone();
            let outputs = outputs.clone();
            let bounding_boxes = bounding_boxes.clone();
            // let tile_list = tile_list.clone();
            // copy over the string parameters
            let interp_parameter = interp_parameter.clone();
            // let palette = palette.clone();
            let return_type = return_type.clone();
            let tool_name = self.get_tool_name();
            let exclude_cls_str = exclude_cls_str.clone();
            let include_class_vals = include_class_vals.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                for tile in (0..num_tiles).filter(|t| t % num_procs == tid) {
                    // let mut tile = 0;
                    // while tile < num_tiles {
                    //     // Get the next tile up for interpolation
                    //     {
                    //         tile = match tile_list.lock().unwrap().next() {
                    //             Some(val) => val,
                    //             None => break, // There are no more tiles to interpolate
                    //         };
                    //     }
                    let start_run = Instant::now();

                    let input_file = inputs[tile].replace("\"", "").clone();
                    let output_file = outputs[tile].replace("\"", "").clone();

                    // Expand the bounding box to include the areas of overlap
                    let bb = BoundingBox {
                        min_x: bounding_boxes[tile].min_x - 2f64,
                        max_x: bounding_boxes[tile].max_x + 2f64,
                        min_y: bounding_boxes[tile].min_y - 2f64,
                        max_y: bounding_boxes[tile].max_y + 2f64,
                    };

                    let mut points: Vec<Point2D> = vec![];
                    let mut z_values: Vec<f64> = vec![];

                    if verbose && inputs.len() == 1 {
                        println!("Reading input LiDAR file...");
                    }

                    let mut progress: i32;
                    let mut old_progress: i32 = -1;
                    let mut epsg_code = 0u16;
                    for m in 0..inputs.len() {
                        if bounding_boxes[m].overlaps(bb) {
                            // let input =
                            //     match LasFile::new(&inputs[m].replace("\"", "").clone(), "r") {
                            //         Ok(lf) => lf,
                            //         Err(err) => panic!(
                            //             "Error reading file {}: {}",
                            //             inputs[m].replace("\"", ""),
                            //             err
                            //         ),
                            //     };

                            match LasFile::new(&inputs[m].replace("\"", "").clone(), "r") {
                                Ok(input) => {
                                    let n_points = input.header.number_of_points as usize;
                                    let num_points: f64 =
                                        (input.header.number_of_points - 1) as f64; // used for progress calculation only
                                    epsg_code = input.get_epsg_code();

                                    match &interp_parameter as &str {
                                        "elevation" | "z" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values.push(xyz.z);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "intensity" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values.push(p.intensity as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "scan angle" | "scan_angle" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values.push(p.scan_angle as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "class" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values.push(p.classification() as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "return_number" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values
                                                                    .push(p.return_number() as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "number_of_returns" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values
                                                                    .push(p.number_of_returns()
                                                                        as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        "rgb" => {
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                // let val = input.get_rgb(i); // ((a << 24) | (b << 16) | (g << 8) | r) as f64;
                                                                z_values
                                                                    .push(p.number_of_returns()
                                                                        as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            // user data
                                            for i in 0..n_points {
                                                let p: PointData = input[i];
                                                let xyz = input.get_transformed_coords(i);
                                                if !p.withheld() {
                                                    if all_returns
                                                        || (p.is_late_return() & late_returns)
                                                        || (p.is_early_return() & early_returns)
                                                    {
                                                        if include_class_vals
                                                            [p.classification() as usize]
                                                        {
                                                            if bb.is_point_in_box(xyz.x, xyz.y)
                                                                && xyz.z >= min_z
                                                                && xyz.z <= max_z
                                                            {
                                                                points.push(Point2D {
                                                                    x: xyz.x,
                                                                    y: xyz.y,
                                                                });
                                                                z_values.push(p.user_data as f64);
                                                            }
                                                        }
                                                    }
                                                }
                                                if verbose && inputs.len() == 1 {
                                                    progress =
                                                        (100.0_f64 * i as f64 / num_points) as i32;
                                                    if progress != old_progress {
                                                        println!("Reading points: {}%", progress);
                                                        old_progress = progress;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_err) => {} // do nothing
                            };
                        }
                    }

                    if points.len() < 3 {
                        println!(
                            "Warning: No eligible points found in {}",
                            inputs[tile].clone()
                        );
                        tx2.send(tile).unwrap();
                    } else {
                        let west: f64 = bounding_boxes[tile].min_x;
                        let north: f64 = bounding_boxes[tile].max_y;
                        let rows: isize =
                            (((north - bounding_boxes[tile].min_y) / grid_res).ceil()) as isize;
                        let columns: isize =
                            (((bounding_boxes[tile].max_x - west) / grid_res).ceil()) as isize;
                        let south: f64 = north - rows as f64 * grid_res;
                        let east = west + columns as f64 * grid_res;
                        let nodata = -32768.0f64;

                        let mut configs = RasterConfigs {
                            ..Default::default()
                        };
                        configs.rows = rows as usize;
                        configs.columns = columns as usize;
                        configs.north = north;
                        configs.south = south;
                        configs.east = east;
                        configs.west = west;
                        configs.resolution_x = grid_res;
                        configs.resolution_y = grid_res;
                        configs.nodata = nodata;
                        configs.data_type = DataType::F32;
                        configs.photometric_interp = PhotometricInterpretation::Continuous;
                        configs.epsg_code = epsg_code;
                        configs.projection = esri_wkt_from_epsg(epsg_code);

                        let mut output = Raster::initialize_using_config(&output_file, &configs);
                        if interp_parameter == "rgb" {
                            output.configs.photometric_interp = PhotometricInterpretation::RGB;
                            output.configs.data_type = DataType::RGBA32;
                        }

                        // do the triangulation
                        if num_tiles == 1 && verbose {
                            println!("Performing triangulation...");
                        }
                        let result = triangulate(&points).expect("No triangulation exists.");
                        let num_triangles = result.triangles.len() / 3;

                        let (mut p1, mut p2, mut p3): (usize, usize, usize);
                        let (mut top, mut bottom, mut left, mut right): (f64, f64, f64, f64);

                        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (
                            isize,
                            isize,
                            isize,
                            isize,
                        );
                        let mut tri_points: Vec<Point2D> = vec![Point2D::new(0f64, 0f64); 4];
                        let mut k: f64;
                        let mut norm: Vector3<f64>;
                        let (mut a, mut b, mut c): (Vector3<f64>, Vector3<f64>, Vector3<f64>);
                        let (mut x, mut y): (f64, f64);
                        let mut zn: f64;
                        let mut i: usize;
                        if !interp_parameter_is_rgb {
                            for triangle in 0..num_triangles {
                                i = triangle * 3;
                                p1 = result.triangles[i];
                                p2 = result.triangles[i + 1];
                                p3 = result.triangles[i + 2];

                                if max_distance_squared(
                                    points[p1],
                                    points[p2],
                                    points[p3],
                                    z_values[p1],
                                    z_values[p2],
                                    z_values[p3],
                                ) < max_triangle_edge_length
                                {
                                    tri_points[0] = points[p1].clone();
                                    tri_points[1] = points[p2].clone();
                                    tri_points[2] = points[p3].clone();
                                    tri_points[3] = points[p1].clone();

                                    // get the equation of the plane
                                    a = Vector3::new(
                                        tri_points[0].x,
                                        tri_points[0].y,
                                        z_values[p1],
                                    );
                                    b = Vector3::new(
                                        tri_points[1].x,
                                        tri_points[1].y,
                                        z_values[p2],
                                    );
                                    c = Vector3::new(
                                        tri_points[2].x,
                                        tri_points[2].y,
                                        z_values[p3],
                                    );
                                    norm = (b - a).cross(&(c - a));
                                    k = -(tri_points[0].x * norm.x
                                        + tri_points[0].y * norm.y
                                        + norm.z * z_values[p1]);

                                    // find grid intersections with this triangle
                                    bottom = points[p1].y.min(points[p2].y.min(points[p3].y));
                                    top = points[p1].y.max(points[p2].y.max(points[p3].y));
                                    left = points[p1].x.min(points[p2].x.min(points[p3].x));
                                    right = points[p1].x.max(points[p2].x.max(points[p3].x));

                                    bottom_row = ((north - bottom) / grid_res).ceil() as isize; // output.get_row_from_y(bottom);
                                    top_row = ((north - top) / grid_res).floor() as isize; // output.get_row_from_y(top);
                                    left_col = ((left - west) / grid_res).floor() as isize; // output.get_column_from_x(left);
                                    right_col = ((right - west) / grid_res).ceil() as isize; // output.get_column_from_x(right);

                                    for row in top_row..=bottom_row {
                                        for col in left_col..=right_col {
                                            x = west + col as f64 * grid_res;
                                            y = north - row as f64 * grid_res;
                                            if point_in_poly(&Point2D::new(x, y), &tri_points) {
                                                // calculate the z values
                                                zn = -(norm.x * x + norm.y * y + k) / norm.z;
                                                output.set_value(row, col, zn);
                                            }
                                        }
                                    }

                                    if verbose && num_tiles == 1 {
                                        progress = (100.0_f64 * triangle as f64
                                            / (num_triangles - 1) as f64)
                                            as i32;
                                        if progress != old_progress {
                                            println!("Progress: {}%", progress);
                                            old_progress = progress;
                                        }
                                    }
                                }
                            }
                        } else {
                            let (mut k_r, mut k_g, mut k_b): (f64, f64, f64);
                            let (mut norm_r, mut norm_g, mut norm_b): (
                                Vector3<f64>,
                                Vector3<f64>,
                                Vector3<f64>,
                            );
                            let (mut red, mut green, mut blue): (f64, f64, f64);
                            for triangle in 0..num_triangles {
                                i = triangle * 3;
                                p1 = result.triangles[i];
                                p2 = result.triangles[i + 1];
                                p3 = result.triangles[i + 2];

                                if max_distance_squared(
                                    points[p1],
                                    points[p2],
                                    points[p3],
                                    z_values[p1],
                                    z_values[p2],
                                    z_values[p3],
                                ) < max_triangle_edge_length
                                {
                                    tri_points[0] = points[p1].clone();
                                    tri_points[1] = points[p2].clone();
                                    tri_points[2] = points[p3].clone();
                                    tri_points[3] = points[p1].clone();

                                    // get the equation of the plane
                                    red = (z_values[p1] as u32 & 0xFF) as f64;
                                    a = Vector3::new(tri_points[0].x, tri_points[0].y, red);
                                    red = (z_values[p2] as u32 & 0xFF) as f64;
                                    b = Vector3::new(tri_points[1].x, tri_points[1].y, red);
                                    red = (z_values[p3] as u32 & 0xFF) as f64;
                                    c = Vector3::new(tri_points[2].x, tri_points[2].y, red);
                                    norm_r = (b - a).cross(&(c - a));
                                    k_r = -(tri_points[2].x * norm_r.x
                                        + tri_points[2].y * norm_r.y
                                        + norm_r.z * red);

                                    green = ((z_values[p1] as u32 >> 8) & 0xFF) as f64;
                                    a = Vector3::new(tri_points[0].x, tri_points[0].y, green);
                                    green = ((z_values[p2] as u32 >> 8) & 0xFF) as f64;
                                    b = Vector3::new(tri_points[1].x, tri_points[1].y, green);
                                    green = ((z_values[p3] as u32 >> 8) & 0xFF) as f64;
                                    c = Vector3::new(tri_points[2].x, tri_points[2].y, green);
                                    norm_g = (b - a).cross(&(c - a));
                                    k_g = -(tri_points[2].x * norm_g.x
                                        + tri_points[2].y * norm_g.y
                                        + norm_g.z * green);

                                    blue = ((z_values[p1] as u32 >> 16) & 0xFF) as f64;
                                    a = Vector3::new(tri_points[0].x, tri_points[0].y, blue);
                                    blue = ((z_values[p2] as u32 >> 16) & 0xFF) as f64;
                                    b = Vector3::new(tri_points[1].x, tri_points[1].y, blue);
                                    blue = ((z_values[p3] as u32 >> 16) & 0xFF) as f64;
                                    c = Vector3::new(tri_points[2].x, tri_points[2].y, blue);
                                    norm_b = (b - a).cross(&(c - a));
                                    k_b = -(tri_points[2].x * norm_b.x
                                        + tri_points[2].y * norm_b.y
                                        + norm_b.z * blue);

                                    // find grid intersections with this triangle
                                    bottom = points[p1].y.min(points[p2].y.min(points[p3].y));
                                    top = points[p1].y.max(points[p2].y.max(points[p3].y));
                                    left = points[p1].x.min(points[p2].x.min(points[p3].x));
                                    right = points[p1].x.max(points[p2].x.max(points[p3].x));

                                    bottom_row = ((north - bottom) / grid_res).ceil() as isize; // output.get_row_from_y(bottom);
                                    top_row = ((north - top) / grid_res).floor() as isize; // output.get_row_from_y(top);
                                    left_col = ((left - west) / grid_res).floor() as isize; // output.get_column_from_x(left);
                                    right_col = ((right - west) / grid_res).ceil() as isize; // output.get_column_from_x(right);

                                    for row in top_row..=bottom_row {
                                        for col in left_col..=right_col {
                                            x = west + col as f64 * grid_res;
                                            y = north - row as f64 * grid_res;
                                            if point_in_poly(&Point2D::new(x, y), &tri_points) {
                                                // calculate the colour values
                                                red =
                                                    -(norm_r.x * x + norm_r.y * y + k_r) / norm_r.z;
                                                green =
                                                    -(norm_g.x * x + norm_g.y * y + k_g) / norm_g.z;
                                                blue =
                                                    -(norm_b.x * x + norm_b.y * y + k_b) / norm_b.z;
                                                zn = ((255u32 << 24)
                                                    | ((blue.round() as u32) << 16)
                                                    | ((green.round() as u32) << 8)
                                                    | (red.round() as u32))
                                                    as f64;
                                                output.set_value(row, col, zn);
                                            }
                                        }
                                    }

                                    if verbose && num_tiles == 1 {
                                        progress = (100.0_f64 * triangle as f64
                                            / (num_triangles - 1) as f64)
                                            as i32;
                                        if progress != old_progress {
                                            println!("Progress: {}%", progress);
                                            old_progress = progress;
                                        }
                                    }
                                }
                            }
                        }

                        let elapsed_time_run = get_formatted_elapsed_time(start_run);
                        output.add_metadata_entry(format!(
                            "Created by whitebox_tools\' {} tool",
                            tool_name
                        ));
                        output.add_metadata_entry(format!("Input file: {}", input_file));
                        output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
                        output.add_metadata_entry(format!(
                            "Interpolation parameter: {}",
                            interp_parameter
                        ));
                        output.add_metadata_entry(format!("Returns: {}", return_type));
                        output.add_metadata_entry(format!("Excluded classes: {}", exclude_cls_str));
                        output.add_metadata_entry(format!(
                            "Elapsed Time (including I/O): {}",
                            elapsed_time_run
                        ));

                        if verbose && inputs.len() == 1 {
                            println!("Saving data...")
                        };

                        let _ = output.write().expect("Error writing file.");

                        tx2.send(tile).unwrap();
                    }
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..inputs.len() {
            let tile_completed = rx2.recv().unwrap();
            if verbose {
                if tile <= 98 {
                    println!(
                        "Finished {} ({} of {})",
                        inputs[tile_completed]
                            .replace("\"", "")
                            .replace(working_directory, "")
                            .replace(".las", ""),
                        tile + 1,
                        inputs.len()
                    );
                } else if tile == 99 {
                    println!(
                        "Finished {} ({} of {})",
                        inputs[tile_completed]
                            .replace("\"", "")
                            .replace(working_directory, "")
                            .replace(".las", ""),
                        tile + 1,
                        inputs.len()
                    );
                    println!("...");
                }
                progress = (100.0_f64 * tile as f64 / (inputs.len() - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

/// Calculate squared Euclidean distance between the point and another.
pub fn max_distance_squared(
    p1: Point2D,
    p2: Point2D,
    p3: Point2D,
    z1: f64,
    z2: f64,
    z3: f64,
) -> f64 {
    let mut dx = p1.x - p2.x;
    let mut dy = p1.y - p2.y;
    let mut dz = z1 - z2;
    let mut max_dist = dx * dx + dy * dy + dz * dz;

    dx = p1.x - p3.x;
    dy = p1.y - p3.y;
    dz = z1 - z3;
    let mut dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    dx = p2.x - p3.x;
    dy = p2.y - p3.y;
    dz = z2 - z3;
    dist = dx * dx + dy * dy + dz * dz;

    if dist > max_dist {
        max_dist = dist
    }

    max_dist
}
