/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/12/2019
Last Modified: 10/12/2019
License: MIT
*/

use whitebox_common::algorithms::{convex_hull, point_in_poly};
use whitebox_raster::*;
use whitebox_common::structures::{Basis, Point2D, RadialBasisFunction};
use crate::tools::*;
use whitebox_vector::{FieldData, ShapeType, Shapefile};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use nalgebra::DVector;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool interpolates vector points into a raster surface using a radial basis function (RBF) scheme.
pub struct RadialBasisFunctionInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RadialBasisFunctionInterpolation {
    /// public constructor
    pub fn new() -> RadialBasisFunctionInterpolation {
        let name = "RadialBasisFunctionInterpolation".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Interpolates vector points into a raster surface using a radial basis function scheme.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Input field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Use z-coordinate instead of field?".to_owned(),
            flags: vec!["--use_z".to_owned()],
            description: "Use z-coordinate instead of field?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Search Radius (map units)".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius (in map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Min. Number of Points".to_owned(),
            flags: vec!["--min_points".to_owned()],
            description: "Minimum number of points.".to_owned(),
            parameter_type: ParameterType::Integer,
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
            default_value: Some("0.1".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Base Raster File (optional)".to_owned(), 
            flags: vec!["--base".to_owned()], 
            description: "Optionally specified input base raster file. Not used when a cell size is specified.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=ELEV -o=output.tif --weight=2.0 --radius=4.0 --min_points=3 --cell_size=1.0", short_exe, name).replace("*", &sep);

        RadialBasisFunctionInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RadialBasisFunctionInterpolation {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut field_name = String::new();
        // let mut use_field = false;
        let mut use_z = false;
        let mut output_file = String::new();
        let mut grid_res = 0f64;
        let mut base_file = String::new();
        let mut radius = 0f64;
        let mut min_points = 0usize;
        let mut func_type = String::from("ThinPlateSpline");
        let mut poly_order = 0usize;
        let mut weight = 0.1f64;

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
            } else if flag_val == "-field" {
                field_name = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            // use_field = true;
            } else if flag_val == "-use_z" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_z = true;
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" || flag_val == "-cell_size" {
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
            } else if flag_val == "-base" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-radius" {
                radius = if keyval {
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
            } else if flag_val == "-min_points" {
                min_points = if keyval {
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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        radius = radius * radius; // squared distances are used

        if verbose {
            println!("Reading data...")
        };
        let input = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let mut points = vec![];
        let mut points_for_hull = vec![];
        let mut z_values = vec![];
        let mut z: f64;
        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        const DIMENSIONS: usize = 2;
        const CAPACITY_PER_NODE: usize = 64;
        let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
        let mut p = 0;
        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            if record.shape_type != ShapeType::Null {
                for i in 0..record.num_points as usize {
                    points.push(DVector::from_vec(vec![
                        record.points[i].x,
                        record.points[i].y,
                    ]));
                    points_for_hull.push(Point2D::new(record.points[i].x, record.points[i].y));
                    z = if use_z {
                        record.z_array[i]
                    } else {
                        let val = match input.attributes.get_value(record_num, &field_name) {
                            FieldData::Int(val) => val as f64,
                            FieldData::Real(val) => val,
                            FieldData::Null => continue,
                            _ => {
                                return Err(Error::new(
                                    ErrorKind::InvalidInput,
                                    "Error: Only vector fields of Int and Real data type may be used as inputs.",
                                ));
                            }
                        };
                        val
                    };
                    z_values.push(DVector::from_vec(vec![z]));
                    if z < min_value {
                        min_value = z;
                    }
                    if z > max_value {
                        max_value = z;
                    }
                    tree.add([record.points[i].x, record.points[i].y], p)
                        .unwrap();
                    p += 1;
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Reading points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if z_values.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error in reading the input point value data.",
            ));
        }

        let range = max_value - min_value;
        let range_threshold = range * 2f64;
        let mid_point = min_value + range / 2f64;

        // get the convex hull
        let mut hull = convex_hull(&mut points_for_hull);
        hull.push(hull[0].clone());
        drop(points_for_hull);

        // Create the output raster. The process of doing this will
        // depend on whether a cell size or a base raster were specified.
        // If both are specified, the base raster takes priority.

        let nodata = -32768.0f64;

        let mut output = if !base_file.trim().is_empty() || grid_res == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let mut base = Raster::new(&base_file, "r")?;
            base.configs.nodata = nodata;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            if grid_res == 0f64 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified grid resolution is incorrect. Either a non-zero grid resolution \nor an input existing base file name must be used.",
                ));
            }
            // base the output raster on the grid_res and the
            // extent of the input vector.
            let west: f64 = input.header.x_min;
            let north: f64 = input.header.y_max;
            let rows: isize = (((north - input.header.y_min) / grid_res).ceil()) as isize;
            let columns: isize = (((input.header.x_max - west) / grid_res).ceil()) as isize;
            let south: f64 = north - rows as f64 * grid_res;
            let east = west + columns as f64 * grid_res;

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

            Raster::initialize_using_config(&output_file, &configs)
        };

        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;
        let west = output.configs.west;
        let north = output.configs.north;
        output.configs.nodata = nodata; // in case a base image is used with a different nodata value.
        let res_x = output.configs.resolution_x;
        let res_y = output.configs.resolution_y;

        let points = Arc::new(points);
        let z_values = Arc::new(z_values);
        let hull = Arc::new(hull);
        let tree = Arc::new(tree);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let points = points.clone();
            let z_values = z_values.clone();
            let hull = hull.clone();
            let tree = tree.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut x, mut y): (f64, f64);
                let mut z: f64;
                let mut point_num;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        x = west + (col as f64 + 0.5) * res_x;
                        y = north - (row as f64 + 0.5) * res_y;
                        if point_in_poly(&Point2D::new(x, y), &hull) {
                            let mut ret = tree.within(&[x, y], radius, &squared_euclidean).unwrap();
                            if ret.len() < min_points {
                                ret = tree
                                    .nearest(&[x, y], min_points, &squared_euclidean)
                                    .unwrap();
                            }
                            if ret.len() > 0 {
                                let mut centers: Vec<DVector<f64>> = Vec::with_capacity(ret.len());
                                let mut vals: Vec<DVector<f64>> = Vec::with_capacity(ret.len());
                                for p in ret {
                                    point_num = *(p.1);
                                    centers.push(points[point_num].clone());
                                    vals.push(z_values[point_num].clone());
                                }
                                let rbf = RadialBasisFunction::create(
                                    centers, vals, basis_func, poly_order,
                                );
                                z = rbf.eval(DVector::from_vec(vec![x, y]))[0];
                                if (z - mid_point).abs() < range_threshold {
                                    // if the estimated value is well outside of the range of values in the input points, don't output it.
                                    data[col as usize] = z;
                                }
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_max = max_value;
        output.configs.display_min = min_value;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Search radius: {}", radius.sqrt()));
        output.add_metadata_entry(format!("Min. num. points: {}", min_points));
        output.add_metadata_entry(format!("Radial basis function type: {}", func_type));
        output.add_metadata_entry(format!("Polynomial order: {}", poly_order));
        output.add_metadata_entry(format!("Weight: {}", weight));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
