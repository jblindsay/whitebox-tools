/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/07/2017
Last Modified: 12/08/2019
License: MIT
*/

use crate::lidar::*;
use crate::tools::*;
use std;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to convert one or more LAS file, containing LiDAR data, into ASCII files. The user must
/// specify the name(s) of the input LAS file(s) (`--inputs`). Each input file will have a coorespondingly named
/// output file with a `.csv` file extension. CSV files are comma separated value files and contain tabular data
/// with each column cooresponding to a field in the table and each row a point value. Fields are separated by
/// commas in the ASCII formated file. The output point data, each on a seperate line, will take the format:
///
/// ```
/// X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE
/// ```
///
/// If the LAS file has a point format that contains RGB data, the final three columns will contain the RED,
/// GREEN, and BLUE values respectively. Use the `AsciiToLas` tool to convert a text file containing LiDAR
/// point data into a LAS file.
///
/// # See Also
/// `AsciiToLas`
pub struct LasToAscii {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LasToAscii {
    pub fn new() -> LasToAscii {
        // public constructor
        let name = "LasToAscii".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Converts one or more LAS files into ASCII text files.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input LiDAR files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Lidar),
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

        LasToAscii {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LasToAscii {
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
            if flag_val == "-i" || flag_val == "-inputs" || flag_val == "-input" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let mut i = 1;
        let num_files = vec.len();
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
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

                let has_rgb = input.has_rgb();
                let mut rgb: ColourData;
                let file_extension = get_file_extension(&input_file);
                let output_file = input_file.replace(&format!(".{}", file_extension), ".csv");
                let f = File::create(output_file)?;
                let mut writer = BufWriter::new(f);

                let n_points = input.header.number_of_points as usize;

                if !has_rgb {
                    writer.write_all(
                        "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE\n".as_bytes(),
                    )?;
                } else {
                    writer.write_all(
                        "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE,RED,GREEN,BLUE\n"
                            .as_bytes(),
                    )?;
                }
                for k in 0..n_points {
                    let pd = input[k];
                    let s = if !has_rgb {
                        format!(
                            "{},{},{},{},{},{},{},{}\n",
                            pd.x,
                            pd.y,
                            pd.z,
                            pd.intensity,
                            pd.classification(),
                            pd.return_number(),
                            pd.number_of_returns(),
                            pd.scan_angle
                        )
                    } else {
                        rgb = match input.get_rgb(k) {
                            Ok(v) => v,
                            Err(_) => {
                                return Err(Error::new(
                                    ErrorKind::NotFound,
                                    "RGB data not read correctly in LAS file.",
                                ))
                            }
                        };
                        format!(
                            "{},{},{},{},{},{},{},{},{},{},{}\n",
                            pd.x,
                            pd.y,
                            pd.z,
                            pd.intensity,
                            pd.classification(),
                            pd.return_number(),
                            pd.number_of_returns(),
                            pd.scan_angle,
                            rgb.red,
                            rgb.green,
                            rgb.blue
                        )
                    };
                    writer.write_all(s.as_bytes())?;

                    if verbose {
                        progress = (100.0_f64 * k as f64 / (n_points - 1) as f64) as usize;
                        if progress != old_progress {
                            if num_files > 1 {
                                println!("Creating file: {} of {}: {}%", i, num_files, progress);
                            } else {
                                println!("Progress: {}%", progress);
                            }
                            old_progress = progress;
                        }
                    }
                }
                let _ = writer.flush();
            }
            i += 1;
        }

        if verbose {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}

/// Returns the file extension.
pub fn get_file_extension(file_name: &str) -> String {
    let file_path = std::path::Path::new(file_name);
    let extension = file_path.extension().unwrap();
    let e = extension.to_str().unwrap();
    e.to_string()
}
