/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/09/2018
Last Modified: 10/05/2019
License: MIT
*/

use self::na::Vector3;
use crate::algorithms::triangulate;
use crate::lidar::*;
use crate::na;
use crate::structures::Point2D;
use crate::tools::*;
use crate::vector::ShapefileGeometry;
use crate::vector::*;
use num_cpus;
use std::io::{Error, ErrorKind};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{env, f64, fs, path, thread};

/// This tool creates a vector triangular irregular network (TIN) for a set of LiDAR points (`--input`)
/// using a 2D [Delaunay triangulation](https://en.wikipedia.org/wiki/Delaunay_triangulation) algorithm.
/// LiDAR points may be excluded from the triangulation operation based on a number of criteria,
/// include the point return number (`--returns`), point classification value (`--exclude_cls`), or
/// a minimum (`--minz`) or maximum (`--maxz`) elevation.
///
/// For vector points, use the `ConstructVectorTIN` tool instead.
///
/// # See Also
/// `ConstructVectorTIN`
pub struct LidarConstructVectorTIN {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarConstructVectorTIN {
    pub fn new() -> LidarConstructVectorTIN {
        // public constructor
        let name = "LidarConstructVectorTIN".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Creates a vector triangular irregular network (TIN) fitted to LiDAR points."
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --returns=last --exclude_cls='3,4,5,6,7,18'", short_exe, name).replace("*", &sep);

        LidarConstructVectorTIN {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarConstructVectorTIN {
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
        let mut return_type = "all".to_string();
        let mut include_class_vals = vec![true; 256];
        let mut exclude_cls_str: String;
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
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-returns" {
                if keyval {
                    return_type = vec[1].to_string();
                } else {
                    return_type = args[i + 1].to_string();
                }
            } else if flag_val == "-exclude_cls" {
                if keyval {
                    exclude_cls_str = vec[1].to_string();
                } else {
                    exclude_cls_str = args[i + 1].to_string();
                }
                let mut cmd = exclude_cls_str.split(",");
                let mut vec = cmd.collect::<Vec<&str>>();
                if vec.len() == 1 {
                    cmd = exclude_cls_str.split(";");
                    vec = cmd.collect::<Vec<&str>>();
                }
                for value in vec {
                    if !value.trim().is_empty() {
                        let c = value.trim().parse::<usize>().unwrap();
                        include_class_vals[c] = false;
                    }
                }
            } else if flag_val == "-minz" {
                if keyval {
                    min_z = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    min_z = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-maxz" {
                if keyval {
                    max_z = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    max_z = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
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

        let mut inputs = vec![];
        let mut outputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            // match fs::read_dir(working_directory) {
            //     Err(why) => println!("! {:?}", why.kind()),
            //     Ok(paths) => {
            //         for path in paths {
            //             let s = format!("{:?}", path.unwrap().path());
            //             if s.replace("\"", "").to_lowercase().ends_with(".las") {
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 outputs.push(
            //                     inputs[inputs.len() - 1]
            //                         .replace(".las", ".tif")
            //                         .replace(".LAS", ".tif"),
            //                 )
            //             } else if s.replace("\"", "").to_lowercase().ends_with(".zip") {
            //                 // assumes the zip file contains LAS data.
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 outputs.push(
            //                     inputs[inputs.len() - 1]
            //                         .replace(".zip", ".tif")
            //                         .replace(".ZIP", ".tif"),
            //                 )
            //             }
            //         }
            //     }
            // }
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

        if verbose {
            println!("Performing interpolation...");
        }

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let inputs = Arc::new(inputs);
        let outputs = Arc::new(outputs);
        let num_procs2 = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for _ in 0..num_procs2 {
            let inputs = inputs.clone();
            let outputs = outputs.clone();
            let tile_list = tile_list.clone();
            // copy over the string parameters
            let include_class_vals = include_class_vals.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut tile = 0;
                while tile < num_tiles {
                    // Get the next tile up for interpolation
                    tile = match tile_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to interpolate
                    };

                    let input_file = inputs[tile].replace("\"", "").clone();
                    let output_file = outputs[tile].replace("\"", "").clone();

                    let mut points = vec![];
                    let mut z_values = vec![];

                    if verbose && inputs.len() == 1 {
                        println!("Reading input LAS file...");
                    }

                    let mut progress: usize;
                    let mut old_progress: usize = 1;

                    let mut input = match LasFile::new(&input_file, "r") {
                        Ok(lf) => lf,
                        Err(err) => panic!(
                            "Error reading file {}: {}",
                            input_file.replace("\"", ""),
                            err
                        ),
                    };

                    let n_points = input.header.number_of_points as usize;
                    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

                    for i in 0..n_points {
                        let p: PointData = input[i];
                        if !p.withheld() {
                            if all_returns
                                || (p.is_late_return() & late_returns)
                                || (p.is_early_return() & early_returns)
                            {
                                if include_class_vals[p.classification() as usize] {
                                    if p.z >= min_z && p.z <= max_z {
                                        points.push(Point2D { x: p.x, y: p.y });
                                        z_values.push(p.z);
                                    }
                                }
                            }
                        }
                        if verbose && inputs.len() == 1 {
                            progress = (100.0_f64 * i as f64 / num_points) as usize;
                            if progress != old_progress {
                                println!("Reading points: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    let azimuth = (315f64 - 90f64).to_radians();
                    let altitude = 30f64.to_radians();
                    let sin_theta = altitude.sin();
                    let cos_theta = altitude.cos();

                    // create output file
                    let mut output = match Shapefile::new(&output_file, ShapeType::Polygon) {
                        Ok(output) => output,
                        Err(e) => panic!("Error creating output file:\n{:?}", e), // TODO: fix this panic.
                    };

                    // set the projection information
                    output.projection = input.get_wkt();

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        5u8,
                        0u8,
                    ));

                    output.attributes.add_field(&AttributeField::new(
                        "CENTROID_Z",
                        FieldDataType::Real,
                        10u8,
                        4u8,
                    ));

                    output.attributes.add_field(&AttributeField::new(
                        "HILLSHADE",
                        FieldDataType::Int,
                        4u8,
                        0u8,
                    ));

                    // do the triangulation
                    if num_tiles == 1 && verbose {
                        println!("Performing triangulation...");
                    }
                    let result = triangulate(&points).expect("No triangulation exists.");
                    let (mut p1, mut p2, mut p3): (usize, usize, usize);
                    let (mut fx, mut fy): (f64, f64);
                    let (mut tan_slope, mut aspect): (f64, f64);
                    let (mut term1, mut term2, mut term3): (f64, f64, f64);
                    let mut hillshade: f64;
                    let mut rec_num = 1i32;
                    for i in (0..result.triangles.len()).step_by(3) {
                        // the points in triangles are counter clockwise ordered and we need clockwise
                        p1 = result.triangles[i + 2];
                        p2 = result.triangles[i + 1];
                        p3 = result.triangles[i];

                        let mut tri_points: Vec<Point2D> = Vec::with_capacity(4);
                        tri_points.push(points[p1].clone());
                        tri_points.push(points[p2].clone());
                        tri_points.push(points[p3].clone());
                        tri_points.push(points[p1].clone());

                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                        sfg.add_part(&tri_points);
                        output.add_record(sfg);

                        // calculate the hillshade value
                        let a = Vector3::new(tri_points[0].x, tri_points[0].y, z_values[p1]);
                        let b = Vector3::new(tri_points[1].x, tri_points[1].y, z_values[p2]);
                        let c = Vector3::new(tri_points[2].x, tri_points[2].y, z_values[p3]);
                        let norm = (b - a).cross(&(c - a)); //).normalize();
                        let centroid = (a + b + c) / 3f64;
                        // k = -(tri_points[0].x * norm.x + tri_points[0].y * norm.y + norm.z * z_values[p1]);
                        // centroid_z = -(norm.x * centroid.x + norm.y * centroid.y + k) / norm.z;

                        hillshade = 0f64;
                        if norm.z != 0f64 {
                            fx = -norm.x / norm.z;
                            fy = -norm.y / norm.z;
                            if fx != 0f64 {
                                tan_slope = (fx * fx + fy * fy).sqrt();
                                aspect = (180f64 - ((fy / fx).atan()).to_degrees()
                                    + 90f64 * (fx / (fx).abs()))
                                .to_radians();
                                term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                                term2 = sin_theta / tan_slope;
                                term3 = cos_theta * (azimuth - aspect).sin();
                                hillshade = term1 * (term2 - term3);
                            } else {
                                hillshade = 0.5;
                            }
                            hillshade = hillshade * 1024f64;
                            if hillshade < 0f64 {
                                hillshade = 0f64;
                            }
                        }

                        output.attributes.add_record(
                            vec![
                                FieldData::Int(rec_num),
                                FieldData::Real(centroid.z),
                                FieldData::Int(hillshade as i32),
                            ],
                            false,
                        );

                        rec_num += 1i32;

                        if verbose && num_tiles == 1 {
                            progress = (100.0_f64 * i as f64 / (result.triangles.len() - 1) as f64)
                                as usize;
                            if progress != old_progress {
                                println!("Creating polygons: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    if verbose && inputs.len() == 1 {
                        println!("Saving data...")
                    };

                    let _ = match output.write() {
                        Ok(_) => {
                            if verbose {
                                println!("Output file written")
                            }
                        }
                        Err(e) => panic!("Error reading file {}:\n{:?}", input_file, e),
                    };

                    tx.send(tile).unwrap();
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..inputs.len() {
            let tile_completed = rx.recv().unwrap();
            if verbose {
                println!(
                    "Finished TINing {} ({} of {})",
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
