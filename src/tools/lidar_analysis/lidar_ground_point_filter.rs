extern crate time;
extern crate num_cpus;

use std::env;
use std::f64;
use std::path;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use lidar::las;
use lidar::point_data::*;
use tools::WhiteboxTool;
use structures::fixed_radius_search::FixedRadiusSearch2D;

pub struct LidarGroundPointFilter {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarGroundPointFilter {
    pub fn new() -> LidarGroundPointFilter { // public constructor
        let name = "LidarGroundPointFilter".to_string();
        
        let description = "Identifies ground points within LiDAR dataset.".to_string();
        
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
    
        LidarGroundPointFilter { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarGroundPointFilter {
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
        let mut otoheight: f64 = 1.0;
        let mut otoslope: f64 = 10.0;
        
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
            } else if vec[0].to_lowercase() == "-otoheight" || vec[0].to_lowercase() == "--otoheight" {
                if keyval {
                    otoheight = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    otoheight = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-otoslope" || vec[0].to_lowercase() == "--otoslope" {
                if keyval {
                    otoslope = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    otoslope = args[i+1].to_string().parse::<f64>().unwrap();
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
        //let input = las::LasFile::new(&input_file, "r");
        let input = match las::LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let start = time::now();

        if verbose { println!("Performing analysis..."); }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch2D<usize> = FixedRadiusSearch2D::new(search_radius);
        for i in 0..n_points {
            let p: PointData = input.get_point_info(i);
            frs.insert(p.x, p.y, i);
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut neighbourhood_min = vec![f64::MAX; n_points];
        let mut residuals = vec![f64::MIN; n_points];
        let mut off_terrain = vec![false; n_points];
        
        /////////////
        // Erosion //
        /////////////

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
                let mut z_n: f64;
                let mut min_z: f64;
                for i in starting_pt..ending_pt {
                    let p: PointData = input.get_point_info(i);
                    let ret = frs.search(p.x, p.y);
                    min_z = f64::MAX;
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        z_n = input.get_point_info(index_n).z;
                        if z_n < min_z {
                            min_z = z_n;
                        }
                    }
                    tx.send((i, min_z)).unwrap();
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
        id = 0;
        ending_pt = 0;
        while ending_pt < n_points {
            let frs = frs.clone();
            let input = input.clone();
            let neighbourhood_min = neighbourhood_min.clone();
            starting_pt = id * pt_block_size;
            ending_pt = starting_pt + pt_block_size;
            if ending_pt > n_points {
                ending_pt = n_points;
            }
            id += 1;
            let tx = tx.clone();
            thread::spawn(move || {
                let mut index_n: usize;
                let mut z_n: f64;
                let mut max_z: f64;
                for i in starting_pt..ending_pt {
                    let p: PointData = input.get_point_info(i);
                    let ret = frs.search(p.x, p.y);
                    max_z = f64::MIN;
                    for j in 0..ret.len() {
                        index_n = ret[j].0;
                        z_n = neighbourhood_min[index_n];
                        if z_n > max_z {
                            max_z = z_n;
                        }
                    }
                    tx.send((i, max_z)).unwrap();
                }
            });
        }

        for i in 0..n_points {
            let data = rx.recv().unwrap();
            let z = input.get_point_info(data.0).z;
            residuals[data.0] = z - data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Dilation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // now output the data
        let mut output = las::LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        for i in 0..n_points {
            if residuals[i] < otoheight {
                output.add_point_record(input.get_record(i));
            }
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

        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}

