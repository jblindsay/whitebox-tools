/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: July 17, 2017
License: MIT
*/
extern crate time;
extern crate nalgebra as na;
extern crate num_cpus;

use std::env;
use std::f64;
use std::path;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use lidar::*;
// use lidar::point_data::*;
use tools::WhiteboxTool;
use self::na::Vector3;
use structures::FixedRadiusSearch3D;

pub struct NormalVectors {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl NormalVectors {
    pub fn new() -> NormalVectors { // public constructor
        let name = "NormalVectors".to_string();
        
        let description = "Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.".to_string();
        
        let parameters = "-i, --input        Input LAS file.
-o, --output       Output LAS file.
--radius           Search radius; default is 1.0.".to_owned();
  
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0", short_exe, name).replace("*", &sep);
    
        NormalVectors { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for NormalVectors {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut search_radius: f64 = -1.0;
        
        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 { keyval = true; }
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-radius" || vec[0].to_lowercase() == "--radius" {
                if keyval {
                    search_radius = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    search_radius = args[i+1].to_string().parse::<f64>().unwrap();
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

        let start = time::now();

        if verbose { println!("Performing analysis..."); }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch3D<usize> = FixedRadiusSearch3D::new(search_radius);
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            frs.insert(p.x, p.y, p.z, i);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut normal_values: Vec<Vector3<f64>> = vec![Vector3::<f64>{x: 0.0, y: 0.0, z: 0.0}; n_points];
        
        let frs = Arc::new(frs); // wrap FRS in an Arc
        let input = Arc::new(input); // wrap input in an Arc
        let mut starting_pt;
        let mut ending_pt = 0;
        let num_procs = num_cpus::get();
        let pt_block_size = n_points / num_procs;
        let (tx, rx) = mpsc::channel();
        let mut id = 0;
        while ending_pt < n_points {
            let frs = frs.clone();
            let input = input.clone();
            starting_pt = id * pt_block_size;
            ending_pt = starting_pt + pt_block_size;
            if ending_pt > n_points {
                ending_pt = n_points;
            }
            id += 1;
            let tx = tx.clone();
            thread::spawn(move || {
                let mut index_n: usize;
                for i in starting_pt..ending_pt {
                    let p: PointData = input.get_point_info(i);
                    let ret = frs.search(p.x, p.y, p.z);
                    let mut data: Vec<Vector3<f64>> = Vec::with_capacity(ret.len());
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        let p2: PointData = input.get_point_info(index_n);
                        data.push(Vector3 { x: p2.x, y: p2.y, z: p2.z });
                    }
                    tx.send((i, plane_from_points(&data))).unwrap();
                }
            });
        }

        for i in 0..n_points {
            let data = rx.recv().unwrap();
            normal_values[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Calculating point normals: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.point_format = 2;

        let (mut r, mut g, mut b): (u16, u16, u16);
        for i in 0..input.header.number_of_points as usize {
            let p: PointData = input.get_point_info(i);
            r = ((1.0 + normal_values[i].x) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].x) / 2.0 * 65535.0) as u16;
            g = ((1.0 + normal_values[i].y) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].y) / 2.0 * 65535.0) as u16;
            b = ((1.0 + normal_values[i].z) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].z) / 2.0 * 65535.0) as u16;
        
            let rgb: RgbData = RgbData{ red: r, green: g, blue: b };
            let lpr = LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

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

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
#[inline]
fn plane_from_points(points: &Vec<Vector3<f64>>) -> Vector3<f64> {
    let n = points.len();
    // assert!(n >= 3, "At least three points required");

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

    let det_max = det_x.max(det_y).max(det_z); //max3(det_x, det_y, det_z);
    // assert!(det_max > 0.0, "The points don't span a plane");

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

    //plane_from_point_and_normal(centroid, normalize(dir))
    normalize(dir)
}

#[inline]
fn normalize(v: Vector3<f64>) -> Vector3<f64> {
    let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    Vector3 { x: v.x/norm, y: v.y/norm, z: v.z/norm }
}