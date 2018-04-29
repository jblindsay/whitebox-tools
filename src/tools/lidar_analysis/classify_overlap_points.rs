/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/04/2018
Last Modified: 27/04/2018
License: MIT

NOTES: If the --filter flag is specified, points from overlapping flightlines (i.e. later GPS times)
are culled from the output point cloud. If this flag is left off, then all overlapping points are
classified as such by setting the classification to 12. Note that points are considered
to be from different flightlines if their GPS times are different by greater than 15 units. Nearby 
points that are from the same flightline generally have times that differ by several orders of magnitude
less than this threshold and neighbouring points from different flightlines generally have times that 
differ by orders of magnitude higher than this threshold. This tool assumes that GPS data are available
for the input LAS file.

When the LAS encoder is updated to output v 1.4 LAS files, the overlap flag should be used to 
designate overlapping points in 'classify' mode rather than class 12.
*/

use time;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use lidar::*;
use structures::FixedRadiusSearch2D;
use tools::*;

pub struct ClassifyOverlapPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ClassifyOverlapPoints {
    pub fn new() -> ClassifyOverlapPoints {
        // public constructor
        let name = "ClassifyOverlapPoints".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Classifies or filters LAS point in regions of overlapping flight lines.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input LiDAR File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Sample Resolution".to_owned(), 
            flags: vec!["--resolution".to_owned()], 
            description: "The distance of the square area used to evaluate nearby points in the LiDAR data.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Filter out points from overlapping flightlines?".to_owned(), 
            flags: vec!["--filter".to_owned()], 
            description: "Filter out points from overlapping flightlines? If false, overlaps will simply be classified.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=5.0 --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        ClassifyOverlapPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ClassifyOverlapPoints {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut filter = false;

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters."));
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
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-filter" {
                filter = true;
            }
        }

        let start = time::now();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        println!("Reading input LAS file...");
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        println!("Performing analysis...");

        // Make sure that the input LAS file have GPS time data?
        if input.header.point_format == 0u8 || input.header.point_format == 2u8 {
            panic!("The input file has a Point Format that does not include GPS time, which is required for the operation of this tool.");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        // let search_dist = grid_res / 2.0;
        let mut frs: FixedRadiusSearch2D<usize> = FixedRadiusSearch2D::new(grid_res);
        let mut gps_times = vec![-1f64; n_points];
        let mut scan_angles = vec![016; n_points];
        let (mut x, mut y, mut gps_time): (f64, f64, f64);
        let mut sa: i16;
        for i in 0..n_points {
            match input.get_record(i) {
                LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                },
                LidarPointRecord::PointRecord3 { mut point_data, gps_data, colour_data } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = colour_data;
                },
                LidarPointRecord::PointRecord4 { mut point_data, gps_data, wave_packet } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = wave_packet;
                },
                LidarPointRecord::PointRecord5 { mut point_data, gps_data, colour_data, wave_packet } => {
                    x = point_data.x;
                    y = point_data.y;
                    gps_time = gps_data;
                    sa = point_data.scan_angle;
                    let _ = colour_data;
                    let _ = wave_packet;
                },
                LidarPointRecord::PointRecord6 { mut point_data, gps_data } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                },
                LidarPointRecord::PointRecord7 { mut point_data, gps_data, colour_data } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = colour_data;
                },
                LidarPointRecord::PointRecord8 { mut point_data, gps_data, colour_data } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = colour_data;
                },
                LidarPointRecord::PointRecord9 { mut point_data, gps_data, wave_packet } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = wave_packet;
                },
                LidarPointRecord::PointRecord10 { mut point_data, gps_data, colour_data, wave_packet } => {
                    x = point_data.x;
                    y = point_data.y;
                    sa = point_data.scan_angle;
                    gps_time = gps_data;
                    let _ = colour_data;
                    let _ = wave_packet;
                },
                _ => {
                    panic!("The input file has a Point Format that does not include GPS time, which is required for the operation of this tool.");
                }
            };
            frs.insert(x, y, i);
            gps_times[i] = gps_time;
            scan_angles[i] = sa.abs();
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as usize;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let west: f64 = input.header.min_x; 
        let north: f64 = input.header.max_y;
        let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
        let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;
        
        let mut filtered = vec![false; n_points];
        let mut overlapping = vec![false; n_points];
        let time_threshold = 15f64;
        let (mut x_n, mut y_n): (f64, f64);
        let mut index_n: usize;
        let half_res_sqrd = grid_res / 2.0 * grid_res / 2.0;
        for row in 0..rows as isize {
            for col in 0..columns as isize {
                x = west + col as f64 * grid_res + 0.5;
                y = north - row as f64 * grid_res - 0.5;
                let ret = frs.search(x, y);
                if ret.len() > 0 {
                    let mut point_nums: Vec<usize> = Vec::with_capacity(ret.len());
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        let p = input[index_n];
                        x_n = p.x;
                        y_n = p.y;
                        if (x_n - x) * (x_n - x) <= half_res_sqrd &&
                        (y_n - y) * (y_n - y) <= half_res_sqrd {
                            // it falls within the grid cell
                            point_nums.push(index_n);
                        }
                    }
                    if point_nums.len() > 0 {
                        // find the overall span of time in the cell and the index 
                        // with the minimum scan angle first and min time second
                        let mut min_scan_angle = i16::max_value(); // actually the min abs scan angle
                        let mut min_time = f64::INFINITY; // actually the earliest time for the points with the min abs scan angles.
                        let mut earliest_time = f64::INFINITY;
                        let mut latest_time = f64::NEG_INFINITY;
                        for j in 0..point_nums.len() {
                            index_n = point_nums[j];
                            if gps_times[index_n] < earliest_time { earliest_time = gps_times[index_n]; }
                            if gps_times[index_n] > latest_time { latest_time = gps_times[index_n]; }
                            if scan_angles[index_n] <= min_scan_angle {
                                if gps_times[index_n] < min_time {
                                    min_scan_angle = scan_angles[index_n];
                                    min_time = gps_times[index_n];
                                }
                            }
                        }

                        if latest_time - earliest_time > time_threshold {
                            for j in 0..point_nums.len() {
                                overlapping[point_nums[j]] = true;
                            }
                            for j in 1..point_nums.len() {
                                index_n = point_nums[j];
                                if (gps_times[index_n] - min_time).abs() > time_threshold {
                                    filtered[index_n] = true;
                                }
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        if filter {
            // filter points
            for i in 0..n_points {
                if !filtered[i] {
                    output.add_point_record(input.get_record(i));
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as usize;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // set overlap flag
            for i in 0..n_points {
                if !overlapping[i] {
                    output.add_point_record(input.get_record(i));
                } else {
                    let pr = input.get_record(i);
                    // pr.point_data.set_overlap(true); // change to this when 1.4 output is supported
                    let pr2: LidarPointRecord;
                    match pr {
                        LidarPointRecord::PointRecord0 { mut point_data }  => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord0 { point_data: point_data };

                        },
                        LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data };
                        },
                        LidarPointRecord::PointRecord2 { mut point_data, colour_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord2 { point_data: point_data, colour_data: colour_data };
                        },
                        LidarPointRecord::PointRecord3 { mut point_data, gps_data, colour_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord3 { point_data: point_data,
                                gps_data: gps_data, colour_data: colour_data};
                        },
                        LidarPointRecord::PointRecord4 { mut point_data, gps_data, wave_packet } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord4 { point_data: point_data,
                                gps_data: gps_data, wave_packet: wave_packet};
                        },
                        LidarPointRecord::PointRecord5 { mut point_data, gps_data, colour_data, wave_packet } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord5 { point_data: point_data,
                                gps_data: gps_data, colour_data: colour_data, wave_packet: wave_packet};
                        },
                        LidarPointRecord::PointRecord6 { mut point_data, gps_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord6 { point_data: point_data,
                                gps_data: gps_data};
                        },
                        LidarPointRecord::PointRecord7 { mut point_data, gps_data, colour_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord7 { point_data: point_data,
                                gps_data: gps_data, colour_data: colour_data};
                        },
                        LidarPointRecord::PointRecord8 { mut point_data, gps_data, colour_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord8 { point_data: point_data,
                                gps_data: gps_data, colour_data: colour_data};
                        },
                        LidarPointRecord::PointRecord9 { mut point_data, gps_data, wave_packet } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord9 { point_data: point_data,
                                gps_data: gps_data, wave_packet: wave_packet};
                        },
                        LidarPointRecord::PointRecord10 { mut point_data, gps_data, colour_data, wave_packet } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord10 { point_data: point_data,
                                gps_data: gps_data, colour_data: colour_data, wave_packet: wave_packet};
                        },
                    }
                    output.add_point_record(pr2);
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as usize;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };
        if verbose {
            println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}
