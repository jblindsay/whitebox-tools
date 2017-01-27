extern crate whitebox_tools;
extern crate nalgebra as na;
extern crate kdtree;

use std::env;
use std::path;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;
use na::Vector3;
use kdtree::KdTree;
use kdtree::distance::squared_euclidean;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut min_points: usize = 10;
    let mut verbose: bool = false;

    // read the arguments
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        let mut keyval = false;
        if vec.len() > 1 { keyval = true; }
        if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--i" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--o" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-num_points" || vec[0].to_lowercase() == "--num_points" {
            if keyval {
                min_points = vec[1].to_string().parse::<usize>().unwrap();
            } else {
                min_points = args[i+1].to_string().parse::<usize>().unwrap();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
          vec[0].to_lowercase() == "--h" {
            let mut s: String = "Help:\n".to_owned();
             s.push_str("-i           Input LAS file.\n");
             s.push_str("-o           Output LAS file.\n");
             s.push_str("-wd          Optional working directory. If specified, input and output filenames need not include a full path.\n");
             s.push_str("-num_points  Number (integer) of nearest-neighbour points used for plane fitting; default is 10.\n");
             s.push_str("-v           Optional verbose mode. Tool will report progress if this flag is provided.\n");
             s.push_str("-version     Prints the tool version number.\n");
             s.push_str("-h           Prints help information.\n\n");
             s.push_str("Example usage:\n\n");
             s.push_str(&">> .*lidar_normal_vec -v -wd *path*to*data* -i input.las -o output.las -num_points 15\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    if !working_directory.ends_with(&sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(&sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }

    lidar_normal_vec(input_file, output_file, min_points, verbose);
}

fn lidar_normal_vec(input_file: String, output_file: String, min_points: usize, verbose: bool) {
    if verbose {
        println!("*******************************");
        println!("* Welcome to lidar_normal_vec *");
        println!("*******************************");
    }

    if verbose { println!("Reading input LAS file..."); }
    //let input = las::LasFile::new(&input_file, "r");
    let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };

    if verbose { println!("Performing analysis..."); }

    let dimensions = 3;
    let capacity_per_node = 128;
    let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);

    let n_points = input.header.number_of_points as usize;
    let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

    let mut progress: i32;
    let mut old_progress: i32 = -1;
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        let coords: [f64; 3] = [ p.x, p.y, p.z ];
        kdtree.add(coords.clone(), i).unwrap();
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Creating tree: {}%", progress);
                //io::stdout().flush().ok().expect("Could not flush stdout");
                //stdout().flush();
                old_progress = progress;
            }
        }
    }

    if verbose { println!(""); }

    let mut index_n: usize;
    let mut normal_values: Vec<Vector3<f64>> = vec![];
    for i in 0..n_points {
        let p: PointData = input.get_point_info(i);
        let ret = kdtree.nearest(&[ p.x, p.y, p.z ], min_points, &squared_euclidean).unwrap();
        let mut data: Vec<Vector3<f64>> = vec![];;
        for j in 0..ret.len() {
            index_n = *ret[j].1;
            let p2: PointData = input.get_point_info(index_n);
            data.push(Vector3 { x: p2.x, y: p2.y, z: p2.z });
        }
        normal_values.push(plane_from_points(&data));
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("Calculating point normals: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if verbose { println!(""); }

    // now output the data
    let mut output = las::LasFile::initialize_using_file(&output_file, &input);
    output.header.point_format = 2;

    let (mut r, mut g, mut b): (u16, u16, u16);
    for i in 0..input.header.number_of_points as usize {
        let p: PointData = input.get_point_info(i);
        r = ((1.0 + normal_values[i].x) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].x) / 2.0 * 65535.0) as u16;
        g = ((1.0 + normal_values[i].y) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].y) / 2.0 * 65535.0) as u16;
        b = ((1.0 + normal_values[i].z) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].z) / 2.0 * 65535.0) as u16;
        let rgb: RgbData = RgbData{ red: r, green: g, blue: b };
        let lpr: las::LidarPointRecord = las::LidarPointRecord::PointRecord2 { point_data: p, rgb_data: rgb };
        output.add_point_record(lpr);
        if verbose {
            progress = (100.0_f64 * i as f64 / num_points) as i32;
            if progress != old_progress {
                println!("\rSaving data: {}%", progress);
                old_progress = progress;
            }
        }
    }

    println!("");
    if verbose { println!("Writing output LAS file..."); }
    let _ = match output.write() {
        Ok(_) => println!("Complete!"),
        Err(e) => println!("error while writing: {:?}", e),
    };
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

// struct Plane {
//     a: f64,
//     b: f64,
//     c: f64,
//     d: f64,
// }

// fn plane_from_point_and_normal(p: Vector3<f64>, normal: Vector3<f64>) -> Plane {
//     let d = normal.x * p.x + normal.y * p.y + normal.z * p.z;
//     Plane { a: normal.x, b: normal.y, c: normal.z, d: d }
// }
