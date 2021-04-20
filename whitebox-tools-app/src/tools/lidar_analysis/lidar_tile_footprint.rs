/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 31/08/2018
Last Modified: 19/05/2020
License: MIT
*/

use whitebox_common::algorithms::convex_hull;
use whitebox_lidar::*;
use whitebox_common::structures::Point2D;
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use num_cpus;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool can be used to create a vector polygon of the bounding box or convex hull of a LiDAR point cloud (i.e. LAS file).
/// If the user specified an input file (`--input`) and output file (`--output`), the tool will calculate the footprint,
/// containing all of the data points, and output this feature to a vector polygon file. If the `input` and
/// `output` parameters are left unspecified, the tool will calculate the footprint of every LAS file contained within the
/// working directory and output these features to a single vector polygon file. If this is the desired mode of
/// operation, it is important to specify the working directory (`--wd`) containing the group of LAS files; do not
/// specify the optional `--input` and `--output` parameters in this case. Each polygon in the output vector will contain
/// a `LAS_NM` field, specifying the source LAS file name, a `NUM_PNTS` field, containing the number of points
/// within the source file, and Z_MIN and Z_MAX fields, containing the minimum and maximum elevations. This output can
/// therefore be useful to create an index map of a large tiled LiDAR dataset.
///
/// By default, this tool identifies the axis-aligned minimum rectangular hull, or bounding box, containing the points
/// in each of the input tiles. If the user specifies the `--hull` flag, the tool will identify the
/// [minimum convex hull](https://en.wikipedia.org/wiki/Convex_hull) instead of the bounding box. This option is considerably
/// more computationally intensive and will be a far longer running operation if many tiles are specified as inputs.
///
/// **A note on LAZ file inputs:** While WhiteboxTools does not currently support the reading and writing of the compressed
/// LiDAR format `LAZ`, it is able to read `LAZ` file headers. This tool, when run in in the bounding box mode (rather than
/// the convex hull mode), is able to take `LAZ` input files.
///
///  `LidarTile`, `LayerFootprint`, `MinimumBoundingBox`, `MinimumConvexHull`
pub struct LidarTileFootprint {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarTileFootprint {
    pub fn new() -> LidarTileFootprint {
        // public constructor
        let name = "LidarTileFootprint".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates a vector polygon of the convex hull of a LiDAR point cloud. When the input/output parameters are not specified, the tool works with all LAS files contained within the working directory.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Create Convex Hull Around Points".to_owned(),
            flags: vec!["--hull".to_owned()],
            description: "Identify the convex hull around points.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.shp",
            short_exe, name
        )
        .replace("*", &sep);

