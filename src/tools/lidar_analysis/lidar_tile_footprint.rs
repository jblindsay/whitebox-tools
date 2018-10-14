/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 31/08/2018
Last Modified: 12/10/2018
License: MIT
*/

use algorithms::convex_hull;
use lidar::*;
use num_cpus;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use structures::Point2D;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// Creates a vector polygon of the convex hull of a LiDAR point cloud. When the input/output parameters
/// are not specified, the tool works with all LAS files contained within the working directory.
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
        ).replace("*", &sep);

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

        // read the arguments
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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let start = Instant::now();

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        let mut inputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            match fs::read_dir(working_directory) {
                Err(why) => println!("! {:?}", why.kind()),
                Ok(paths) => for path in paths {
                    let s = format!("{:?}", path.unwrap().path());
                    if s.replace("\"", "").to_lowercase().ends_with(".las") {
                        inputs.push(format!("{:?}", s.replace("\"", "")));
                    }
                },
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

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let wkt = Arc::new(Mutex::new(String::new()));
        let inputs = Arc::new(inputs);
        let num_procs = num_cpus::get() as isize;
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
                    tile = match tile_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to interpolate
                    };

                    let input_file = inputs[tile].replace("\"", "").clone();

                    if verbose && num_tiles == 1 {
                        println!("Reading input LAS file...");
                    }

                    let path = path::Path::new(&input_file);
                    let filenm = path.file_stem().unwrap();
                    let short_filename = filenm.to_str().unwrap().to_string();
                    if verbose && num_tiles > 1 {
                        println!("Processing {}", short_filename);
                    } else if verbose {
                        println!("Performing analysis...");
                    }
                    match LasFile::new(&input_file, "r") {
                        Ok(mut input) => {
                            let n_points = input.header.number_of_points as usize;

                            // read the points into a Vec<Point2D>
                            let mut points: Vec<Point2D> = Vec::with_capacity(n_points);
                            for i in 0..n_points {
                                let p: PointData = input.get_point_info(i);
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
                            tx.send((hull_points, short_filename, n_points)).unwrap();
                        }
                        Err(err) => {
                            tx.send((
                                vec![],
                                format!("Error reading file {}:\n{}", input_file, err),
                                0,
                            )).unwrap();
                        }
                    };
                }
            });
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // add the attributes
        let fid = AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8);
        let las_nm = AttributeField::new("LAS_NM", FieldDataType::Text, 25u8, 4u8);
        let num_pnts = AttributeField::new("NUM_PNTS", FieldDataType::Int, 9u8, 0u8);
        output.attributes.add_field(&fid);
        output.attributes.add_field(&las_nm);
        output.attributes.add_field(&num_pnts);

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
                            ],
                            false,
                        );
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

        let data = wkt.lock().unwrap();
        if *data != "" && *data != "Unknown EPSG Code" && output.projection.is_empty() {
            output.projection = data.to_string();
        }

        let elapsed_time = get_formatted_elapsed_time(start);

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
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
