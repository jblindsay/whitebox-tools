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

use whitebox_lidar::*;
use crate::tools::*;
use whitebox_common::structures::Point3D;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool normalizes an input LiDAR point cloud (`--input`) such that point z-values in the output LAS file
/// (`--output`) are converted from elevations to heights above the ground, specifically the height above the
/// nearest ground-classified point. The input LAS file must have ground-classified points, otherwise the tool
/// will return an error. The `LidarTophatTransform` tool can be used to perform the normalization if a ground
/// classification is lacking.
///
/// # See Also
/// `LidarTophatTransform`
pub struct HeightAboveGround {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HeightAboveGround {
    pub fn new() -> HeightAboveGround {
        // public constructor
        let name = "HeightAboveGround".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Normalizes a LiDAR point cloud, providing the height above the nearest ground-classified point."
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif",
            short_exe, name
        )
        .replace("*", &sep);

        HeightAboveGround {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HeightAboveGround {
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

        let sep = path::MAIN_SEPARATOR;
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let start = Instant::now();

        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        const DIMENSIONS: usize = 2;
        const CAPACITY_PER_NODE: usize = 64;
        let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
        let mut pd1: PointData;
        let mut p1: Point3D;
        for i in 0..n_points {
            pd1 = input[i];
            p1 = input.get_transformed_coords(i);
            if !pd1.withheld() && pd1.classification() == 2u8 {
                tree.add([p1.x, p1.y], i).unwrap();
            }

            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Reading points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if tree.size() == 0 {
            panic!(
                "Error: None of the points in the input LAS file have been classified as 'ground'"
            );
        }

        let num_solved_pts = Arc::new(Mutex::new(0usize));
        let tree = Arc::new(tree);
        let input = Arc::new(input); // wrap input in an Arc
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let num_solved_pts = num_solved_pts.clone();
            let tree = tree.clone();
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut pd1: PointData;
                // let mut p2: PointData;
                let mut p1: Point3D;
                let mut p2: Point3D;
                let mut z: f64;
                let mut point_num: usize;
                let mut residuals = vec![];
                let mut old_progress: i32;
                let mut progress: i32;
                for i in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    pd1 = input[i];
                    p1 = input.get_transformed_coords(i);
                    z = if pd1.classification() != 2u8 {
                        let ret = tree.nearest(&[p1.x, p1.y], 1, &squared_euclidean).unwrap();
                        point_num = *(ret[0].1);
                        // p2 = input[point_num];
                        p2 = input.get_transformed_coords(point_num);
                        p1.z - p2.z
                    } else {
                        0f64
                    };
                    residuals.push((i, z));

                    if verbose {
                        let mut num_solved_pts =
                            num_solved_pts.lock().expect("Error unlocking mutex");
                        old_progress = (100.0_f64 * *num_solved_pts as f64 / num_points) as i32;
                        *num_solved_pts += 1;
                        progress = (100.0_f64 * *num_solved_pts as f64 / num_points) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                        }
                    }
                }
                tx.send(residuals).unwrap();
            });
        }

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        for n in 0..num_procs {
            let residuals = rx.recv().expect("Error receiving data from thread.");
            for (i, z) in residuals {
                let pr = input.get_record(i);
                let pr2: LidarPointRecord;
                match pr {
                    LidarPointRecord::PointRecord0 { mut point_data } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord0 {
                            point_data: point_data,
                        };
                    }
                    LidarPointRecord::PointRecord1 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord1 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord2 {
                        mut point_data,
                        colour_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord2 {
                            point_data: point_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord3 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord3 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord4 {
                        mut point_data,
                        gps_data,
                        wave_packet,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord4 {
                            point_data: point_data,
                            gps_data: gps_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord5 {
                        mut point_data,
                        gps_data,
                        colour_data,
                        wave_packet,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord5 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord6 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord6 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord7 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord7 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord8 {
                        mut point_data,
                        gps_data,
                        colour_data,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord8 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                        };
                    }
                    LidarPointRecord::PointRecord9 {
                        mut point_data,
                        gps_data,
                        wave_packet,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord9 {
                            point_data: point_data,
                            gps_data: gps_data,
                            wave_packet: wave_packet,
                        };
                    }
                    LidarPointRecord::PointRecord10 {
                        mut point_data,
                        gps_data,
                        colour_data,
                        wave_packet,
                    } => {
                        point_data.z = ((z - input.header.z_offset) / input.header.z_scale_factor) as i32;
                        pr2 = LidarPointRecord::PointRecord10 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                }
                output.add_point_record(pr2);
            }
            if verbose {
                progress = (100.0_f64 * (n + 1) as f64 / num_procs as f64) as i32;
                if progress != old_progress {
                    println!("Creating output: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Complete!")
                }
            }
            Err(e) => println!("error while writing: {:?}", e),
        };

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
