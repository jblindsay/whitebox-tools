/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 13/05/2020
Last Modified: 13/05/2020
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use std;
use std::io::{Error, ErrorKind};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{env, fs, path, thread};

/// This tool can be used to convert one or more *zLidar* files ('*.zlidar') files into the *LAS*
/// LiDAR data format. [zLidar](https://jblindsay.github.io/zLidar_spec/intro.html) files are a compressed
/// form of the LAS data format. The tool takes a list of input LAS files (`--inputs`). If `--inputs`
/// is unspecified, the tool will use all ZLidar files contained within the working directory
/// as the tool inputs. The user may also specify an optional output directory `--outdir`.
/// If this parameter is unspecified, each output LAS file will be written to the same
/// directory as the input files.
///
/// # See Also
/// `LasToZlidar`, `AsciiToLas`
pub struct ZlidarToLas {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ZlidarToLas {
    pub fn new() -> ZlidarToLas {
        // public constructor
        let name = "ZlidarToLas".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Converts one or more zlidar files into the LAS data format.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input ZLidar Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input ZLidar files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Directory".to_owned(),
            flags: vec!["--outdir".to_owned()],
            description: "Output directory into which zlidar files are created. If unspecified, it is assumed to be the same as the inputs."
                .to_owned(),
            parameter_type: ParameterType::Directory,
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.zlidar, file2.zlidar, file3.zlidar\"",
            short_exe, name
        )
        .replace("*", &sep);

        ZlidarToLas {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ZlidarToLas {
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
        let mut output_directory: String = String::new();

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
            } else if flag_val == "-outdir" {
                output_directory = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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

        let sep = std::path::MAIN_SEPARATOR;

        let start = Instant::now();

        if !output_directory.is_empty() && !output_directory.ends_with(sep) {
            output_directory = format!("{}{}", output_directory, sep);
        }

        let mut inputs: Vec<String> = vec![];
        if input_files.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            if std::path::Path::new(&working_directory).is_dir() {
                for entry in fs::read_dir(working_directory.clone())? {
                    let s = entry?
                        .path()
                        .into_os_string()
                        .to_str()
                        .expect("Error reading path string")
                        .to_string();
                    if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("The input directory ({}) is incorrect.", working_directory),
                ));
            }
        } else {
            let mut cmd = input_files.split(";");
            inputs = cmd
                .collect::<Vec<&str>>()
                .iter()
                .map(|x| String::from(x.trim()))
                .collect::<Vec<String>>();
            if inputs.len() == 1 {
                cmd = input_files.split(",");
                inputs = cmd
                    .collect::<Vec<&str>>()
                    .iter()
                    .map(|x| String::from(x.trim()))
                    .collect::<Vec<String>>();
            }
        }

        for a in (0..inputs.len()).rev() {
            if !inputs[a].to_lowercase().ends_with(".zlidar") {
                if verbose {
                    println!("WARNGING: Non-zLidar input file detected. This file will be excluded from the analysis.");
                }
                inputs.remove(a);
            }
        }

        let num_files = inputs.len();
        if num_files == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "No valid input files were detected. Analysis will not proceed.",
            ));
        }

        let inputs = Arc::new(inputs);
        let working_directory = Arc::new(working_directory.to_owned());
        let output_directory = Arc::new(output_directory.clone());
        let tile_list = Arc::new(Mutex::new(0..num_files));
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for _ in 0..num_procs {
            let inputs = inputs.clone();
            let tile_list = tile_list.clone();
            let working_directory = working_directory.clone();
            let output_directory = output_directory.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut k = 0;
                let mut progress: usize;
                let mut old_progress: usize = 1;
                while k < num_files {
                    // Get the next tile up for examination
                    k = match tile_list.lock().unwrap().next() {
                        Some(val) => val,
                        None => break, // There are no more tiles to examine
                    };

                    let mut input_file = inputs[k].replace("\"", "").clone();
                    if !input_file.is_empty() {
                        if !input_file.contains(sep) && !input_file.contains("/") {
                            input_file = format!("{}{}", working_directory, input_file);
                        }

                        let input: LasFile = match LasFile::new(&input_file, "r") {
                            Ok(lf) => lf,
                            Err(_) => {
                                panic!("Error reading file: {}", input_file);
                            }
                        };

                        let short_filename = input.get_short_filename();
                        let file_extension = get_file_extension(&input_file);
                        if file_extension.to_lowercase() != "zlidar" {
                            panic!("All input files should be of zlidar format.")
                        }

                        let output_file = if output_directory.is_empty() {
                            input_file.replace(&format!(".{}", file_extension), ".las")
                        } else {
                            format!("{}{}.las", output_directory, short_filename)
                        };
                        let mut output = LasFile::initialize_using_file(&output_file, &input);

                        let n_points = input.header.number_of_points as usize;

                        for p in 0..n_points {
                            let pr = input.get_record(p);
                            output.add_point_record(pr);
                            if verbose && num_files == 1 {
                                progress =
                                    (100.0_f64 * (p + 1) as f64 / (n_points - 1) as f64) as usize;
                                if progress != old_progress {
                                    println!("Creating output: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }
                        let _ = match output.write() {
                            Ok(_) => {
                                // do nothing
                            }
                            Err(e) => println!("error while writing: {:?}", e),
                        };
                        tx.send(short_filename.clone()).unwrap();
                    } else {
                        tx.send(format!("Empty file name for tile {}.", k)).unwrap();
                    }
                }
            });
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;
        for tile in 0..num_files {
            let file_nm = rx.recv().expect("Error receiving data from thread.");
            if verbose && !file_nm.contains("Empty") && num_files > 1 && tile < 99 {
                println!("Completed conversion of {}", file_nm);
            } else if verbose && tile == 99 {
                println!("Completed conversion of {}", file_nm);
                println!("...");
            } else if file_nm.to_lowercase().contains("empty file name") {
                println!("{}", file_nm);
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
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
