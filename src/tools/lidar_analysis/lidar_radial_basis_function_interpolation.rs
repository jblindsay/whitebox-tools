/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 08/11/2019
Last Modified: 15/12/2019
License: MIT

NOTES:
1. This tool is designed to work either by specifying a single input and output file or
   a working directory containing multiple input LAS files.
2. Need to add the ability to exclude points based on max scan angle deviation.
*/

use crate::lidar::*;
use crate::raster::*;
use crate::structures::{Basis, BoundingBox, RadialBasisFunction};
use crate::tools::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use nalgebra::DVector;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct LidarRbfInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarRbfInterpolation {
    pub fn new() -> LidarRbfInterpolation {
        // public constructor
        let name = "LidarRbfInterpolation".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Interpolates LAS files using a radial basis function (RBF) scheme. When the input/output parameters are not specified, the tool interpolates all LAS files contained within the working directory."
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
            name: "Number of Points".to_owned(),
            flags: vec!["--num_points".to_owned()],
            description: "Number of points.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("20".to_string()),
            optional: false,
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

        parameters.push(ToolParameter{
            name: "Radial Basis Function Type".to_owned(), 
            flags: vec!["--func_type".to_owned()], 
            description: "Radial basis function type; options are 'ThinPlateSpline' (default), 'PolyHarmonic', 'Gaussian', 'MultiQuadric', 'InverseMultiQuadric'.".to_owned(),
            parameter_type: ParameterType::OptionList(
                vec![
                    "ThinPlateSpline".to_owned(),
                    "PolyHarmonic".to_owned(), 
                    "Gaussian".to_owned(), 
                    "MultiQuadric".to_owned(), 
                    "InverseMultiQuadric".to_owned()
                ]
            ),
            default_value: Some("ThinPlateSpline".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Polynomial Order".to_owned(),
            flags: vec!["--poly_order".to_owned()],
            description: "Polynomial order; options are 'none' (default), 'constant', 'affine'."
                .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "none".to_owned(),
                "constant".to_owned(),
                "affine".to_owned(),
            ]),
            default_value: Some("none".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Weight".to_owned(),
            flags: vec!["--weight".to_owned()],
            description: "Weight parameter used in basis function.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("5".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0 --radius=5.0", short_exe, name).replace("*", &sep);

        LidarRbfInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarRbfInterpolation {
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
        // let mut interp_parameter_is_rgb = false;
        let mut return_type = "all".to_string();
        let mut grid_res: f64 = 1.0;
        let search_radius = 1.0;
        let mut include_class_vals = vec![true; 256];
        let mut palette = "default".to_string();
        let mut exclude_cls_str = String::new();
        let mut max_z = f64::INFINITY;
        let mut min_z = f64::NEG_INFINITY;
        let mut func_type = String::from("ThinPlateSpline");
        let mut poly_order = 0usize;
        let mut weight = 0.1f64;
        let mut num_points = 15usize;

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
            // } else if flag_val == "-radius" {
            //     search_radius = if keyval {
            //         vec[1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
            //     } else {
            //         args[i + 1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
            //     };
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
            } else if flag_val == "-func_type" {
                func_type = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };
            } else if flag_val == "-poly_order" {
                let s = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };
                poly_order = if s.contains("none") {
                    0usize
                } else if s.contains("const") {
                    1usize
                } else {
                    2usize
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
            } else if flag_val == "-num_points" {
                num_points = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            }
        }

        let basis_func = if func_type.contains("thin") {
            Basis::ThinPlateSpine(weight)
        } else if func_type.contains("PolyHarmonic") {
            Basis::PolyHarmonic(weight as i32)
        } else if func_type.contains("Gaussian") {
            Basis::Gaussian(weight)
        } else if func_type.contains("MultiQuadric") {
            Basis::MultiQuadric(weight)
        } else {
            //if func_type.contains("InverseMultiQuadric") {
            Basis::InverseMultiQuadric(weight)
        };

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
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
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
                    .replace(".LAS", ".tif");
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

        let func_type = Arc::new(func_type);
        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let inputs = Arc::new(inputs);
        let outputs = Arc::new(outputs);
        let bounding_boxes = Arc::new(bounding_boxes);
        let num_procs2 = num_cpus::get() as isize;
        let (tx2, rx2) = mpsc::channel();
        for _ in 0..num_procs2 {
            let func_type = func_type.clone();
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
                    tile = match tile_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to interpolate
                    };
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

                    const DIMENSIONS: usize = 2;
                    const CAPACITY_PER_NODE: usize = 64;
                    let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
                    let mut min_value = f64::INFINITY;
                    let mut max_value = f64::NEG_INFINITY;

                    let mut points = vec![];
                    let mut z_values = vec![];
                    let mut z: f64;

                    if verbose && inputs.len() == 1 {
                        println!("Reading input LAS file...");
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

                            let mut pt = 0;
                            match &interp_parameter as &str {
                                "elevation" | "z" => {
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.z;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![p.z]));
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
                                "intensity" => {
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.intensity as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![
                                                            p.intensity as f64,
                                                        ]));
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
                                "scan angle" | "scan_angle" => {
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.scan_angle as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![
                                                            p.scan_angle as f64,
                                                        ]));
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
                                "class" => {
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.classification() as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![p
                                                            .classification()
                                                            as f64]));
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
                                "return_number" => {
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.return_number() as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![p
                                                            .return_number()
                                                            as f64]));
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
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.number_of_returns() as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![p
                                                            .number_of_returns()
                                                            as f64]));
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
                                    // let mut clr: ColourData;
                                    for i in 0..n_points {
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        // clr = match input.get_rgb(i) {
                                                        //     Ok(value) => { value },
                                                        //     Err(_) => break,
                                                        // };
                                                        // ***************************
                                                        // This needs to be fixed
                                                        // ***************************
                                                        // frs.insert(p.x, p.y, ((255u32 << 24) | ((clr.blue as u32) << 16) | ((clr.green as u32) << 8) | (clr.red as u32)) as f64);
                                                        // frs.insert(p.x, p.y, pt);
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.z;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![
                                                            p.z as f64,
                                                        ]));
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
                                        let p: PointData = input[i];
                                        if !p.withheld() {
                                            if all_returns
                                                || (p.is_late_return() & late_returns)
                                                || (p.is_early_return() & early_returns)
                                            {
                                                if include_class_vals[p.classification() as usize] {
                                                    if bb.is_point_in_box(p.x, p.y)
                                                        && p.z >= min_z
                                                        && p.z <= max_z
                                                    {
                                                        tree.add([p.x, p.y], pt).unwrap();
                                                        z = p.user_data as f64;
                                                        if z < min_value {
                                                            min_value = z;
                                                        }
                                                        if z > max_value {
                                                            max_value = z;
                                                        }
                                                        pt += 1;
                                                        points.push(DVector::from_vec(vec![
                                                            p.x, p.y,
                                                        ]));
                                                        z_values.push(DVector::from_vec(vec![
                                                            p.user_data as f64,
                                                        ]));
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
                            }
                        }
                    }

                    let range = max_value - min_value;
                    let range_threshold = range * 1f64; // only estimated values that are +/- 0.5 range beyond the min and max values will be output
                    let mid_point = min_value + range / 2f64;

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
                        let mut point_num: usize;
                        for row in 0..rows {
                            for col in 0..columns {
                                x = west + (col as f64 + 0.5) * grid_res;
                                y = north - (row as f64 + 0.5) * grid_res;
                                let ret = tree
                                    .nearest(&[x, y], num_points, &squared_euclidean)
                                    .unwrap();
                                if ret.len() > 0 {
                                    let mut centers: Vec<DVector<f64>> =
                                        Vec::with_capacity(ret.len());
                                    let mut vals: Vec<DVector<f64>> = Vec::with_capacity(ret.len());
                                    for p in ret {
                                        point_num = *(p.1);
                                        centers.push(points[point_num].clone());
                                        vals.push(z_values[point_num].clone());
                                    }
                                    let rbf = RadialBasisFunction::create(
                                        centers, vals, basis_func, poly_order,
                                    );
                                    zn = rbf.eval(DVector::from_vec(vec![x, y]))[0];
                                    if (zn - mid_point).abs() < range_threshold {
                                        // if the estimated value is well outside of the range of values in the input points, don't output it.
                                        output.set_value(row, col, zn);
                                    }
                                } else {
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
                        // there's only one file, so use all cores to interpolate this one tile.
                        let points = Arc::new(points);
                        let z_values = Arc::new(z_values);
                        let tree = Arc::new(tree);
                        let num_procs = num_cpus::get() as isize;
                        let (tx, rx) = mpsc::channel();
                        for tid in 0..num_procs {
                            let tree = tree.clone();
                            let tx1 = tx.clone();
                            let points = points.clone();
                            let z_values = z_values.clone();
                            thread::spawn(move || {
                                let (mut x, mut y): (f64, f64);
                                let mut zn: f64;
                                let mut point_num: usize;
                                for row in (0..rows).filter(|r| r % num_procs == tid) {
                                    let mut data = vec![nodata; columns as usize];
                                    for col in 0..columns {
                                        x = west + (col as f64 + 0.5) * grid_res;
                                        y = north - (row as f64 + 0.5) * grid_res;
                                        let ret = tree
                                            .nearest(&[x, y], num_points, &squared_euclidean)
                                            .unwrap();
                                        if ret.len() > 0 {
                                            let mut centers: Vec<DVector<f64>> =
                                                Vec::with_capacity(ret.len());
                                            let mut vals: Vec<DVector<f64>> =
                                                Vec::with_capacity(ret.len());
                                            for p in ret {
                                                point_num = *(p.1);
                                                centers.push(points[point_num].clone());
                                                vals.push(z_values[point_num].clone());
                                            }
                                            let rbf = RadialBasisFunction::create(
                                                centers, vals, basis_func, poly_order,
                                            );
                                            zn = rbf.eval(DVector::from_vec(vec![x, y]))[0];
                                            if (zn - mid_point).abs() < range_threshold {
                                                // if the estimated value is well outside of the range of values in the input points, don't output it.
                                                data[col as usize] = zn;
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
                    output.configs.display_max = max_value;
                    output.configs.display_min = min_value;
                    output.add_metadata_entry(format!(
                        "Created by whitebox_tools\' {} tool",
                        tool_name
                    ));
                    output.add_metadata_entry(format!("Input file: {}", input_file));
                    output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
                    output.add_metadata_entry(format!("Num. points: {}", num_points));
                    output.add_metadata_entry(format!("Radial basis function type: {}", func_type));
                    output.add_metadata_entry(format!("Polynomial order: {}", poly_order));
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
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..inputs.len() {
            let tile_completed = rx2.recv().unwrap();
            if verbose {
                println!(
                    "Finished interpolating {} ({} of {})",
                    inputs[tile_completed]
                        .replace("\"", "")
                        .replace(working_directory, "")
                        .replace(".las", ""),
                    tile + 1,
                    inputs.len()
                );
            }
            if verbose {
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
