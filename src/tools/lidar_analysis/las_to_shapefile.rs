/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/10/2018
Last Modified: 10/05/2019
License: MIT
*/

use crate::lidar::*;
use crate::tools::*;
use crate::vector::*;
use num_cpus;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool converts one or more LAS files into a POINT vector. When the input parameter is
/// not specified, the tool grids all LAS files contained within the working directory.
/// The attribute table of the output Shapefile will contain fields for the z-value,
/// intensity, point class, return number, and number of return.
///
/// This tool can be used in place of the `LasToMultipointShapefile` tool when the
/// number of points are relatively low and when the desire is to represent more than
/// simply the x,y,z position of points. Notice however that because each point in
/// the input LAS file will be represented as a separate record in the output
/// Shapefile, the output file will be many time larger than the equivalent output of
/// the `LasToMultipointShapefile` tool. There is also a practical limit on the
/// total number of records that can be held in a single Shapefile and large LAS
/// files approach this limit. In these cases, the `LasToMultipointShapefile` tool
/// should be preferred instead.
///
/// # See Also
/// `LasToMultipointShapefile`
pub struct LasToShapefile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LasToShapefile {
    pub fn new() -> LasToShapefile {
        // public constructor
        let name = "LasToShapefile".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Converts one or more LAS files into a vector Shapefile of POINT ShapeType."
                .to_string();

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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.las",
            short_exe, name
        )
        .replace("*", &sep);

        LasToShapefile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LasToShapefile {
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
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
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
                    if s.to_lowercase().ends_with(".las") {
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
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = channel();
        for _ in 0..num_procs {
            let inputs = inputs.clone();
            let tile_list = tile_list.clone();
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
                    let output_file = input_file
                        .clone()
                        .replace(".las", ".shp")
                        .replace(".LAS", ".shp");

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
                    let ret_val = match LasFile::new(&input_file, "r") {
                        Ok(mut input) => {
                            // create the output file
                            let mut output = match Shapefile::new(&output_file, ShapeType::Point) {
                                Ok(output) => output,
                                Err(e) => panic!("Error creating output file:\n{:?}", e), // TODO: fix this panic.
                            };
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

                            output.attributes.add_field(&AttributeField::new(
                                "INTENSITY",
                                FieldDataType::Int,
                                7u8,
                                0u8,
                            ));

                            output.attributes.add_field(&AttributeField::new(
                                "CLASS",
                                FieldDataType::Int,
                                5u8,
                                0u8,
                            ));

                            output.attributes.add_field(&AttributeField::new(
                                "RTN_NUM",
                                FieldDataType::Int,
                                3u8,
                                0u8,
                            ));

                            output.attributes.add_field(&AttributeField::new(
                                "NUM_RTNS",
                                FieldDataType::Int,
                                3u8,
                                0u8,
                            ));

                            let n_points = input.header.number_of_points as usize;
                            let mut progress: usize;
                            let mut old_progress: usize = 1;
                            // read the points
                            for i in 0..n_points {
                                let p: PointData = input.get_point_info(i);
                                output.add_point_record(p.x, p.y);
                                output.attributes.add_record(
                                    vec![
                                        FieldData::Int(i as i32 + 1i32),
                                        FieldData::Real(p.z),
                                        FieldData::Int(p.intensity as i32),
                                        FieldData::Int(p.classification as i32),
                                        FieldData::Int(p.return_number() as i32),
                                        FieldData::Int(p.number_of_returns() as i32),
                                    ],
                                    false,
                                );

                                if verbose && num_tiles == 1 {
                                    progress =
                                        (100.0_f64 * i as f64 / (n_points - 1) as f64) as usize;
                                    if progress != old_progress {
                                        println!("Progress: {}%", progress);
                                        old_progress = progress;
                                    }
                                }
                            }

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
