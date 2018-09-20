/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/06/2017
Last Modified: 18/09/2018
License: MIT
*/

use lidar::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use structures::{DistanceMetric, FixedRadiusSearch2D};
use time;
use tools::*;

pub struct LidarGroundPointFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarGroundPointFilter {
    pub fn new() -> LidarGroundPointFilter {
        // public constructor
        let name = "LidarGroundPointFilter".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Identifies ground points within LiDAR dataset using a slope-based method.".to_string();

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
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Number of Neighbours".to_owned(),
            flags: vec!["--min_neighbours".to_owned()],
            description: "The minimum number of neighbouring points within search areas. If fewer points than this threshold are idenfied during the fixed-radius search, a subsequent kNN search is performed to identify the k number of neighbours.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Inter-point Slope Threshold".to_owned(),
            flags: vec!["--slope_threshold".to_owned()],
            description: "Maximum inter-point slope to be considered an off-terrain point."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("45.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Off-terrain Point Height Threshold".to_owned(),
            flags: vec!["--height_threshold".to_owned()],
            description: "Inter-point height difference to be considered an off-terrain point."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Classify Points".to_owned(),
            flags: vec!["--classify".to_owned()],
            description: "Classify points as ground (2) or off-ground (1).".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Perform initial ground slope normalization?".to_owned(),
            flags: vec!["--slope_norm".to_owned()],
            description: "Perform initial ground slope normalization?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0 --min_neighbours=10 --slope_threshold=30.0 --height_threshold=0.5 --classify --slope_norm", short_exe, name).replace("*", &sep);

        LidarGroundPointFilter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarGroundPointFilter {
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
        let mut search_radius: f64 = -1.0;
        let mut min_neighbours = 0usize;
        let mut height_threshold: f64 = 1.0;
        let mut slope_threshold: f64 = 15.0;
        let ground_class_value = 2u8;
        let otp_class_value = 1u8;
        let mut filter = true;
        let mut slope_norm = false;

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
            } else if flag_val == "-radius" {
                search_radius = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-min_neighbours" || flag_val == "-min_neighbors" {
                min_neighbours = if keyval {
                    vec[1].to_string().parse::<usize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<usize>().unwrap()
                };
            } else if flag_val == "-height_threshold" {
                height_threshold = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-slope_threshold" {
                slope_threshold = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-classify" {
                filter = false;
            } else if flag_val == "-slope_norm" {
                slope_norm = true;
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

        let start = time::now();

        if verbose {
            println!("Performing analysis...");
        }

        slope_threshold = slope_threshold.to_radians().tan();

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut residuals = vec![f64::MIN; n_points];
        let mut is_off_terrain = vec![false; n_points];

        let mut frs: FixedRadiusSearch2D<usize> =
            FixedRadiusSearch2D::new(search_radius, DistanceMetric::SquaredEuclidean);

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            if p.is_late_return() && !p.is_classified_noise() {
                frs.insert(p.x, p.y, i);
                if !slope_norm {
                    residuals[i] = p.z;
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let frs = Arc::new(frs); // wrap FRS in an Arc
        let num_procs = num_cpus::get();
        let input = Arc::new(input); // wrap input in an Arc

        if slope_norm {
            /////////////
            // Erosion //
            /////////////
            let mut neighbourhood_min = vec![f64::MAX; n_points];
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let frs = frs.clone();
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut index_n: usize;
                    let mut z_n: f64;
                    let mut min_z: f64;
                    let mut ret: Vec<(usize, f64)>;
                    for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid)
                    {
                        let p: PointData = input.get_point_info(point_num);
                        if p.is_late_return() && !p.is_classified_noise() {
                            ret = frs.search(p.x, p.y);
                            min_z = f64::MAX;
                            for j in 0..ret.len() {
                                index_n = ret[j].0;
                                z_n = input.get_point_info(index_n).z;
                                if z_n < min_z {
                                    min_z = z_n;
                                }
                            }
                            tx.send((point_num, min_z)).unwrap();
                        } else {
                            tx.send((point_num, f64::MAX)).unwrap();
                        }
                    }
                });
            }

            for i in 0..n_points {
                let data = rx.recv().unwrap();
                neighbourhood_min[data.0] = data.1;
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Erosion: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            //////////////
            // Dilation //
            //////////////
            let neighbourhood_min = Arc::new(neighbourhood_min); // wrap neighbourhood_min in an Arc
            for tid in 0..num_procs {
                let frs = frs.clone();
                let input = input.clone();
                let neighbourhood_min = neighbourhood_min.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut index_n: usize;
                    let mut z_n: f64;
                    let mut max_z: f64;
                    let mut ret: Vec<(usize, f64)>;
                    for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid)
                    {
                        let p: PointData = input.get_point_info(point_num);
                        if p.is_late_return() && !p.is_classified_noise() {
                            ret = frs.search(p.x, p.y);
                            max_z = f64::MIN;
                            for j in 0..ret.len() {
                                index_n = ret[j].0;
                                z_n = neighbourhood_min[index_n];
                                if z_n > max_z {
                                    max_z = z_n;
                                }
                            }
                            tx.send((point_num, max_z)).unwrap();
                        } else {
                            tx.send((point_num, f64::MIN)).unwrap();
                        }
                    }
                });
            }

            for i in 0..n_points {
                let data = rx.recv().unwrap();
                if data.1 != f64::MIN {
                    let z = input.get_point_info(data.0).z;
                    residuals[data.0] = z - data.1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Dilation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {

        }

        ////////////////////////
        // Slope-based filter //
        ////////////////////////
        let residuals = Arc::new(residuals);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            let input = input.clone();
            let residuals = residuals.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut index_n: usize;
                let mut max_slope: f64;
                let mut slope: f64;
                let mut dist: f64;
                let mut ret: Vec<(usize, f64)>;
                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let p: PointData = input.get_point_info(point_num);
                    if (!slope_norm || residuals[point_num] < height_threshold)
                        && p.is_late_return()
                        && !p.is_classified_noise()
                    {
                        ret = frs.search(p.x, p.y);
                        if ret.len() < min_neighbours {
                            ret = frs.knn_search(p.x, p.y, min_neighbours);
                        }
                        max_slope = f64::MIN;
                        for j in 0..ret.len() {
                            dist = ret[j].1;
                            if dist > 0f64 {
                                index_n = ret[j].0;
                                slope = (residuals[point_num] - residuals[index_n]) / dist.sqrt();
                                if slope > max_slope {
                                    max_slope = slope;
                                }
                            }
                        }
                        if max_slope > slope_threshold {
                            tx.send((point_num, true)).unwrap();
                        } else {
                            tx.send((point_num, false)).unwrap();
                        }
                    } else {
                        tx.send((point_num, true)).unwrap();
                    }
                }
            });
        }

        for i in 0..n_points {
            let data = rx.recv().unwrap();
            is_off_terrain[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Slope-based Filter: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        /////////////////////
        // Output the data //
        /////////////////////
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        let mut num_points_filtered = 0;
        if filter {
            output.header.system_id = "EXTRACTION".to_string();

            for point_num in 0..n_points {
                if !is_off_terrain[point_num] {
                    output.add_point_record(input.get_record(point_num));
                } else {
                    num_points_filtered += 1;
                }
                if verbose {
                    progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // classify
            for point_num in 0..n_points {
                let class_val = match !is_off_terrain[point_num] {
                    true => ground_class_value,
                    false => otp_class_value,
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
            println!("Warning: No points were filtered from the point cloud.");
        }

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Complete!")
            },
            Err(e) => println!("error while writing: {:?}", e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
