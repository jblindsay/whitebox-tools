/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/04/2018
Last Modified: 24/03/2022
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

use kd_tree::{KdPoint, KdTree};
use whitebox_lidar::*;
use whitebox_common::structures::Point3D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to flag points within an input LiDAR file (`--input`) that overlap with other 
/// nearby points from different flightlines, i.e. to identify overlap points. The flightline associated 
/// with a LiDAR point is assumed to be contained within the point's `Point Source ID` (PSID) property.
/// If the PSID property is not set, or has been lost, users may with to apply the `RecoverFlightlineInfo` 
/// tool prior to running `FlightlineOverlap`.
/// 
/// Areas of multiple flightline overlap tend to have point densities that are far greater than areas
/// of single flightlines. This can produce suboptimal results for applications that assume regular point 
/// distribution, e.g. in point classification operations.
/// 
/// The tool works by applying a square grid over the extent of the input LiDAR file. The grid cell size is 
/// determined by the user-defined `--resolution` parameter.  Grid cells containing multiple PSIDs, i.e. 
/// with more than one flightline, are then identified. Overlap points within these grid cells can then be 
/// flagged on the basis of a user-defined `--criterion`. The flagging options include the following:
/// 
/// | Criterion | Overlap Point Definition |
/// |:-|:-|
/// | `max scan angle` | All points that share the PSID of the point with the maximum absolute scan angle |
/// | `not min point source ID` | All points with a different PSID to that of the point with the lowest PSID |
/// | `not min time` | All points with a different PSID to that of the point with the minimum GPS time |
/// | `multiple point source IDs` | All points in grid cells with multiple PSIDs, i.e. all overlap points. |
/// 
/// Note that the `max scan angle` criterion may not be appropriate when more than two flightlines overlap, 
/// since it will result in only flagging points from one of the multiple flightlines. 
/// 
/// It is important to set the `--resolution` parameter appropriately, as setting this value too high will
/// yield the filtering of points in non-overlap areas, and setting the resolution to low will result in
/// fewer than expected points being flagged. An appropriate resolution size value may require experimentation,
/// however a value that is 2-3 times the nominal point spacing has been previously recommended. The nominal
/// point spacing can be determined using the `LidarInfo` tool.
/// 
/// By default, all flagged overlap points are reclassified in the output LiDAR file (`--output`) to class 
/// 12. Alternatively, if the user specifies the `--filter` parameter, then each overlap point will be 
/// excluded from the output file. Classified overlap points may also be filtered from LiDAR point clouds
/// using the `FilterLidar` tool.
/// 
/// Note that this tool is intended to be applied to LiDAR tile data containing points that have been merged
/// from multiple overlapping flightlines. It is commonly the case that airborne LiDAR data from each of the
/// flightlines from a survey are merged and then tiled into 1 km<sup>2</sup> tiles, which are the target
/// dataset for this tool.
///
/// # See Also
/// `FlightlineOverlap`, `RecoverFlightlineInfo`, `FilterLidar`, `LidarInfo`
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
        let description =
            "Classifies or filters LAS points in regions of overlapping flight lines.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
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
            name: "Sample Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description:
                "The size of the square area used to evaluate nearby points in the LiDAR data."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Overlap Criterion".to_owned(), 
            flags: vec!["-c".to_owned(), "--criterion".to_owned()], 
            description: "Criterion used to identify overlapping points; options are 'max scan angle', 'not min point source ID', 'not min time', 'multiple point source IDs'.".to_owned(),
            parameter_type: ParameterType::OptionList(
                vec![
                    "max scan angle".to_owned(), 
                    "not min point source ID".to_owned(), 
                    "not min time".to_owned(), 
                    "multiple point source IDs".to_owned(),
                ]
            ),
            default_value: Some("max scan angle".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.las --resolution=2.0",
            short_exe, name
        )
        .replace("*", &sep);

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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut filter = false;
        let mut based_on_scan_angle = true;
        let mut based_on_min_pt_src_id = false;
        let mut based_on_min_time = false;
        let mut based_on_all_overlapping_pts = false;

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
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
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
            } else if flag_val == "-c" || flag_val == "-criterion" {
                let criterion = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };

                if criterion.contains("scan") {
                    based_on_scan_angle = true;
                    based_on_min_pt_src_id = false;
                    based_on_min_time = false;
                    based_on_all_overlapping_pts = false;
                } else if criterion.contains("min point source id") || criterion.contains("min pt_src_id") {
                    based_on_scan_angle = false;
                    based_on_min_pt_src_id = true;
                    based_on_min_time = false;
                    based_on_all_overlapping_pts = false;
                } else if criterion.contains("time") {
                    based_on_scan_angle = false;
                    based_on_min_pt_src_id = false;
                    based_on_min_time = true;
                    based_on_all_overlapping_pts = false;
                } else { // multiple point source IDs -- i.e. all points in overlap areas
                    based_on_scan_angle = false;
                    based_on_min_pt_src_id = false;
                    based_on_min_time = false;
                    based_on_all_overlapping_pts = true;
                }
            } else if flag_val == "-filter" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    filter = true;
                }
            }
        }

        let start = Instant::now();

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
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        println!("reading input LiDAR file...");
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

        let mut points: Vec<TreeItem> = Vec::with_capacity(n_points);
        let mut p: Point3D;
        for i in 0..n_points {
            if !input[i].withheld() {
                p = input.get_transformed_coords(i);
                points.push( TreeItem { point: [p.x, p.y ], id: i } );
            }
        }
        // build the tree
        if verbose {
            println!("Building kd-tree...");
        }
        let kdtree: KdTree<TreeItem> = KdTree::build_by_ordered_float(points);

        let west: f64 = input.header.min_x;
        let north: f64 = input.header.max_y;
        let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
        let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;

        let mut overlapping = vec![false; n_points];
        let (mut x, mut y): (f64, f64);
        let (mut x_n, mut y_n): (f64, f64);
        let mut index_n: usize;
        let half_res_sqrd = grid_res / 2.0 * grid_res / 2.0;
        let mut pd: PointData;
        let mut num_overlap = 0;
        let search_dist = grid_res * 2.0_f64.sqrt();
        let mut t: f64;

        let mut max_angle: i16; // actually the max abs scan angle
        let mut max_angle_pt_src: u16;
        let mut min_pt_src: u16;
        let mut prev_pt_src: u16;
        let mut contains_multiple_pt_src_ids: bool;
        let mut min_t: f64;
        let mut min_t_pt_src: u16;
        for row in 0..rows as isize {
            for col in 0..columns as isize {
                x = west + col as f64 * grid_res + 0.5;
                y = north - row as f64 * grid_res - 0.5;

                let ret = kdtree.within_radius(&[x, y], search_dist);

                if ret.len() > 0 {
                    let mut point_nums: Vec<usize> = Vec::with_capacity(ret.len());
                    for i in 0..ret.len() {
                        x_n = ret[i].point[0];
                        y_n = ret[i].point[1];
                        if (x_n - x).powi(2) <= half_res_sqrd
                            && (y_n - y).powi(2) <= half_res_sqrd
                        {
                            // it falls within the grid cell
                            point_nums.push(ret[i].id);
                        }
                    }

                    if point_nums.len() > 0 {
                        max_angle = input[point_nums[0]].scan_angle.abs(); // actually the max abs scan angle
                        max_angle_pt_src = input[point_nums[0]].point_source_id;
                        min_pt_src = input[point_nums[0]].point_source_id;
                        prev_pt_src = input[point_nums[0]].point_source_id;
                        contains_multiple_pt_src_ids = false;
                        min_t = f64::INFINITY;
                        min_t_pt_src = input[point_nums[0]].point_source_id;
                        for j in 0..point_nums.len() {
                            index_n = point_nums[j];
                            pd = input[index_n];
                            if pd.point_source_id != prev_pt_src {
                                contains_multiple_pt_src_ids = true;
                            }
                            prev_pt_src = pd.point_source_id;
                            if pd.scan_angle.abs() > max_angle {
                                max_angle = pd.scan_angle.abs();
                                max_angle_pt_src = pd.point_source_id;
                            }
                            if pd.point_source_id < min_pt_src {
                                min_pt_src = pd.point_source_id;
                            }
                            t = input.get_gps_time(j).unwrap_or(0f64);
                            if t < min_t { 
                                min_t = t; 
                                min_t_pt_src = pd.point_source_id;
                            }
                        }

                        if contains_multiple_pt_src_ids {
                            for j in 0..point_nums.len() {
                                index_n = point_nums[j];
                                pd = input[index_n];
                                if based_on_all_overlapping_pts {
                                    num_overlap += 1;
                                    overlapping[index_n] = true;
                                }
                                if based_on_scan_angle && pd.point_source_id == max_angle_pt_src {
                                    num_overlap += 1;
                                    overlapping[index_n] = true;
                                }
                                if based_on_min_pt_src_id && pd.point_source_id != min_pt_src {
                                    num_overlap += 1;
                                    overlapping[index_n] = true;
                                }
                                if based_on_min_time && pd.point_source_id != min_t_pt_src {
                                    num_overlap += 1;
                                    overlapping[index_n] = true;
                                }
                            }
                        }
                    }
                }
                

                // let ret = frs.search(x, y);
                // if ret.len() > 0 {
                //     let mut point_nums: Vec<usize> = Vec::with_capacity(ret.len());
                //     for j in 0..ret.len() {
                //         index_n = ret[j].0;
                //         // let p = input[index_n];
                //         let p = input.get_transformed_coords(index_n);
                //         x_n = p.x;
                //         y_n = p.y;
                //         if (x_n - x) * (x_n - x) <= half_res_sqrd
                //             && (y_n - y) * (y_n - y) <= half_res_sqrd
                //         {
                //             // it falls within the grid cell
                //             point_nums.push(index_n);
                //         }
                //     }
                //     if point_nums.len() > 0 {
                //         // find the overall span of time in the cell and the index
                //         // with the minimum scan angle first and min time second
                //         let mut min_scan_angle = i16::max_value(); // actually the min abs scan angle
                //         let mut min_time = f64::INFINITY; // actually the earliest time for the points with the min abs scan angles.
                //         let mut earliest_time = f64::INFINITY;
                //         let mut latest_time = f64::NEG_INFINITY;
                //         for j in 0..point_nums.len() {
                //             index_n = point_nums[j];
                //             if gps_times[index_n] < earliest_time {
                //                 earliest_time = gps_times[index_n];
                //             }
                //             if gps_times[index_n] > latest_time {
                //                 latest_time = gps_times[index_n];
                //             }
                //             if scan_angles[index_n] <= min_scan_angle {
                //                 if gps_times[index_n] < min_time {
                //                     min_scan_angle = scan_angles[index_n];
                //                     min_time = gps_times[index_n];
                //                 }
                //             }
                //         }

                //         if latest_time - earliest_time > time_threshold {
                //             for j in 0..point_nums.len() {
                //                 overlapping[point_nums[j]] = true;
                //             }
                //             for j in 1..point_nums.len() {
                //                 index_n = point_nums[j];
                //                 if (gps_times[index_n] - min_time).abs() > time_threshold {
                //                     filtered[index_n] = true;
                //                 }
                //             }
                //         }
                //     }
                // }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        println!("Num. overlapping points flagged: {num_overlap}");

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        if filter {
            // filter points
            for i in 0..n_points {
                if !overlapping[i] {
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
                    let pr2: LidarPointRecord;
                    match pr {
                        LidarPointRecord::PointRecord0 { mut point_data } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord0 {
                                point_data: point_data,
                            };
                        }
                        LidarPointRecord::PointRecord1 {
                            mut point_data,
                            gps_data,
                        } => {
                            point_data.set_classification(12);
                            pr2 = LidarPointRecord::PointRecord1 {
                                point_data: point_data,
                                gps_data: gps_data,
                            };
                        }
                        LidarPointRecord::PointRecord2 {
                            mut point_data,
                            colour_data,
                        } => {
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                            point_data.set_classification(12);
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
                    progress = (100.0_f64 * i as f64 / num_points) as usize;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

struct TreeItem {
    point: [f64; 2],
    id: usize,
}

impl KdPoint for TreeItem {
    type Scalar = f64;
    type Dim = typenum::U2; // 3 dimensional tree.
    fn at(&self, k: usize) -> f64 { self.point[k] }
}