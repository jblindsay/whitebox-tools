/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay and Kevin Roberts
Created: 24/10/2018
Last Modified: 24/10/2018
License: MIT
*/

extern crate kdtree;

use whitebox_lidar::*;
use crate::tools::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool classifies points within a user-specified LiDAR point cloud (`--base`) that correspond
/// with points in a subset cloud (`--subset`). The subset point cloud may have been derived by filtering
/// the original point cloud. The user must specify the names of the two input LAS files (i.e.
/// the full and subset clouds) and the class value (`--subset_class`) to assign the matching points. This class
/// value will be assigned to points in the base cloud, overwriting their input class values in the
/// output LAS file (`--output`). Class values
/// should be numerical (integer valued) and should follow the LAS specifications below:
///
/// | Classification Value  | Meaning                              |
/// | :-------------------- | :------------------------------------|
/// | 0                     | Created never classified
/// | 1                     | Unclassified3
/// | 2                     | Ground
/// | 3                     | Low Vegetation
/// | 4                     | Medium Vegetation
/// | 5                     | High Vegetation
/// | 6                     | Building
/// | 7                     | Low Point (noise)
/// | 8                     | Reserved
/// | 9                     | Water
/// | 10                    | Rail
/// | 11                    | Road Surface
/// | 12                    | Reserved
/// | 13                    |	Wire – Guard (Shield)
/// | 14                    | Wire – Conductor (Phase)
/// | 15                    | Transmission Tower
/// | 16                    | Wire-structure Connector (e.g. Insulator)
/// | 17                    | Bridge Deck
/// | 18                    | High noise
///
/// The user may optionally specify a class value to be assigned to non-subset (i.e. non-matching)
/// points (`--nonsubset_class`) in the base file. If this parameter is not specified, output
/// non-sutset points will have the same class value as the base file.
pub struct LidarClassifySubset {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarClassifySubset {
    pub fn new() -> LidarClassifySubset {
        // public constructor
        let name = "LidarClassifySubset".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Classifies the values in one LiDAR point cloud that correspond with points in a subset cloud."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base LiDAR File".to_owned(),
            flags: vec!["--base".to_owned()],
            description: "Input base LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Subset LiDAR File".to_owned(),
            flags: vec!["--subset".to_owned()],
            description: "Input subset LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output LiDAR File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LiDAR file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Subset Point Class Value".to_owned(),
            flags: vec!["--subset_class".to_owned()],
            description: "Subset point class value (must be 0-18; see LAS specifications)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Non-Subset Point Class Value (Optional)".to_owned(),
            flags: vec!["--nonsubset_class".to_owned()],
            description: "Non-subset point class value (must be 0-18; see LAS specifications)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=\"full_cloud.las\" --subset=\"filtered_cloud.las\" -o=\"output.las\" --subset_class=2", short_exe, name).replace("*", &sep);

        LidarClassifySubset {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarClassifySubset {
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
        let mut input_base_file: String = "".to_string();
        let mut input_subset_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut subset_class: u8 = 255;
        let mut nonsubset_class: u8 = 255;

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
            if flag_val == "-base" {
                input_base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-subset" {
                input_subset_file = if keyval {
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
            } else if flag_val == "-subset_class" {
                subset_class = if keyval {
                    vec[1].to_string().parse::<u8>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<u8>().unwrap()
                };
            } else if flag_val == "-nonsubset_class" {
                nonsubset_class = if keyval {
                    vec[1].to_string().parse::<u8>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<u8>().unwrap()
                };
            }
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

        let sep = path::MAIN_SEPARATOR;
        if !input_base_file.contains(sep) && !input_base_file.contains("/") {
            input_base_file = format!("{}{}", working_directory, input_base_file);
        }
        if !input_subset_file.contains(sep) && !input_subset_file.contains("/") {
            input_subset_file = format!("{}{}", working_directory, input_subset_file);
        }
        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if subset_class > 18 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: An input point class value greater than the maximum stated in the LAS specification (18) has been used. Operation cancelled.",
            ));
        }

        if nonsubset_class > 18 && nonsubset_class != 255 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: An input point class value greater than the maximum stated in the LAS specification (18) has been used. Operation cancelled.",
            ));
        }

        if verbose {
            println!("Reading input files...");
        }
        let base_lidar = LasFile::new(&input_base_file, "r")?;
        let subset_lidar = LasFile::new(&input_subset_file, "r")?;

        let start = Instant::now();

        if verbose {
            println!("Performing analysis...");
        }

        let num_points = (base_lidar.header.number_of_points - 1) as f64;
        let mut progress: usize;
        let mut old_progress = 1usize;

        let capacity_per_node = 128;
        let mut kdtree = KdTree::with_capacity(3, capacity_per_node);
        println!("Creating tree...");
        for i in 0..subset_lidar.header.number_of_points as usize {
            // let p: PointData = subset_lidar.get_point_info(i);
            let p = subset_lidar.get_transformed_coords(i);
            kdtree.add([p.x, p.y, p.z], i).unwrap();
        }

        let tolerance = 2f64
            * base_lidar
                .header
                .x_scale_factor
                .max(subset_lidar.header.x_scale_factor);

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &base_lidar);

        println!("Performing analysis...");

        for i in 0..base_lidar.header.number_of_points as usize {
            let pd: PointData = base_lidar.get_point_info(i);
            let p = subset_lidar.get_transformed_coords(i);
            // let pr = base_lidar.get_transformed_coords(i);
            let pr = base_lidar.get_record(i);
            
            let pr2: LidarPointRecord;

            let ret = kdtree
                .nearest(&[p.x, p.y, p.z], 1, &squared_euclidean)
                .unwrap();
            if ret[0].0 <= tolerance {
                // We have a match. It's a subset point.
                match pr {
                    LidarPointRecord::PointRecord0 { mut point_data } => {
                        point_data.set_classification(subset_class);
                        pr2 = LidarPointRecord::PointRecord0 {
                            point_data: point_data,
                        };
                    }
                    LidarPointRecord::PointRecord1 {
                        mut point_data,
                        gps_data,
                    } => {
                        point_data.set_classification(subset_class);
                        pr2 = LidarPointRecord::PointRecord1 {
                            point_data: point_data,
                            gps_data: gps_data,
                        };
                    }
                    LidarPointRecord::PointRecord2 {
                        mut point_data,
                        colour_data,
                    } => {
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
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
                        point_data.set_classification(subset_class);
                        pr2 = LidarPointRecord::PointRecord10 {
                            point_data: point_data,
                            gps_data: gps_data,
                            colour_data: colour_data,
                            wave_packet: wave_packet,
                        };
                    }
                }
                output.add_point_record(pr2);
            } else {
                // We don't have a match. It's not a subset point.
                let class_val = match nonsubset_class == 255 {
                    true => pd.classification(),
                    false => nonsubset_class,
                };
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
            }

            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
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
