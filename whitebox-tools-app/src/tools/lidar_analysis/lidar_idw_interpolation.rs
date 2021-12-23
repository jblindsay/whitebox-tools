/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 03/07/2017
Last Modified: 19/05/2020
License: MIT

NOTES:
1. This tool is designed to work either by specifying a single input and output file or
   a working directory containing multiple input LAS files.
2. Need to add the ability to exclude points based on max scan angle deviation.
*/

use whitebox_lidar::*;
use whitebox_raster::*;
use whitebox_common::structures::{BoundingBox, DistanceMetric, FixedRadiusSearch2D, Point3D};
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool interpolates LiDAR files using [inverse-distance weighting](https://en.wikipedia.org/wiki/Inverse_distance_weighting)
/// (IDW) scheme. The user must specify the value of the IDW weight parameter (`--weight`). The output grid can be
/// based on any of the stored LiDAR point parameters (`--parameter`), including elevation
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
/// The tool will search for the nearest input LiDAR point to each grid cell centre, up to a maximum search distance
/// (`--radius`). If a grid cell does not have a LiDAR point within this search distance, it will be assigned the
/// NoData value in the output raster. In LiDAR data, these void areas are often associated with larger waterbodies.
/// These NoData areas can later be better dealt with using the `FillMissingData` tool after interpolation.
///
/// # See Also
/// `LidarTINGridding`, `LidarNearestNeighbourGridding`, `LidarSibsonInterpolation`
pub struct LidarIdwInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarIdwInterpolation {
    pub fn new() -> LidarIdwInterpolation {
        // public constructor
        let name = "LidarIdwInterpolation".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Interpolates LAS files using an inverse-distance weighted (IDW) scheme. When the input/output parameters are not specified, the tool interpolates all LAS files contained within the working directory."
            .to_string();

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

        parameters.push(ToolParameter {
            name: "IDW Weight (Exponent) Value".to_owned(),
            flags: vec!["--weight".to_owned()],
            description: "IDW weight value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Search Radius".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Exclusion Classes (0-18, based on LAS spec; e.g. 3,4,5,6,7)".to_owned(), 
            flags: vec!["--exclude_cls".to_owned()], 
            description: "Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0 --radius=5.0\"
.*{0} -r={1} --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=5.0 --weight=2.0 --radius=2.0 --exclude_cls='3,4,5,6,7,18'", short_exe, name).replace("*", &sep);

        LidarIdwInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarIdwInterpolation {
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
        let mut weight = 1.0;
        let mut search_radius = 2.5;
        let mut include_class_vals = vec![true; 256];
        let mut palette = "default".to_string();
        let mut exclude_cls_str = String::new();
        let mut max_z = f64::INFINITY;
        let mut min_z = f64::NEG_INFINITY;

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
            } else if flag_val == "-weight" {
                weight = if keyval {
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
            } else if flag_val == "-radius" {
                search_radius = if keyval {
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
            } else if flag_val == "-palette" {
                palette = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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
            }
        }

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
            let header = LasHeader::read_las_header(&in_file.replace("\"", ""))?;
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
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let inputs = Arc::new(inputs);
        let outputs = Arc::new(outputs);
        let bounding_boxes = Arc::new(bounding_boxes);
        let mut num_procs2 = num_cpus::get() as isize;
        let configurations = whitebox_common::configs::get_configs()?;
        let max_procs = configurations.max_procs;
        if max_procs > 0 && max_procs < num_procs2 {
            num_procs2 = max_procs;
        }
        let (tx2, rx2) = mpsc::channel();
        for _ in 0..num_procs2 {
            let inputs = inputs.clone();
            let outputs = outputs.clone();
            let bounding_boxes = bounding_boxes.clone();
            let tile_list = tile_list.clone();
            // copy over the string parameters
            let interp_parameter = interp_parameter.clone();
            let palette = palette.clone();
            let return_type = return_type.clone();
            let tool_name = self.get_tool_name();
            let exclude_cls_str = exclude_cls_str.clone();
            let include_class_vals = include_class_vals.clone();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let mut tile = 0;
                while tile < num_tiles {
                    // Get the next tile up for interpolation
                    {
                        tile = match tile_list.lock().unwrap().next() {
                            Some(val) => val,
                            None => break, // There are no more tiles to interpolate
                        };
                    }
                    // for tile in (0..inputs.len()).filter(|t| t % num_procs2 as usize == tid as usize) {
                    let start_run = Instant::now();

                    let input_file = inputs[tile].replace("\"", "").clone();
                    let output_file = outputs[tile].replace("\"", "").clone();

                    // Expand the bounding box to include the areas of overlap
                    let bb = BoundingBox {
                        min_x: bounding_boxes[tile].min_x - search_radius,
                        max_x: bounding_boxes[tile].max_x + search_radius,
                        min_y: bounding_boxes[tile].min_y - search_radius,
                        max_y: bounding_boxes[tile].max_y + search_radius,
                    };
                    let mut frs: FixedRadiusSearch2D<f64> =
                        FixedRadiusSearch2D::new(search_radius, DistanceMetric::Euclidean);

                    if verbose && inputs.len() == 1 {
                        println!("reading input LiDAR file...");
                    }

                    let mut progress: i32;
                    let mut old_progress: i32 = -1;

                    for m in 0..inputs.len() {
                        if bounding_boxes[m].overlaps(bb) {
                            let input =
                                match LasFile::new(&inputs[m].replace("\"", "").clone(), "r") {
                                    Ok(lf) => lf,
                                    Err(err) => panic!(
                                        "Error reading file {}: {}",
                                        inputs[m].replace("\"", ""),
                                        err
                                    ),
                                };

                            let n_points = input.header.number_of_points as usize;
                            let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only
                            let mut p: Point3D;
                            let mut pd: PointData;
                            match &interp_parameter as &str {
                                "elevation" | "z" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(p.x, p.y, p.z);
                                                    }
                                                }
                                            }
                                        }

                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Binning points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "intensity" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(p.x, p.y, pd.intensity as f64);
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Binning points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "scan angle" | "scan_angle" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(p.x, p.y, pd.scan_angle as f64);
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Binning points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "class" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(
                                                            p.x,
                                                            p.y,
                                                            pd.classification() as f64,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Binning points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "return_number" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(
                                                            p.x,
                                                            p.y,
                                                            pd.return_number() as f64,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Reading points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "number_of_returns" => {
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(
                                                            p.x,
                                                            p.y,
                                                            pd.number_of_returns() as f64,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Reading points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                                "rgb" => {
                                    if !input.has_rgb() {
                                        println!("Error: The input LAS file does not contain RGB colour data. The interpolation will not proceed.");
                                        break;
                                    }
                                    let mut clr: ColourData;
                                    for i in 0..n_points {
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        clr = match input.get_rgb(i) {
                                                            Ok(value) => value,
                                                            Err(_) => break,
                                                        };
                                                        frs.insert(
                                                            p.x,
                                                            p.y,
                                                            ((255u32 << 24)
                                                                | ((clr.blue as u32) << 16)
                                                                | ((clr.green as u32) << 8)
                                                                | (clr.red as u32))
                                                                as f64,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
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
                                        pd = input[i];
                                        p = input.get_transformed_coords(i);
                                        if !pd.withheld() {
                                            if all_returns
                                                || (pd.is_late_return() & late_returns)
                                                || (pd.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[pd.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        frs.insert(p.x, p.y, pd.user_data as f64);
                                                    }
                                                }
                                            }
                                        }
                                        if verbose && inputs.len() == 1 {
                                            progress = (100.0_f64 * i as f64 / num_points) as i32;
                                            if progress != old_progress {
                                                println!("Binning points: {}%", progress);
                                                old_progress = progress;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if frs.size() == 0 {
                        println!("Warning: No points found in {}.", inputs[tile].clone());
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
                        configs.palette = palette.clone();

                        let mut output = Raster::initialize_using_config(&output_file, &configs);
                        if interp_parameter == "rgb" {
                            output.configs.photometric_interp = PhotometricInterpretation::RGB;
                            output.configs.data_type = DataType::RGBA32;
                        }

                        if num_tiles > 1 {
                            let (mut x, mut y): (f64, f64);
                            let mut zn: f64;
                            let mut dist: f64;
                            let mut val: f64;
                            let (mut val_red, mut val_green, mut val_blue): (f64, f64, f64);
                            let (mut red, mut green, mut blue): (f64, f64, f64);
                            let mut sum_weights: f64;
                            for row in 0..rows {
                                for col in 0..columns {
                                    x = west + (col as f64 + 0.5) * grid_res;
                                    y = north - (row as f64 + 0.5) * grid_res;
                                    let ret = frs.search(x, y);
                                    if ret.len() > 0 {
                                        sum_weights = 0.0;
                                        val = 0.0;
                                        val_red = 0f64;
                                        val_green = 0f64;
                                        val_blue = 0f64;
                                        for j in 0..ret.len() {
                                            zn = ret[j].0;
                                            dist = ret[j].1 as f64;
                                            if dist > 0.0 {
                                                if !interp_parameter_is_rgb {
                                                    val += zn / dist.powf(weight);
                                                } else {
                                                    red = (zn as u32 & 0xFF) as f64;
                                                    green = ((zn as u32 >> 8) & 0xFF) as f64;
                                                    blue = ((zn as u32 >> 16) & 0xFF) as f64;
                                                    val_red += red / dist.powf(weight);
                                                    val_green += green / dist.powf(weight);
                                                    val_blue += blue / dist.powf(weight);
                                                }
                                                sum_weights += 1.0 / dist.powf(weight);
                                            } else {
                                                output.set_value(row, col, zn);
                                                sum_weights = 0.0;
                                                break;
                                            }
                                        }
                                        if sum_weights > 0.0 {
                                            if interp_parameter_is_rgb {
                                                val = ((255u32 << 24)
                                                    | ((val_blue.round() as u32) << 16)
                                                    | ((val_green.round() as u32) << 8)
                                                    | (val_red.round() as u32))
                                                    as f64;
                                            }
                                            output.set_value(row, col, val / sum_weights);
                                        }
                                    }
                                }
                                if verbose && inputs.len() == 1 {
                                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                                    if progress != old_progress {
                                        println!("Progress: {}%", progress);
                                        old_progress = progress;
                                    }
                                }
                            }
                        } else {
                            // there's only one tile, so use all cores to interpolate this one tile.
                            let frs = Arc::new(frs); // wrap FRS in an Arc
                            let mut num_procs = num_cpus::get() as isize;
                            let configs = whitebox_common::configs::get_configs().unwrap();
                            let max_procs = configs.max_procs;
                            if max_procs > 0 && max_procs < num_procs {
                                num_procs = max_procs;
                            }
                            let (tx, rx) = mpsc::channel();
                            for tid in 0..num_procs {
                                let frs = frs.clone();
                                let tx1 = tx.clone();
                                thread::spawn(move || {
                                    let (mut x, mut y): (f64, f64);
                                    let mut zn: f64;
                                    let mut dist: f64;
                                    let mut val: f64;
                                    let mut sum_weights: f64;
                                    let (mut val_red, mut val_green, mut val_blue): (
                                        f64,
                                        f64,
                                        f64,
                                    );
                                    let (mut red, mut green, mut blue): (f64, f64, f64);
                                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                                        let mut data = vec![nodata; columns as usize];
                                        for col in 0..columns {
                                            x = west + (col as f64 + 0.5) * grid_res;
                                            y = north - (row as f64 + 0.5) * grid_res;
                                            let ret = frs.search(x, y);
                                            if ret.len() > 0 {
                                                sum_weights = 0.0;
                                                val = 0.0;
                                                val_red = 0f64;
                                                val_green = 0f64;
                                                val_blue = 0f64;
                                                for j in 0..ret.len() {
                                                    zn = ret[j].0;
                                                    dist = ret[j].1 as f64;
                                                    if dist > 0.0 {
                                                        if !interp_parameter_is_rgb {
                                                            val += zn / dist.powf(weight);
                                                        } else {
                                                            red = (zn as u32 & 0xFF) as f64;
                                                            green =
                                                                ((zn as u32 >> 8) & 0xFF) as f64;
                                                            blue =
                                                                ((zn as u32 >> 16) & 0xFF) as f64;
                                                            val_red += red / dist.powf(weight);
                                                            val_green += green / dist.powf(weight);
                                                            val_blue += blue / dist.powf(weight);
                                                        }
                                                        sum_weights += 1.0 / dist.powf(weight);
                                                    } else {
                                                        data[col as usize] = zn;
                                                        sum_weights = 0.0;
                                                        break;
                                                    }
                                                }
                                                if sum_weights > 0.0 {
                                                    if interp_parameter_is_rgb {
                                                        val = ((255u32 << 24)
                                                            | ((val_blue.round() as u32) << 16)
                                                            | ((val_green.round() as u32) << 8)
                                                            | (val_red.round() as u32))
                                                            as f64;
                                                    }
                                                    data[col as usize] = val / sum_weights;
                                                }
                                            }
                                        }
                                        tx1.send((row, data)).unwrap();
                                    }
                                });
                            }

                            for row in 0..rows {
                                let data = rx.recv().expect("Error receiving data from thread.");
                                output.set_row_data(data.0, data.1);
                                if verbose {
                                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                                    if progress != old_progress {
                                        println!("Progress: {}%", progress);
                                        old_progress = progress;
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
                        output.add_metadata_entry(format!("Search radius: {}", search_radius));
                        output.add_metadata_entry(format!("Weight: {}", weight));
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

                        let _ = output.write().unwrap();

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
