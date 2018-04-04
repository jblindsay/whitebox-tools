/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: February 18, 2018
Last Modified: February 18, 2018
License: MIT
*/

use time;
use num_cpus;
use std::env;
use std::f64;
use std::path;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use lidar::*;
use raster::*;
use tools::*;

pub struct LidarColourize {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarColourize {
    pub fn new() -> LidarColourize { // public constructor
        let name = "LidarColourize".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Adds the red-green-blue colour fields of a LiDAR (LAS) file based on an input image.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input LiDAR File".to_owned(), 
            flags: vec!["--in_lidar".to_owned()], 
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Input Colour Image File".to_owned(), 
            flags: vec!["--in_image".to_owned()], 
            description: "Input colour image file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output LiDAR File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false
        });
  
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --in_lidar=\"input.las\" --in_image=\"image.tif\" -o=\"output.las\" ", short_exe, name).replace("*", &sep);
    
        LidarColourize { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for LidarColourize {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_lidar_file: String = "".to_string();
        let mut input_image_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        
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
            if flag_val == "-in_lidar" {
                input_lidar_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-in_image" {
                input_image_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_lidar_file.contains(sep) && !input_lidar_file.contains("/") {
            input_lidar_file = format!("{}{}", working_directory, input_lidar_file);
        }
        if !input_image_file.contains(sep) && !input_image_file.contains("/") {
            input_image_file = format!("{}{}", working_directory, input_image_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading input files..."); }
        let in_lidar = Arc::new(LasFile::new(&input_lidar_file, "r")?);
        let in_image = Arc::new(Raster::new(&input_image_file, "r")?);

        let start = time::now();

        if verbose { println!("Performing analysis..."); }

        let n_points = in_lidar.header.number_of_points as usize;
        let num_points: f64 = (in_lidar.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let num_procs = num_cpus::get();
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let in_lidar = in_lidar.clone();
            let in_image = in_image.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut row, mut col): (isize, isize);
                let mut value: f64;
                let nodata = in_image.configs.nodata;
                for i in (0..n_points).filter(|point_num| point_num % num_procs == tid) {
                    let p: PointData = in_lidar.get_point_info(i);
                    row = in_image.get_row_from_y(p.y);
                    col = in_image.get_column_from_x(p.x);
                    value = in_image.get_value(row, col);
                    if value != nodata {
                        tx.send((i, value as u32)).unwrap();
                    } else {
                        tx.send((i, 0u32)).unwrap();
                    }
                }
            });
        }

        let mut colour_values: Vec<u32> = vec![0u32; n_points];
        for i in 0..n_points {
            let data = rx.recv().unwrap();
            colour_values[data.0] = data.1;
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        ////////////////////////////////////////////////////////////////
        // NOTICE THIS NEEDS UPDATING ONCE LAS 1.4 OUTPUT IS SUPPORTED /
        ////////////////////////////////////////////////////////////////

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &in_lidar);
        let out_pt_format = match in_lidar.header.point_format {
            0 | 2 => 2, // No GPS data supplied
            1 | 3 | 4 | 5 => 3, // GPS data is supplied
            6 | 7 | 8 | 9 | 10  => 3, // This is a 64-bit format and will require LAS 1.4 output support. For now, output point format 3.
            _ => return Err(Error::new(ErrorKind::InvalidInput, "Unsupport input point record format.")),
        };
        output.header.point_format = out_pt_format;

        let (mut r, mut g, mut b): (u16, u16, u16);
        let mut value: u32;
        let mut p: PointData;
        let mut gps: f64;
        for i in 0..in_lidar.header.number_of_points as usize {
            value = colour_values[i];
            r = (value & 0xFF) as u16 * 256u16;
            g = ((value >> 8) & 0xFF) as u16 * 256u16;
            b = ((value >> 16) & 0xFF) as u16 * 256u16;   
            let rgb: ColourData = ColourData{ red: r, green: g, blue: b, nir: 0u16 };

            p = in_lidar[i];

            if out_pt_format == 2 {
                output.add_point_record(LidarPointRecord::PointRecord2 { 
                    point_data: p, 
                    colour_data: rgb 
                });
            } else {
                gps = in_lidar.get_gps_time(i)?;
                output.add_point_record(LidarPointRecord::PointRecord3 { 
                    point_data: p, 
                    gps_data: gps, 
                    colour_data: rgb 
                });
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

        println!("");
        if verbose { println!("Writing output LAS file..."); }
        let _ = match output.write() {
            Ok(_) => println!("Complete!"),
            Err(e) => println!("error while writing: {:?}", e),
        };
        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}
