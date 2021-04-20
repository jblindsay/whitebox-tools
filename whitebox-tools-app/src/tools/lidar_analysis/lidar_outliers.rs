/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/02/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_lidar::*;
use whitebox_common::structures::{DistanceMetric, FixedRadiusSearch2D, Point3D};
use crate::tools::*;
use num_cpus;
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool will filter out points from a LiDAR point cloud if the absolute elevation
/// difference between a point and the averge elevation of its neighbourhood, calculated
/// without the point, exceeds a threshold (elev_diff).
pub struct LidarRemoveOutliers {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarRemoveOutliers {
    pub fn new() -> LidarRemoveOutliers {
        // public constructor
        let name = "LidarRemoveOutliers".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Removes outliers (high and low points) in a LiDAR point cloud.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Search Radius".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Max. Elevation Difference".to_owned(),
            flags: vec!["--elev_diff".to_owned()],
            description: "Max. elevation difference.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("50.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use difference from median elevation?".to_owned(),
            flags: vec!["--use_median".to_owned()],
            description: "Optional flag indicating whether to use the difference from median elevation rather than mean."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Classify Points".to_owned(),
            flags: vec!["--classify".to_owned()],
            description: "Classify points as ground (2) or off-ground (1).".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0 --elev_diff=25.0 --use_median", short_exe, name).replace("*", &sep);

        LidarRemoveOutliers {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarRemoveOutliers {
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
        let mut search_radius = 2f64;
        let mut elev_diff = 50f64;
        let mut use_median = false;
        let mut filter = true;

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
            } else if flag_val == "-elev_diff" {
                elev_diff = if keyval {
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
            } else if flag_val == "-use_median" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_median = true;
                }
            } else if flag_val == "-classify" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    filter = false;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let start = Instant::now();

        if verbose {
            println!("Performing analysis...");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch2D<f64> =
            FixedRadiusSearch2D::new(search_radius, DistanceMetric::SquaredEuclidean);
        let mut pd: PointData;
        let mut p: Point3D;
        for i in 0..n_points {
            pd = input.get_point_info(i);
            p = input.get_transformed_coords(i);
            if !pd.is_classified_noise() && !pd.withheld() {
                frs.insert(p.x, p.y, p.z);
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Adding points to search tree: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let frs = Arc::new(frs); // wrap FRS in an Arc
        let input = Arc::new(input); // wrap input in an Arc
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        if !use_median {
            for tid in 0..num_procs {
                let frs = frs.clone();
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut avg_z: f64;
                    let mut n: f64;
                    let mut p: Point3D;
                    for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid)
                    {
                        p = input.get_transformed_coords(point_num);
                        let ret = frs.search(p.x, p.y);
                        avg_z = 0f64;
                        n = 0f64;
                        for j in 0..ret.len() {
                            if ret[j].1 != 0f64 {
                                avg_z += ret[j].0;
                                n += 1f64;
                            }
                        }
                        if n > 0f64 {
                            tx.send((point_num, p.z - avg_z / n)).unwrap();
                        } else {
                            tx.send((point_num, p.z)).unwrap();
                        }
                    }
                });
            }
        } else {
            for tid in 0..num_procs {
                let frs = frs.clone();
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut n: usize;
                    let mut median: f64;
                    let mut p: Point3D;
                    for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid)
                    {
                        p = input.get_transformed_coords(point_num);
                        let ret = frs.search(p.x, p.y);
                        n = 0;
                        let mut z_values: Vec<f64> = Vec::with_capacity(ret.len());
                        for j in 0..ret.len() {
                            if ret[j].1 != 0f64 {
                                z_values.push(ret[j].0);
                                n += 1;
                            }
                        }
                        if n > 3 {
                            z_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
                            if n % 2 != 0 {
                                // odd num neighbours
                                median = z_values[n / 2];
                            } else {
                                // even num neighbours
                                median = (z_values[n / 2 - 1] + z_values[n / 2]) / 2f64;
                            }
                            tx.send((point_num, p.z - median)).unwrap();
                        } else if n == 2 {
                            median = (z_values[0] + z_values[1]) / 2f64;
                            tx.send((point_num, p.z - median)).unwrap();
                        } else {
                            // n == 0 or n == 1 {
                            tx.send((point_num, p.z)).unwrap();
                        }
                    }
                });
            }
        }

        let mut residuals = vec![0f64; n_points];
        for i in 0..n_points {
            let data = rx.recv().expect("Error receiving data from thread.");
            residuals[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();
        let mut num_points_filtered = 0;
        if filter {
            for i in 0..n_points {
                pd = input.get_point_info(i);
                if residuals[i].abs() < elev_diff && !pd.is_classified_noise() {
                    output.add_point_record(input.get_record(i));
                } else {
                    num_points_filtered += 1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // classify
            let mut p: PointData;
            for point_num in 0..n_points {
                p = input.get_point_info(point_num);
                let class_val = match residuals[point_num] {
                    d if d < -elev_diff => 7,
                    d if d > elev_diff => 18,
                    _ => p.classification(),
                };
                let pr = input.get_record(point_num);
                let pr2: LidarPointRecord;
                match pr {
                    LidarPointRecord::PointRecord0 { mut point_data } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord0 {
                            point_data: point_data,
                        };
                    }
                    LidarPointRecord::PointRecord1 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord1 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord2 {
                        mut point_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord10 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                }
                output.add_point_record(pr2);
                if verbose {
                    progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            num_points_filtered = 1; // so it passes the saving
        }

        if num_points_filtered == 0 {
            println!("Warning: No outlier points were filtered from the point cloud.");
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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
