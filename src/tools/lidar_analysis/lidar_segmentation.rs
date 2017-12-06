/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 5/12/2017, 2017
Last Modified: 5/12/2017, 2017
License: MIT

Notes: The 3D space-filling nature of point clouds under heavy forest cover do not
       lend themselves to useful estimation of point normal vectors. As such,
       this tool will not work satisfactory under dense forest cover. Tree cover
       should first be removed using the LidarGroundPointRemoval or similar tool.
*/
extern crate time;
extern crate nalgebra as na;
extern crate num_cpus;
extern crate rand;

use std::cmp;
use std::env;
use std::f64;
use std::f64::NEG_INFINITY;
use std::path;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use lidar::*;
use tools::*;
use self::na::Vector3;
use structures::FixedRadiusSearch3D;
use self::rand::Rng;

pub struct LidarSegmentation {
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarSegmentation {
    pub fn new() -> LidarSegmentation { // public constructor
        let name = "LidarSegmentation".to_string();
        
        let description = "Segments a LiDAR point cloud based on normal vectors.".to_string();
        
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
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Search Radius".to_owned(), 
            flags: vec!["--dist".to_owned(), "--radius".to_owned()], 
            description: "Search Radius.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("5.0".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Normal Difference Threshold".to_owned(), 
            flags: vec!["--norm_diff".to_owned()], 
            description: "Maximum difference in normal vectors, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: false
        });
        
