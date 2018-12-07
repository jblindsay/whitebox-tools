/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 26, 2017
Last Modified: 12/10/2018
License: MIT
*/

use self::na::Vector3;
use crate::lidar::*;
use crate::na;
use crate::structures::{DistanceMetric, FixedRadiusSearch3D};
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.
pub struct NormalVectors {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NormalVectors {
    pub fn new() -> NormalVectors {
        // public constructor
        let name = "NormalVectors".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Calculates normal vectors for points within a LAS file and stores these data (XYZ vector components) in the RGB field.".to_string();

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
            default_value: Some("1.0".to_owned()),
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --radius=10.0", short_exe, name).replace("*", &sep);

        NormalVectors {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NormalVectors {
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
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-radius" {
                if keyval {
                    search_radius = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    search_radius = args[i + 1].to_string().parse::<f64>().unwrap();
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
        let mut frs: FixedRadiusSearch3D<usize> =
            FixedRadiusSearch3D::new(search_radius, DistanceMetric::SquaredEuclidean);
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

        let mut normal_values: Vec<Vector3<f64>> = vec![Vector3::new(0.0, 0.0, 0.0); n_points];

        let frs = Arc::new(frs); // wrap FRS in an Arc
        let input = Arc::new(input); // wrap input in an Arc
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut index_n: usize;
                for i in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let p: PointData = input.get_point_info(i);
                    let ret = frs.search(p.x, p.y, p.z);
                    let mut data: Vec<Vector3<f64>> = Vec::with_capacity(ret.len());
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        let p2: PointData = input.get_point_info(index_n);
                        data.push(Vector3::new(p2.x, p2.y, p2.z));
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
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            r = ((1.0 + normal_values[i].x) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].x) / 2.0 * 65535.0) as u16;
            g = ((1.0 + normal_values[i].y) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].y) / 2.0 * 65535.0) as u16;
            b = ((1.0 + normal_values[i].z) / 2.0 * 255.0) as u16 * 256u16; //((1.0 + normal_values[i].z) / 2.0 * 65535.0) as u16;

            let rgb: ColourData = ColourData {
                red: r,
                green: g,
                blue: b,
                nir: 0u16,
            };
            let lpr = LidarPointRecord::PointRecord2 {
                point_data: p,
                colour_data: rgb,
            };
            output.add_point_record(lpr);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Saving data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        println!("");
        if verbose {
            println!("Writing output LAS file...");
        }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
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

// Constructs a plane from a collection of points
// so that the summed squared distance to all points is minimzized
#[inline]
fn plane_from_points(points: &Vec<Vector3<f64>>) -> Vector3<f64> {
    let n = points.len();
    // assert!(n >= 3, "At least three points required");
    if n < 3 {
        return Vector3::new(0f64, 0f64, 0f64);
    }

    let mut sum = Vector3::new(0.0, 0.0, 0.0);
    for p in points {
        sum = sum + *p;
    }
    let centroid = sum * (1.0 / (n as f64));

    // Calc full 3x3 covariance matrix, excluding symmetries:
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut xz = 0.0;
    let mut yy = 0.0;
    let mut yz = 0.0;
    let mut zz = 0.0;

    for p in points {
        let r = p - &centroid;
        xx += r.x * r.x;
        xy += r.x * r.y;
        xz += r.x * r.z;
        yy += r.y * r.y;
        yz += r.y * r.z;
        zz += r.z * r.z;
    }

    let det_x = yy * zz - yz * yz;
    let det_y = xx * zz - xz * xz;
    let det_z = xx * yy - xy * xy;

    let det_max = det_x.max(det_y).max(det_z); //max3(det_x, det_y, det_z);
                                               // assert!(det_max > 0.0, "The points don't span a plane");

    // Pick path with best conditioning:
    let dir = if det_max == det_x {
        let a = (xz * yz - xy * zz) / det_x;
        let b = (xy * yz - xz * yy) / det_x;
        Vector3::new(1.0, a, b)
    } else if det_max == det_y {
        let a = (yz * xz - xy * zz) / det_y;
        let b = (xy * xz - yz * xx) / det_y;
        Vector3::new(a, 1.0, b)
    } else {
        let a = (yz * xy - xz * yy) / det_z;
        let b = (xz * xy - yz * xx) / det_z;
        Vector3::new(a, b, 1.0)
    };

    //plane_from_point_and_normal(centroid, normalize(dir))
    normalize(dir)
}

#[inline]
fn normalize(v: Vector3<f64>) -> Vector3<f64> {
    let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    Vector3::new(v.x / norm, v.y / norm, v.z / norm)
}
