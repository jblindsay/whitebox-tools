/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/05/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::{DistanceMetric, FixedRadiusSearch2D};
use crate::tools::*;
use crate::vector::{FieldData, ShapeType, Shapefile};
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool interpolates vector points into a raster surface using an inverse-distance weighted scheme.
///
/// Most IDW tool have the option to work either based on a fixed number of neighbouring
/// points or a fixed neighbourhood size. This tool is currently configured to perform the later
/// only, using a FixedRadiusSearch structure. Using a fixed number of neighbours will require
/// use of a KD-tree structure. I've been testing one Rust KD-tree library but its performance
/// does not appear to be satisfactory compared to the FixedRadiusSearch. I will need to explore
/// other options here.
///
/// Another change that will need to be implemented is the use of a nodal function. The original
/// Whitebox GAT tool allows for use of a constant or a quadratic. This tool only allows the
/// former.
pub struct IdwInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl IdwInterpolation {
    /// public constructor
    pub fn new() -> IdwInterpolation {
        let name = "IdwInterpolation".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Interpolates vector points into a raster surface using an inverse-distance weighted scheme.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector Points file.".to_owned(),
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
            name: "IDW Weight (Exponent) Value".to_owned(),
            flags: vec!["--weight".to_owned()],
            description: "IDW weight value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Search Radius".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=ELEV -o=output.tif --weight=2.0 --radius=4.0 --min_points=3 --cell_size=1.0
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --use_z -o=output.tif --weight=2.0 --radius=4.0 --min_points=3 --base=existing_raster.tif", short_exe, name).replace("*", &sep);

        IdwInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for IdwInterpolation {
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
        let mut use_z = false;
        let mut output_file = String::new();
        let mut grid_res = 0f64;
        let mut base_file = String::new();
        let mut weight = 2f64;
        let mut radius = 0f64;
        let mut min_points = 0usize;
        // let mut max_dist = f64::INFINITY;

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            } else if flag_val == "-use_z" {
                use_z = true;
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-cell_size" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-base" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-weight" {
                weight = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-radius" {
                radius = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-min_points" {
                min_points = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap() as usize
                };
                // } else if flag_val == "-max_dist" {
                //     max_dist = if keyval {
                //         vec[1].to_string().parse::<f64>().unwrap()
                //     } else {
                //         args[i+1].to_string().parse::<f64>().unwrap()
                //     };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // radius = radius * radius; // squared distances are used

        // if max_dist != f64::INFINITY {
        //     max_dist = max_dist * max_dist; // square the max dist
        // }

        if verbose {
            println!("Reading data...")
        };
        let vector_data = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        // // Create the kd tree
        let (mut x, mut y, mut z): (f64, f64, f64);
        // let mut points = vec![];
        // for record_num in 0..vector_data.num_records {
        //     let record = vector_data.get_record(record_num);
        //     for i in 0..record.points.len() {
        //         x = record.points[i].x;
        //         y = record.points[i].y;
        //         points.push([x, y]);
        //     }
        // }

        // let kdtree = if !use_z {
        //     // use the specified attribute

        //     // What is the index of the field to be analyzed?
        //     let field_index = match vector_data.attributes.get_field_num(&field_name) {
        //         Some(i) => i,
        //         None => {
        //             // Field not found
        //             return Err(Error::new(ErrorKind::InvalidInput,
        //                 "Attribute not found in table."));
        //         },
        //     };

        //     // Is the field numeric?
        //     if !vector_data.attributes.is_field_numeric(field_index) {
        //         // Warn user of non-numeric
        //         return Err(Error::new(ErrorKind::InvalidInput,
        //             "Non-numeric attributes cannot be rasterized."));
        //     }

        //     let mut kdtree = KdTree::new_with_capacity(2, vector_data.num_records);

        //     for record_num in 0..vector_data.num_records {
        //         match vector_data.attributes.get_field_value(record_num, field_index) {
        //             FieldData::Int(val) => {
        //                 kdtree.add(points[record_num], val as f64).unwrap();
        //             },
        //             FieldData::Int64(val) => {
        //                 kdtree.add(points[record_num], val as f64).unwrap();
        //             },
        //             FieldData::Real(val) => {
        //                 kdtree.add(points[record_num], val as f64).unwrap();
        //             },
        //             _ => {
        //                 // do nothing; likely due to null value for record.
        //             }
        //         }

        //         if verbose {
        //             progress = (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Creating kd-tree: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }

        //     kdtree
        // } else {
        //     // use the z dimension of the point data.
        //     if vector_data.header.shape_type != ShapeType::PointZ &&
        //         vector_data.header.shape_type != ShapeType::PointM &&
        //         vector_data.header.shape_type != ShapeType::MultiPointZ &&
        //         vector_data.header.shape_type != ShapeType::MultiPointM {
        //         return Err(Error::new(ErrorKind::InvalidInput,
        //             "The input vector data must be of PointZ, PointM, MultiPointZ, or MultiPointM shape type."));
        //     }

        //     let mut kdtree = KdTree::new_with_capacity(2, vector_data.num_records);

        //     let mut p = 0;
        //     for record_num in 0..vector_data.num_records {
        //         let record = vector_data.get_record(record_num);
        //         for i in 0..record.z_array.len() {
        //             z = record.z_array[i];
        //             kdtree.add(points[p], z).unwrap();
        //             p += 1;
        //         }

        //         if verbose {
        //             progress = (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Creating kd-tree: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }

        //     kdtree
        // };

        let frs = if !use_z {
            // use the specified attribute

            // What is the index of the field to be analyzed?
            let field_index = match vector_data.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    // Field not found
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Attribute not found in table.",
                    ));
                }
            };

            // Is the field numeric?
            if !vector_data.attributes.is_field_numeric(field_index) {
                // Warn user of non-numeric
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Non-numeric attributes cannot be rasterized.",
                ));
            }

            let mut frs: FixedRadiusSearch2D<f64> =
                FixedRadiusSearch2D::new(radius, DistanceMetric::Euclidean);

            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                x = record.points[0].x;
                y = record.points[0].y;
                match vector_data.attributes.get_value(record_num, &field_name) {
                    FieldData::Int(val) => {
                        frs.insert(x, y, val as f64);
                    }
                    // FieldData::Int64(val) => {
                    //     frs.insert(x, y, val as f64);
                    // },
                    FieldData::Real(val) => {
                        frs.insert(x, y, val);
                    }
                    _ => {
                        // do nothing; likely due to null value for record.
                    }
                }

                if verbose {
                    progress = (100.0_f64 * record_num as f64
                        / (vector_data.num_records - 1) as f64)
                        as usize;
                    if progress != old_progress {
                        println!("Creating search structure: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            frs
        } else {
            // use the z dimension of the point data.
            if vector_data.header.shape_type != ShapeType::PointZ
                && vector_data.header.shape_type != ShapeType::PointM
                && vector_data.header.shape_type != ShapeType::MultiPointZ
                && vector_data.header.shape_type != ShapeType::MultiPointM
            {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "The input vector data must be of PointZ, PointM, MultiPointZ, or MultiPointM shape type."));
            }

            let mut frs: FixedRadiusSearch2D<f64> =
                FixedRadiusSearch2D::new(radius, DistanceMetric::Euclidean);

            // let mut p = 0;
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.z_array.len() {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    z = record.z_array[i];
                    frs.insert(x, y, z);
                    // p += 1;
                }

                if verbose {
                    progress = (100.0_f64 * record_num as f64
                        / (vector_data.num_records - 1) as f64)
                        as usize;
                    if progress != old_progress {
                        println!("Creating search structure: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            frs
        };

        // Create the output raster. The process of doing this will
        // depend on whether a cell size or a base raster were specified.
        // If both are specified, the base raster takes priority.

        let nodata = -32768.0f64;

        let mut output = if !base_file.trim().is_empty() || grid_res == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let base = Raster::new(&base_file, "r")?;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            // base the output raster on the grid_res and the
            // extent of the input vector.
            let west: f64 = vector_data.header.x_min;
            let north: f64 = vector_data.header.y_max;
            let rows: isize = (((north - vector_data.header.y_min) / grid_res).ceil()) as isize;
            let columns: isize = (((vector_data.header.x_max - west) / grid_res).ceil()) as isize;
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

        // let kdtree = Arc::new(kdtree); // wrap FRS in an Arc
        let frs = Arc::new(frs);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            // let kdtree = kdtree.clone();
            let frs = frs.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut x, mut y): (f64, f64);
                let mut zn: f64;
                let mut dist: f64;
                let mut val: f64;
                let mut sum_weights: f64;
                // let diff_weight = weight - 2f64; // diff between weight and 2, because distances are returned squared
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        x = west + (col as f64 + 0.5) * grid_res;
                        y = north - (row as f64 + 0.5) * grid_res;
                        let mut ret = frs.search(x, y);
                        if ret.len() < min_points {
                            ret = frs.knn_search(x, y, min_points);
                        }
                        if ret.len() >= min_points {
                            sum_weights = 0.0;
                            val = 0.0;
                            for j in 0..ret.len() {
                                zn = ret[j].0;
                                dist = ret[j].1 as f64;
                                if dist > 0.0 {
                                    val += zn / dist.powf(weight);
                                    sum_weights += 1.0 / dist.powf(weight);
                                } else {
                                    data[col as usize] = zn;
                                    sum_weights = 0.0;
                                    break;
                                }
                            }
                            if sum_weights > 0.0 {
                                data[col as usize] = val / sum_weights;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
                // if radius > 0f64 {
                //     for row in (0..rows).filter(|r| r % num_procs == tid) {
                //         let mut data = vec![nodata; columns as usize];
                //         for col in 0..columns {
                //             x = west + col as f64 * grid_res + 0.5;
                //             y = north - row as f64 * grid_res - 0.5;
                //             let ret = kdtree.within(&[x, y], radius, &squared_euclidean).unwrap();
                //             if ret.len() >= min_points {
                //                 sum_weights = 0.0;
                //                 val = 0.0;
                //                 for j in 0..ret.len() {
                //                     zn = *ret[j].1;
                //                     dist = ret[j].0;
                //                     if dist > 0.0 {
                //                         val += zn / (dist * dist.powf(diff_weight));
                //                         sum_weights += 1.0 / (dist * dist.powf(diff_weight));
                //                     } else {
                //                         data[col as usize] = zn;
                //                         sum_weights = 0.0;
                //                         break;
                //                     }
                //                 }
                //                 if sum_weights > 0.0 {
                //                     data[col as usize] = val / sum_weights;
                //                 }
                //             }
                //         }
                //         tx.send((row, data)).unwrap();
                //     }
                // } else {
                //     for row in (0..rows).filter(|r| r % num_procs == tid) {
                //         let mut data = vec![nodata; columns as usize];
                //         for col in 0..columns {
                //             x = west + col as f64 * grid_res + 0.5;
                //             y = north - row as f64 * grid_res - 0.5;
                //             let ret = kdtree.nearest(&[x, y], min_points, &squared_euclidean).unwrap();
                //             sum_weights = 0.0;
                //             val = 0.0;
                //             for j in 0..ret.len() {
                //                 zn = *ret[j].1;
                //                 dist = ret[j].0;
                //                 if dist < max_dist {
                //                     if dist > 0.0 {
                //                         val += zn / (dist * dist.powf(diff_weight));
                //                         sum_weights += 1.0 / (dist * dist.powf(diff_weight));
                //                     } else {
                //                         data[col as usize] = zn;
                //                         sum_weights = 0.0;
                //                         break;
                //                     }
                //                 } else {
                //                     // There are fewer than the required number of neighbouring
                //                     // points. Assign the output nodata.
                //                     sum_weights = 0.0;
                //                     break;
                //                 }
                //             }
                //             if sum_weights > 0.0 {
                //                 data[col as usize] = val / sum_weights;
                //             }
                //         }
                //         tx.send((row, data)).unwrap();
                //     }
                // }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
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
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
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
