/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/06/2017
Last Modified: 29/08/2018
License: MIT
*/

use crate::lidar::*;
use crate::tools::*;
use std;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to merge multiple LiDAR LAS files into a single output LAS file. Due to their large size,
/// LiDAR data sets are often tiled into smaller, non-overlapping tiles. Sometimes it is more convenient to combine
/// multiple tiles together for data processing and `LidarJoin` can be used for this purpose.
///
/// # See Also
/// `LidarTile`
pub struct LidarJoin {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarJoin {
    pub fn new() -> LidarJoin {
        // public constructor
        let name = "LidarJoin".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Joins multiple LiDAR (LAS) files into a single LAS file.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input LiDAR files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Lidar),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.las, file2.las, file3.las\" -o=outfile.las\"", short_exe, name).replace("*", &sep);

        LidarJoin {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarJoin {
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
        let mut input_files: String = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-inputs" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;

        if !output_file.contains(sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut output: LasFile = LasFile::new(&output_file, "w")?;

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let mut i = 0;
        let num_files = vec.len();
        let mut file_format = -1i32;
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                let input = match LasFile::new(&input_file, "r") {
                    Ok(lf) => lf,
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("No such file or directory ({})", input_file),
                        ))
                    }
                };

                if file_format == -1 {
                    file_format = input.header.point_format as i32;
                } else {
                    if input.header.point_format as i32 != file_format {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "All input files must be of the same LAS Point Format.",
                        ));
                    }
                }

                if i == 0 {
                    output = LasFile::initialize_using_file(&output_file, &input);
                }

                let n_points = input.header.number_of_points as usize;

                let mut pr: LidarPointRecord;
                for i in 0..n_points {
                    pr = input.get_record(i);
                    output.add_point_record(pr);
                }
            }
            i += 1;
            if verbose {
                println!("Adding file: {} of {}", i, num_files);
            }
        }

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
        // if verbose {
        //     println!(
        //         "{}",
        //         &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
        //     );
        // }

        Ok(())
    }
}