        LidarTileFootprint {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarTileFootprint {
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
        let mut is_convex_hull = false;

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
            } else if flag_val == "-hull" || flag_val == "-convex_hull" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    is_convex_hull = true;
                }
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let start = Instant::now();

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        let mut inputs = vec![];
        let mut contains_laz = false;
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
            //             } else if s.replace("\"", "").to_lowercase().ends_with(".laz") {
            //                 inputs.push(format!("{:?}", s.replace("\"", "")));
            //                 contains_laz = true;
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
                    if s.to_lowercase().ends_with(".las") || s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                    } else if s.to_lowercase().ends_with(".laz") {
                        inputs.push(s);
                        contains_laz = true;
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
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if contains_laz && is_convex_hull {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: This tool only works with the compressed `LAZ` file format when
                the footprint is a bounding box and not a convex hull (`--hull`).",
            ));
        }

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let wkt = Arc::new(Mutex::new(String::new()));
        let inputs = Arc::new(inputs);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = channel();
        for _ in 0..num_procs {
            let inputs = inputs.clone();
            let tile_list = tile_list.clone();
            let wkt = wkt.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut tile = 0;
                while tile < num_tiles {
                    // Get the next tile up for processing
                    {
                        tile = match tile_list.lock().unwrap().next() {
                            Some(val) => val,
                            None => break, // There are no more tiles to interpolate
                        };
                    }

                    let input_file = inputs[tile].replace("\"", "").clone();

                    if verbose && num_tiles == 1 {
                        println!("Reading input LAS file...");
                    }

                    let path = path::Path::new(&input_file);
                    let filenm = path.file_stem().unwrap();
                    let short_filename = filenm.to_str().unwrap().to_string();
                    if verbose && num_tiles > 1 && num_tiles < 500 {
                        println!("Processing {}", short_filename);
                    } else if verbose && num_tiles == 1 && num_tiles < 500 {
                        println!("Performing analysis...");
                    }
                    if is_convex_hull {
                        match LasFile::new(&input_file, "r") {
                            Ok(mut input) => {
                                let n_points = input.header.get_number_of_points() as usize;

                                if n_points == 0usize {
                                    println!(
                                        "Warning {} does not contain any points.",
                                        short_filename
                                    );
                                }

                                // read the points into a Vec<Point2D>
                                let mut points: Vec<Point2D> = Vec::with_capacity(n_points);
                                for i in 0..n_points {
                                    // let p: PointData = input.get_point_info(i);
                                    let p = input.get_transformed_coords(i);
                                    points.push(Point2D::new(p.x, p.y));
                                }

                                let mut hull_points = convex_hull(&mut points);

                                // convex_hull returns points in a counter-clockwise order but we need it to be clockwise for a shapefile poly.
                                hull_points.reverse();
                                // now add a last point same as the first.
                                let p = hull_points[0];
                                hull_points.push(p);

                                if tile == 0 {
                                    let mut data = wkt.lock().unwrap();
                                    *data = input.get_wkt();
                                }
                                // send the data to the main thread to be output
                                tx.send((
                                    hull_points,
                                    short_filename,
                                    n_points,
                                    input.header.min_z,
                                    input.header.max_z,
                                    input.get_wkt(),
                                ))
                                .unwrap();
                            }
                            Err(err) => {
                                tx.send((
                                    vec![],
                                    format!("Error reading file {}:\n{}", input_file, err),
                                    0,
                                    0f64,
                                    0f64,
                                    "".to_string(),
                                ))
                                .unwrap();
                            }
                        };
                    } else {
                        match LasHeader::read_las_header(&input_file) {
                            Ok(header) => {
                                let mut bounding_points: Vec<Point2D> = Vec::with_capacity(5);
                                bounding_points.push(Point2D::new(header.min_x, header.max_y));
                                bounding_points.push(Point2D::new(header.max_x, header.max_y));
                                bounding_points.push(Point2D::new(header.max_x, header.min_y));
                                bounding_points.push(Point2D::new(header.min_x, header.min_y));
                                bounding_points.push(Point2D::new(header.min_x, header.max_y));

                                if header.get_number_of_points() == 0u64 {
                                    println!(
                                        "Warning {} does not contain any points.",
                                        short_filename
                                    );
                                }

                                tx.send((
                                    bounding_points,
                                    short_filename,
                                    header.get_number_of_points() as usize,
                                    header.min_z,
                                    header.max_z,
                                    "".to_string(),
                                ))
                                .unwrap();
                            }
                            Err(err) => {
                                tx.send((
                                    vec![],
                                    format!("Error reading file {}:\n{}", input_file, err),
                                    0,
                                    0f64,
                                    0f64,
                                    "".to_string(),
                                ))
                                .unwrap();
                            }
                        }
                    }
                }
            });
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "LAS_NM",
            FieldDataType::Text,
            25u8,
            4u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "NUM_PNTS",
            FieldDataType::Int,
            9u8,
            0u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "Z_MIN",
            FieldDataType::Real,
            11u8,
            5u8,
        ));
        output.attributes.add_field(&AttributeField::new(
            "Z_MAX",
            FieldDataType::Real,
            11u8,
            5u8,
        ));

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..num_tiles {
            match rx.recv() {
                Ok(data) => {
                    if data.0.len() > 0 {
                        let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                        sfg.add_part(&data.0);
                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(tile as i32 + 1i32),
                                FieldData::Text(data.1),
                                FieldData::Int(data.2 as i32),
                                FieldData::Real(data.3 as f64),
                                FieldData::Real(data.4 as f64),
                            ],
                            false,
                        );
                        if !data.5.is_empty() && output.projection.is_empty() {
                            output.projection = data.5.clone();
                        }
                    } else {
                        // there was an error, likely reading a LAS file.
                        println!("{}", data.1);
                    }
                }
                Err(val) => println!("Error: {:?}", val),
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (num_tiles - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress ({} of {}): {}%", (tile + 1), num_tiles, progress);
                    old_progress = progress;
                }
            }
        }

        if output.projection.is_empty() {
            let input_file = inputs[0].replace("\"", "").clone();
            let mut input = LasFile::new(&input_file, "rh")?;
            // let mut input =  LasHeader::read_las_header(&input_file)?;
            output.projection = input.get_wkt().clone();
        }

        let data = wkt.lock().unwrap();
        if *data != "" && *data != "Unknown EPSG Code" && output.projection.is_empty() {
            output.projection = data.to_string();
        }

        let elapsed_time = get_formatted_elapsed_time(start);

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
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