        parameters.push(ToolParameter{
            name: "Maximum Elevation Difference Between Points".to_owned(), 
            flags: vec!["--maxzdiff".to_owned()], 
            description: "Maximum difference in elevation (z units) between neighbouring points of the same segment.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: false
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0 --norm_diff=2.5 --maxzdiff=0.75", short_exe, name).replace("*", &sep);
    
        LidarSegmentation { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarSegmentation {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut search_radius = 5f64;
        let mut max_norm_diff = 2f64;
        let mut max_z_diff = 1f64;
        
        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 { keyval = true; }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if flag_val == "-dist" || flag_val == "-radius" {
                if keyval {
                    search_radius = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    search_radius = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-norm_diff" {
                if keyval {
                    max_norm_diff = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_norm_diff = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if flag_val == "-maxzdiff" {
                if keyval {
                    max_z_diff = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_z_diff = args[i+1].to_string().parse::<f64>().unwrap();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading input LAS file..."); }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };
        let n_points = input.header.number_of_points as usize;
        let num_points = n_points as f64;

        let start = time::now();

        if max_norm_diff < 0f64 { max_norm_diff = 0f64; }
        if max_norm_diff > 90f64 { max_norm_diff = 90f64; }
        max_norm_diff = max_norm_diff.to_radians();
        
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let num_procs = num_cpus::get();
        let input = Arc::new(input); // wrap input in an Arc
        
        /////////////////////////////////////////////////////////
        // Calculate the normals for each point in the dataset //
        /////////////////////////////////////////////////////////
        if verbose { println!("Calculating point normals..."); }
        let mut frs3d: FixedRadiusSearch3D<usize> = FixedRadiusSearch3D::new(search_radius);
        for point_num in 0..n_points {
            let p: PointData = input.get_point_info(point_num);
            frs3d.insert(p.x, p.y, p.z, point_num); 
            if verbose {
                progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points in 3D: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let frs = Arc::new(frs3d); // wrap FRS in an Arc
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut index_n: usize;
                let mut height_diff: f64;
                for point_num in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let p: PointData = input.get_point_info(point_num);
                    let ret = frs.search(p.x, p.y, p.z);
                    let mut data: Vec<Vector3<f64>> = Vec::with_capacity(ret.len());
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        let pn: PointData = input.get_point_info(index_n);
                        height_diff = (pn.z - p.z).abs();
                        if height_diff < max_z_diff {
                            data.push(Vector3 { x: pn.x, y: pn.y, z: pn.z });
                        }
                    }
                    tx.send((point_num, plane_from_points(&data))).unwrap();
                }
            });
        }

        let mut normal_vectors = vec![Normal::new(); n_points];
        for point_num in 0..n_points {
            let data = rx.recv().unwrap();
            normal_vectors[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Calculating point normals: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        ////////////////////////////////////////
        // Perform the segmentation operation //
        ////////////////////////////////////////
        if verbose { println!("Segmenting the point cloud..."); }
        let mut segment_id = vec![0usize; n_points];
        let mut current_segment = 0usize;
        let mut point_id: usize;
        let mut norm_diff: f64;
        let mut height_diff: f64;
        let mut index_n: usize;
        let mut solved_points = 0;
        let mut stack = vec![];
        while solved_points < n_points {
            // Find a seed-point for a segment
            for i in 0..n_points {
                if segment_id[i] == 0 {
                    // No segment ID has yet been assigned to this point.
                    current_segment += 1;
                    segment_id[i] = current_segment;
                    stack.push(i);
                    break;
                }
            }

            while !stack.is_empty() {
                solved_points += 1;
                if verbose {
                    progress = (100f64 * solved_points as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Segmenting the point cloud: {}%", progress);
                        old_progress = progress;
                    }
                }
                point_id = stack.pop().unwrap();
                /* Check the neighbours to see if there are any
                points that have similar normal vectors and 
                heights. */
                let p: PointData = input.get_point_info(point_id);
                let ret = frs.search(p.x, p.y, p.z);
                for j in 0..ret.len() {
                    index_n = ret[j].0;
                    if segment_id[index_n] == 0 { 
                        // It hasn't already been placed in a segment.
                        let pn: PointData = input.get_point_info(index_n);
                        // Calculate height difference.
                        height_diff = (pn.z - p.z).abs();
                        if height_diff < max_z_diff {
                            // Check the difference in normal vectors.
                            norm_diff = normal_vectors[point_id].angle_between(normal_vectors[index_n]);
                            if norm_diff < max_norm_diff {
                                // This neighbour is part of the ground.
                                segment_id[index_n] = current_segment;
                                stack.push(index_n);
                            }
                        }
                    }
                }
            }
        }

        /////////////////////
        // Output the data //
        /////////////////////

        let mut clrs: Vec<(u16, u16, u16)> = Vec::new();
        let mut rng = rand::thread_rng();
        let (mut r, mut g, mut b): (u16, u16, u16) = (0u16, 0u16, 0u16);
        for _ in 0..current_segment+1 as usize {
            let mut flag = false;
            while !flag {
                r = rng.gen::<u8>() as u16 * 256u16;
                g = rng.gen::<u8>() as u16 * 256u16;
                b = rng.gen::<u8>() as u16 * 256u16;
                let max_val = cmp::max(cmp::max(r, g), b);
                //let min_val = cmp::min(cmp::min(r, g), b);
                if max_val >= u16::max_value() / 2 { // && min_val >= u16::max_value() / 4 {
                    flag = true;
                }
            }
            clrs.push((r, g, b));
        }

        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.point_format = 2;
        for point_num in 0..n_points {
            let p: PointData = input[point_num];
            let seg_val = segment_id[point_num];
            let rgb: RgbData = RgbData{ red: clrs[seg_val].0, green: clrs[seg_val].1, blue: clrs[seg_val].2 };
            let lpr: LidarPointRecord = LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * point_num as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        // let (mut r, mut g, mut b): (u16, u16, u16);
        // for i in 0..n_points {
        //     let p: PointData = input.get_point_info(i);
        //     r = ((1.0 + normal_values[i].x) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].x) / 2.0 * 65535.0) as u16;
        //     g = ((1.0 + normal_values[i].y) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].y) / 2.0 * 65535.0) as u16;
        //     b = ((1.0 + normal_values[i].z) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].z) / 2.0 * 65535.0) as u16;
        
        //     let rgb: RgbData = RgbData{ red: r, green: g, blue: b };
        //     let lpr = LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
        //     output.add_point_record(lpr);
        //     if verbose {
        //         progress = (100.0_f64 * i as f64 / num_points) as i32;
        //         if progress != old_progress {
        //             println!("Saving data: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let end = time::now();
        let elapsed_time = end - start;

        println!("");
        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct Normal {
    a: f64,
    b: f64,
    c: f64,
}

impl Normal {
    fn new() -> Normal {
        // angle_between won't work with perfectly flat normals so add a small delta.
        Normal { a: 0.0000001, b: 0f64, c: 0f64}
    }

    // fn from_vector3(v: Vector3<f64>) -> Normal {
    //     if v.x == 0f64 && v.y == 0f64 && v.z == 0f64 {
    //         return Normal { a: 0.0000001, b: 0f64, c: 0f64}; 
    //         // angle_between won't work with perfectly flat normals so add a small delta.
    //     }
    //     Normal { a: v.x, b: v.y, c: v.z }
    // }

    fn angle_between(self, other: Normal) -> f64 {
        let numerator = self.a * other.a + self.b * other.b + self.c * other.c;
        let denom1 = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        let denom2 = (other.a * other.a + other.b * other.b + other.c * other.c).sqrt();
        if denom1*denom2 != 0f64 {
            return (numerator / (denom1 * denom2)).acos();
        }
        NEG_INFINITY
    }
}

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
#[inline]
fn plane_from_points(points: &Vec<Vector3<f64>>) -> Normal {
    let n = points.len();
    // assert!(n >= 3, "At least three points required");
    if n < 3 {
        return Normal { a: 0f64, b: 0f64, c: 0f64 };
    }

    let mut sum = Vector3{ x: 0.0, y: 0.0, z: 0.0 };
    for p in points {
        sum = sum + *p;
    }
    let centroid = sum * (1.0 / (n as f64));

    // Calc full 3x3 covariance matrix, excluding symmetries:
    let mut xx = 0.0; let mut xy = 0.0; let mut xz = 0.0;
    let mut yy = 0.0; let mut yz = 0.0; let mut zz = 0.0;

    for p in points {
        let r = p - &centroid;
        xx += r.x * r.x;
        xy += r.x * r.y;
        xz += r.x * r.z;
        yy += r.y * r.y;
        yz += r.y * r.z;
        zz += r.z * r.z;
    }

    let det_x = yy*zz - yz*yz;
    let det_y = xx*zz - xz*xz;
    let det_z = xx*yy - xy*xy;

    let det_max = det_x.max(det_y).max(det_z);

    // Pick path with best conditioning:
    let dir =
        if det_max == det_x {
            let a = (xz*yz - xy*zz) / det_x;
            let b = (xy*yz - xz*yy) / det_x;
            Vector3{ x: 1.0, y: a, z: b }
        } else if det_max == det_y {
            let a = (yz*xz - xy*zz) / det_y;
            let b = (xy*xz - yz*xx) / det_y;
            Vector3{ x: a, y: 1.0, z: b }
        } else {
            let a = (yz*xy - xz*yy) / det_z;
            let b = (xz*xy - yz*xx) / det_z;
            Vector3{ x: a, y: b, z: 1.0 }
        };

    normalize(dir)
}

#[inline]
fn normalize(v: Vector3<f64>) -> Normal {
    let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    Normal { a: v.x/norm, b: v.y/norm, c: v.z/norm }
}