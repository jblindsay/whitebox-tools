/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/06/2017
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to either extract or classify the elevation values (z) of LiDAR points within
/// a specified elevation range (slice). In addition to the names of the input and output LiDAR files
/// (`--input` and `--output`), the user must specify the lower (`--minz`) and upper (`--maxz`) bounds of
/// the elevation range. By default, the tool will only output points within the elevation slice, filtering
/// out all points lying outside of this range. If the `--class` parameter is used, the tool will operate
/// by assigning a class value (`--inclassval`) to the classification bit of points within the slice and
/// another class value (`--outclassval`) to those points falling outside the range.
///
/// # See Also
/// `LidarRemoveOutliers`, `LidarClassifySubset`
pub struct LidarElevationSlice {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarElevationSlice {
    pub fn new() -> LidarElevationSlice {
        // public constructor
        let name = "LidarElevationSlice".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Outputs all of the points within a LiDAR (LAS) point file that lie between a specified elevation range.".to_string();

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
            name: "Minimum Elevation Value".to_owned(),
            flags: vec!["--minz".to_owned()],
            description: "Minimum elevation value (optional).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Elevation Value".to_owned(),
            flags: vec!["--maxz".to_owned()],
            description: "Maximum elevation value (optional).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Retain but reclass points outside the specified elevation range?".to_owned(), 
            flags: vec!["--class".to_owned()], 
            description: "Optional boolean flag indicating whether points outside the range should be retained in output but reclassified.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Class Value Assigned to Points Within Range (Optional)".to_owned(),
            flags: vec!["--inclassval".to_owned()],
            description:
                "Optional parameter specifying the class value assigned to points within the slice."
                    .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("2".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Class Value Assigned to Points Outside Range (Optional)".to_owned(),
            flags: vec!["--outclassval".to_owned()],
            description:
                "Optional parameter specifying the class value assigned to points within the slice."
                    .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --minz=100.0 --maxz=250.0
>>.*{0} -r={1} -v -i=\"*path*to*data*input.las\" -o=\"*path*to*data*output.las\" --minz=100.0 --maxz=250.0 --class
>>.*{0} -r={1} -v -i=\"*path*to*data*input.las\" -o=\"*path*to*data*output.las\" --minz=100.0 --maxz=250.0 --inclassval=1 --outclassval=0", short_exe, name).replace("*", &sep);

        LidarElevationSlice {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarElevationSlice {
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
        let mut minz = f64::NEG_INFINITY;
        let mut maxz = f64::INFINITY;
        let mut filter = true;
        let mut in_class_value = 2u8;
        let mut out_class_value = 1u8;

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
            } else if flag_val == "-maxz" {
                maxz = if keyval {
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
            } else if flag_val == "-minz" {
                minz = if keyval {
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
            } else if flag_val == "-class" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    filter = false;
                }
            } else if flag_val == "-inclassval" {
                in_class_value = if keyval {
                    vec[1].to_string().parse::<u8>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<u8>().unwrap()
                };
            } else if flag_val == "-outclassval" {
                out_class_value = if keyval {
                    vec[1].to_string().parse::<u8>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<u8>().unwrap()
                };
            }
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***********************************");
            println!("* Welcome to lidar_elevation_slice *");
            println!("************************************");
        }

        if in_class_value > 31 || out_class_value > 31 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: Either the in-slice or out-of-slice class values are larger than 31.",
            ));
        }

        let sep = path::MAIN_SEPARATOR;
        if !input_file.contains(sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("reading input LiDAR file...");
        }
        let input: LasFile = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("No such file or directory ({})", input_file),
                ))
            }
        };
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        if verbose {
            println!("Performing analysis...");
        }
        let mut z: f64;
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut num_points_filtered: i64 = 0;
        let num_points: f64 = (input.header.number_of_points - 1) as f64;

        if filter {
            for i in 0..input.header.number_of_points as usize {
                // z = input.get_point_info(i).z;
                z = input.get_transformed_coords(i).z;
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
                // z = input.get_point_info(i).z;
                z = input.get_transformed_coords(i).z;
                if z >= minz && z <= maxz {
                    class_val = in_class_value; // inside elevation slice
                }
                let pr = input.get_record(i);
                let pr2: LidarPointRecord;
                match pr {
                    LidarPointRecord::PointRecord0 { mut point_data } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord0 {
                            point_data: point_data,
                        };
                    }
                    LidarPointRecord::PointRecord1 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord1 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord2 {
                        mut point_data,
                        colour_data,
                    } => {
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
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
                        point_data.set_classification(class_val);
                        pr2 = LidarPointRecord::PointRecord10 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
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
        } else {
            println!("No points were contained in the elevation slice.");
        }

        Ok(())
    }
}
