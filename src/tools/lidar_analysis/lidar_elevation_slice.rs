extern crate time;

use std::env;
use std::f64;
use std::path;
use std::io::{Error, ErrorKind};
use lidar::las;
use tools::WhiteboxTool;

pub struct LidarElevationSlice {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarElevationSlice {
    pub fn new() -> LidarElevationSlice { // public constructor
        let name = "LidarElevationSlice".to_string();
        
        let description = "Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.".to_string();
        
        let parameters = "-i, --input        Input LAS file.
-o, --output       Output LAS file.
--maxz             Maximum elevation value.
--minz             Minimum elevation value.
--class            Optional boolean flag indicating whether points outside the range should be retained in output but reclassified.
--inclassval       Optional parameter specifying the class value assigned to points within the slice; default is 2.
--outclassval      Optional parameter specifying the class value assigned to points outside the slice; default is 1.".to_owned();
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --minz=100.0 --maxz=250.0
>>.*{0} -r={1} -i=\"*path*to*data*input.las\" -o=\"*path*to*data*output.las\" --minz=100.0 --maxz=250.0 --class
>>.*{0} -r={1} -i=\"*path*to*data*input.las\" -o=\"*path*to*data*output.las\" --minz=100.0 --maxz=250.0 --inclassval=1 --outclassval=0", short_exe, name).replace("*", &sep);
    
        LidarElevationSlice { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarElevationSlice {
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
        let mut minz = -f64::INFINITY;
        let mut maxz = f64::INFINITY;
        let mut filter = true;
        let mut in_class_value = 2u8;
        let mut out_class_value = 1u8;

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
            } else if vec[0].to_lowercase() == "-maxz" || vec[0].to_lowercase() == "--maxz" {
                if keyval {
                    maxz = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    maxz = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-minz" || vec[0].to_lowercase() == "--minz" {
                if keyval {
                    minz = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    minz = args[i+1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-class" || vec[0].to_lowercase() == "--class" {
                filter = false;
            } else if vec[0].to_lowercase() == "-inclassval" || vec[0].to_lowercase() == "--inclassval" {
                filter = false;
                if keyval {
                    in_class_value = vec[1].to_string().parse::<u8>().unwrap();
                } else {
                    in_class_value = args[i+1].to_string().parse::<u8>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-outclassval" || vec[0].to_lowercase() == "--outclassval" {
                filter = false;
                if keyval {
                    out_class_value = vec[1].to_string().parse::<u8>().unwrap();
                } else {
                    out_class_value = args[i+1].to_string().parse::<u8>().unwrap();
                }
            }
        }

        if !input_file.contains(path::MAIN_SEPARATOR) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(path::MAIN_SEPARATOR) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***********************************");
            println!("* Welcome to lidar_elevation_slice *");
            println!("************************************");
        }

        if in_class_value > 31 || out_class_value > 31 {
            return Err(Error::new(ErrorKind::InvalidInput, "Error: Either the in-slice or out-of-slice class values are larger than 31."));
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
        let input: las::LasFile = match las::LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(_) => return Err(Error::new(ErrorKind::NotFound, format!("No such file or directory ({})", input_file))),
        };
        let mut output = las::LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        if verbose { println!("Performing analysis..."); }
        let mut z: f64;
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut num_points_filtered: i64 = 0;
        let num_points: f64 = (input.header.number_of_points - 1) as f64;

        if filter {
            for i in 0..input.header.number_of_points as usize {
                z = input.get_point_info(i).z;
                if z >= minz && z <= maxz {
                    output.add_point_record(input.get_record(i));
                    num_points_filtered += 1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            for i in 0..input.header.number_of_points as usize {
                let mut class_val = out_class_value; // outside elevation slice
                z = input.get_point_info(i).z;
                if z >= minz && z <= maxz {
                    class_val = in_class_value; // inside elevation slice
                }
                let pr = input.get_record(i);
                let pr2: las::LidarPointRecord;
                match pr {
                    las::LidarPointRecord::PointRecord0 { mut point_data }  => {
                        point_data.set_classification(class_val);
                        pr2 = las::LidarPointRecord::PointRecord0 { point_data: point_data };

                    },
                    las::LidarPointRecord::PointRecord1 { mut point_data, gps_data } => {
                        point_data.set_classification(class_val);
                        pr2 = las::LidarPointRecord::PointRecord1 { point_data: point_data, gps_data: gps_data };
                    },
                    las::LidarPointRecord::PointRecord2 { mut point_data, rgb_data } => {
                        point_data.set_classification(class_val);
                        pr2 = las::LidarPointRecord::PointRecord2 { point_data: point_data, rgb_data: rgb_data };
                    },
                    las::LidarPointRecord::PointRecord3 { mut point_data, gps_data, rgb_data } => {
                        point_data.set_classification(class_val);
                        pr2 = las::LidarPointRecord::PointRecord3 { point_data: point_data,
                            gps_data: gps_data, rgb_data: rgb_data};
                    },
                }
                output.add_point_record(pr2);
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Saving data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            num_points_filtered = 1;
        }

        if num_points_filtered > 0 {
            if verbose { println!("Writing output LAS file..."); }
            let _ = match output.write() {
                Ok(_) => println!("Complete!"),
                Err(e) => println!("error while writing: {:?}", e),
            };
        } else {
            println!("No points were contained in the elevation slice.");
        }

        Ok(())
    }
}
