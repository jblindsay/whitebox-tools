/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/09/2018
Last Modified: 19/05/2020
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use whitebox_common::structures::Point2D;
use num_cpus;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

/// Converts one or more LAS files into MultipointZ vector Shapefiles. When the input parameter is
/// not specified, the tool grids all LAS files contained within the working directory.
///
/// This tool can be used in place of the `LasToShapefile` tool when the number of points are
/// relatively high and when the desire is to represent the x,y,z position of points only. The z
/// values of LAS points will be stored in the z-array of the output Shapefile. Notice that because
/// the output file stores each point in a single multi-point record, this Shapefile representation,
/// while unable to represent individual point classes, return numbers, etc, is an efficient means
/// of converting LAS point positional information.
///
/// # See Also
/// `LasToShapefile`
pub struct LasToMultipointShapefile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LasToMultipointShapefile {
    pub fn new() -> LasToMultipointShapefile {
        // public constructor
        let name = "LasToMultipointShapefile".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Converts one or more LAS files into MultipointZ vector Shapefiles. When the input parameter is not specified, the tool grids all LAS files contained within the working directory.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.las",
            short_exe, name
        )
        .replace("*", &sep);

        LasToMultipointShapefile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LasToMultipointShapefile {
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

        // read the arguments
        if args.len() == 0 && working_directory.is_empty() {
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        let mut inputs = vec![];
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
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
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

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
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
                    let output_file = input_file
                        .clone()
                        .replace(".las", ".shp")
                        .replace(".LAS", ".shp")
                        .replace(".zlidar", ".shp")
                        .replace(".ZLIDAR", ".shp");

                    if verbose && num_tiles == 1 {
                        println!("reading input LiDAR file...");
                    }

                    let path = path::Path::new(&input_file);
                    let filenm = path.file_stem().unwrap();
                    let short_filename = filenm.to_str().unwrap().to_string();
                    if verbose && num_tiles > 1 {
                        println!("Processing {}", short_filename);
                    } else if verbose {
                        println!("Performing analysis...");
                    }
                    let ret_val = match LasFile::new(&input_file, "r") {
                        Ok(mut input) => {
                            // create the output file
                            let mut output =
                                match Shapefile::new(&output_file, ShapeType::MultiPointZ) {
                                    Ok(output) => output,
                                    Err(e) => panic!("Error creating output file:\n{:?}", e), // TODO: fix this panic.
                                };
                            output.projection = input.get_wkt();

                            // add the attributes
                            let fid = AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8);
                            output.attributes.add_field(&fid);

                            let n_points = input.header.number_of_points as usize;

                            // read the points into a Vec<Point2D>
                            let mut points: Vec<Point2D> = Vec::with_capacity(n_points);
                            let mut m_values: Vec<f64> = Vec::with_capacity(n_points);
                            let mut z_values: Vec<f64> = Vec::with_capacity(n_points);
                            for i in 0..n_points {
                                let pd: PointData = input.get_point_info(i);
                                let p = input.get_transformed_coords(i);
                                points.push(Point2D::new(p.x, p.y));
                                m_values.push(pd.intensity as f64);
                                z_values.push(p.z);
                            }

                            let mut sfg = ShapefileGeometry::new(ShapeType::MultiPointZ);
                            sfg.add_partz(&points, &m_values, &z_values);
                            output.add_record(sfg);
                            output
                                .attributes
                                .add_record(vec![FieldData::Int(1i32)], false);

                            // output the file
                            let v = match output.write() {
                                Ok(_) => (true, String::new()),
                                Err(err) => (
                                    false,
                                    format!("Error reading file {}:\n{:?}", input_file, err),
                                ),
                            };

                            v
                        }
                        Err(err) => (
                            false,
                            format!("Error reading file {}:\n{:?}", input_file, err),
                        ),
                    };
                    // send the data to the main thread to be output
                    tx.send(ret_val).unwrap();
                }
            });
        }

        for tile in 0..num_tiles {
            match rx.recv() {
                Ok(data) => {
                    if !data.0 {
                        println!("{}", data.1);
                    }
                }
                Err(val) => println!("Error: {:?}", val),
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (num_tiles - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress ({} of {}): {}%", (tile + 1), num_tiles, progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
