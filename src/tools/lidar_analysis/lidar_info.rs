/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 1, 2017
Last Modified: July 17, 2017
License: MIT
*/

use std;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::u16;
use lidar::*;
// use lidar::point_data::*;
use tools::WhiteboxTool;

pub struct LidarInfo {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarInfo {
    pub fn new() -> LidarInfo { // public constructor
        let name = "LidarInfo".to_string();
        
        let description = "Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.".to_string();
        
        let parameters = "-i, input        Input LAS file.
--vlr            Flag indicates whether to print variable length records (VLRs).
--geokeys        Flag indicates whether to print the geokeys.".to_owned();
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=file.las --vlr --geokeys\"
.*{0} -r={1} --wd=\"*path*to*data*\" -i=file.las", short_exe, name).replace("*", &sep);
    
        LidarInfo { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for LidarInfo {
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
        let mut show_vlrs = false;
        let mut show_geokeys = false;
        let mut keyval: bool;
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            keyval = false;
            if vec.len() > 1 { keyval = true; }
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-vlr" || vec[0].to_lowercase() == "--vlr" {
                show_vlrs = true;
            } else if vec[0].to_lowercase() == "-geokeys" || vec[0].to_lowercase() == "--geokeys" {
                show_geokeys = true;
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;
        // if !working_directory.ends_with(sep) {
        //     working_directory.push_str(&(sep.to_string()));
        // }

        if !input_file.contains(sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }

        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(_) => return Err(Error::new(ErrorKind::NotFound, format!("No such file or directory ({})", input_file))),
        };
        println!("{}", input);

        let num_points = input.header.number_of_points;
        let mut min_i = u16::MAX;
        let mut max_i = u16::MIN;
        let mut intensity: u16;
        let mut num_first: i64 = 0;
        let mut num_last: i64 = 0;
        let mut num_only: i64 = 0;
        let mut num_intermediate: i64 = 0;
        let mut ret: u8;
        let mut nrets: u8;
        let mut p: PointData;
        let mut ret_array: [i32; 5] = [0; 5];
        let mut class_array: [i32; 256] = [0; 256];
        for i in 0..input.header.number_of_points as usize {
            p = input[i]; //.get_point_info(i);
            ret = p.return_number();
            if ret > 5 {
                // Return is too high
                ret = 5;
            }
            ret_array[(ret - 1) as usize] += 1;
            nrets = p.number_of_returns();
            class_array[p.classification() as usize] += 1;
            if nrets == 1 {
                num_only += 1;
            } else if ret == 1 && nrets > 1 {
                num_first += 1;
            } else if ret == nrets {
                num_last += 1;
            } else {
                num_intermediate += 1;
            }
            intensity = p.intensity;
            if intensity > max_i { max_i = intensity; }
            if intensity < min_i { min_i = intensity; }
        }

        println!("\n\nMin I: {}\nMax I: {}", min_i, max_i);

        println!("\nPoint Return Table");
        for i in 0..5 {
            println!("Return {}:           {}", i + 1, ret_array[i]);
        }

        println!("\nPoint Position Table");
        println!("Only returns:         {}", num_only);
        println!("First returns:        {}", num_first);
        println!("Intermediate returns: {}", num_intermediate);
        println!("Last returns:         {}", num_last);

        println!("\nPoint Classification Table");
        for i in 0..256 {
            if class_array[i] > 0 {
                let percent: f64 = class_array[i] as f64 / num_points as f64 * 100.0;
                let percent_str = format!("{:.*}", 2, percent);
                let class_string = convert_class_val_to_class_string(i as u8);
                println!("{} ({}): {} ({}%)", class_string, i, class_array[i], percent_str);
            }

        }

        if show_vlrs {
            for i in 0..(input.header.number_of_vlrs as usize) {
                println!("\nVLR {}:\n{}", i, input.vlr_data[i].clone());
            }
        }

        if show_geokeys {
            println!("\n\n{}", input.geokeys.interpret_geokeys());
        }

        Ok(())
    }
}
