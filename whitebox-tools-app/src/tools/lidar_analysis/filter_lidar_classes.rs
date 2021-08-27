/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/07/2019
Last Modified: 16/01/2020
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to remove points within a LAS LiDAR file that possess certain
/// specified class values. The user must input the names of the input (`--input`) and
/// output (`--output`) LAS files and the class values to be excluded (`--exclude_cls`).
/// Class values are specified by their numerical values, such that:
///
/// | Classification Value  | Meaning                              |
/// | :-------------------- | :------------------------------------|
/// | 0                     | Created never classified
/// | 1                     | Unclassified
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
/// | 13                    | Wire – Guard (Shield)
/// | 14                    | Wire – Conductor (Phase)
/// | 15                    | Transmission Tower
/// | 16                    | Wire-structure Connector (e.g. Insulator)
/// | 17                    | Bridge Deck
/// | 18                    | High noise
///
/// Thus, to filter out low and high noise points from a point cloud, specify
/// `--exclude_cls='7,18'`. Class ranges may also be specified, e.g. `--exclude_cls='3-5,7,18'`.
/// Notice that usage of this tool assumes that the
/// LAS file has underwent a comprehensive point classification, which not all
/// point clouds have had. Use the `LidarInfo` tool determine the distribution
/// of various class values in your file.
///
/// # See Also
/// `LidarInfo`
pub struct FilterLidarClasses {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FilterLidarClasses {
    pub fn new() -> FilterLidarClasses {
        // public constructor
        let name = "FilterLidarClasses".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Removes points in a LAS file with certain specified class values.".to_string();

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

        parameters.push(ToolParameter{
            name: "Exclusion Classes (0-18, based on LAS spec; e.g. 7,18)".to_owned(), 
            flags: vec!["--exclude_cls".to_owned()], 
            description: "Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"input.las\" -o=\"output.las\" --exclude_cls='7,18'", short_exe, name).replace("*", &sep);

        FilterLidarClasses {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FilterLidarClasses {
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
        let mut include_class_vals = vec![true; 256];

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
            } else if flag_val == "-exclude_cls" {
                let exclude_cls_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                let mut cmd = exclude_cls_str.split(",");
                let mut vec = cmd.collect::<Vec<&str>>();
                if vec.len() == 1 {
                    cmd = exclude_cls_str.split(";");
                    vec = cmd.collect::<Vec<&str>>();
                }
                for value in vec {
                    if !value.trim().is_empty() {
                        if value.contains("-") {
                            cmd = value.split("-");
                            vec = cmd.collect::<Vec<&str>>();
                            let c = vec[0].trim().parse::<usize>().unwrap();
                            let d = vec[1].trim().parse::<usize>().unwrap();
                            for e in c..=d {
                                include_class_vals[e] = false;
                            }
                        } else if value.contains("...") {
                            cmd = value.split("...");
                            vec = cmd.collect::<Vec<&str>>();
                            let c = vec[0].trim().parse::<usize>().unwrap();
                            let d = vec[1].trim().parse::<usize>().unwrap();
                            for e in c..=d {
                                include_class_vals[e] = false;
                            }
                        } else {
                            let c = value.trim().parse::<usize>().unwrap();
                            include_class_vals[c] = false;
                        }
                    }
                }
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

        // now output the data
        let mut output = LasFile::initialize_using_file(&output_file, &input);
        output.header.system_id = "EXTRACTION".to_string();

        for i in 0..n_points {
            if include_class_vals[input[i].classification() as usize] {
                output.add_point_record(input.get_record(i));
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

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
