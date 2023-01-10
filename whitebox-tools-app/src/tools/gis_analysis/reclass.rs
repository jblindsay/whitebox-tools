/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/09/2017
Last Modified: 31/05/2021
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool creates a new raster in which the value of each grid cell is determined by an input raster (`--input`) and a
/// collection of user-defined classes. The user must specify the *New* value, the *From* value, and the *To Just Less Than*
/// value of each class triplet of the reclass string. Classes must be mutually exclusive, i.e. non-overlapping. For example:
///
/// > --reclass_vals='0.0;0.0;1.0;1.0;1.0;2.0'
///
/// The above reclass string assigns 0.0 to all grid cells in the input image with values from 0.0-1.0 and an output
/// value of 1.0 from to inputs from 1.0-2.0. Alternatively, if the `--assign_mode` flag is specified, `Reclass` will
/// operate in assign mode, using a reclass string composed of paired values:
///
/// > --reclass_vals='0.0;1.0;1.0;2.0'
///
/// Here, 0.0 is assigned to input grid cell values of 1.0 and 1.0 is output for all input cells with a value of 2.0. Users
/// may add the text strings *min* and *max* in the class definitions to stand in for the raster's minimum and maximum values.
/// Using *max* in a class triplet will change this class from *To Just Less Than* to *To Less Or Equal Than*.
/// For example:
///
/// > --reclass_vals='0.0;min;1.0;1.0;1.0;max'
///
/// Any values in the input raster that do not fall within one of the classes will be assigned its original value in the
/// output raster. NoData values in the input raster will be assigned NoData values in the output raster, unless NoData is
/// used in one of the user-defined reclass ranges (notice that it is valid to enter 'NoData' in these ranges).
///
/// # See Also
/// `ReclassEqualInterval`, `ReclassFromFile`
pub struct Reclass {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Reclass {
    /// public constructor
    pub fn new() -> Reclass {
        let name = "Reclass".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Reclassifies the values in a raster image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Reclass Values (new value; from value; to less than)".to_owned(), 
            flags: vec!["--reclass_vals".to_owned()], 
            description: "Reclassification triplet values (new value; from value; to less than), e.g. '0.0;0.0;1.0;1.0;1.0;2.0'".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Operate in assign mode? (i.e. Reclass data are pair values rather than triplets)".to_owned(), 
            flags: vec!["--assign_mode".to_owned()], 
            description: "Optional Boolean flag indicating whether to operate in assign mode, reclass_vals values are interpreted as new value; old value pairs.".to_owned(),
            parameter_type: ParameterType::Boolean,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --reclass_vals='0.0;0.0;1.0;1.0;1.0;2.0'
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --reclass_vals='10;1;20;2;30;3;40;4' --assign_mode ", short_exe, name).replace("*", &sep);

        Reclass {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Reclass {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut reclass_str = String::new();
        let mut assign_mode = false;

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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "-reclass_vals"
                || vec[0].to_lowercase() == "--reclass_vals"
            {
                reclass_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "-assign_mode"
                || vec[0].to_lowercase() == "--assign_mode"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    assign_mode = true;
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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;

        let mut v: Vec<&str> = reclass_str.split(";").collect();
        if v.len() < 2 {
            // delimiter can be a semicolon, comma, space, or tab.
            v = reclass_str.split(",").collect();
            if v.len() < 2 {
                v = reclass_str.split(" ").collect();
                if v.len() < 2 {
                    v = reclass_str.split("\t").collect();
                }
            }
        }
        let reclass_vals: Vec<f64> = v
            .iter()
            .map(|s| {
                if s.to_lowercase().contains("min") {
                    min_val
                } else if s.to_lowercase().contains("max") {
                    max_val + 0.1f64
                } else if s.to_lowercase().contains("nodata") {
                    nodata
                } else {
                    s.trim().parse().unwrap()
                }
            })
            .collect();
        if reclass_vals.len() % 3 != 0 && !assign_mode {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The reclass values string must include triplet values (new value; from value; to less than), e.g. '0.0;0.0;1.0;1.0;1.0;2.0'"));
        } else if reclass_vals.len() % 2 != 0 && assign_mode {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The reclass values string must include pair values (new value; old value), e.g. '1;10;2;20;3;30;4;40'"));
        }
        let num_ranges = match assign_mode {
            false => reclass_vals.len() / 3,
            true => reclass_vals.len() / 2,
        };
        let reclass_vals = Arc::new(reclass_vals);

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();

        let mut output = Raster::initialize_using_file(&output_file, &input);

        if !assign_mode {
            for tid in 0..num_procs {
                let input = input.clone();
                let reclass_vals = reclass_vals.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let mut prev_idx: usize = 0;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data: Vec<f64> = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input[(row, col)];
                            if z != nodata {
                                // This is a shortcut intended to take advantage of the inherent
                                // spatial autocorrelation in spatial distributions to speed up
                                // the search for the appropriate range bin.
                                if z >= reclass_vals[prev_idx * 3 + 1]
                                    && z < reclass_vals[prev_idx * 3 + 2]
                                {
                                    z = reclass_vals[prev_idx * 3];
                                } else {
                                    for a in 0..num_ranges {
                                        if z >= reclass_vals[a * 3 + 1]
                                            && z < reclass_vals[a * 3 + 2]
                                        {
                                            z = reclass_vals[a * 3];
                                            prev_idx = a;
                                            break;
                                        }
                                    }
                                }
                                data[col as usize] = z;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(row, data);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // assign_mode
            // create a hashmap to hold the assign values
            // the key is the old_value and the value is the new_value.
            /* Note: Rust doesn't support using HashMaps with floating-point keys because it is unsafe.
                NaN != NaN and due to rounding errors sometimes 0.1 ! = 0.1. To deal with this, we apply
                a multiplier of 10000 and convert to an i64.
            */
            let multiplier = 10000f64;
            let mut assign_map = HashMap::new();
            for a in 0..num_ranges {
                assign_map.insert(
                    (reclass_vals[a * 2 + 1] * multiplier).round() as i64,
                    reclass_vals[a * 2],
                );
            }
            let assign_map = Arc::new(assign_map);

            for tid in 0..num_procs {
                let input = input.clone();
                let assign_map = assign_map.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data: Vec<f64> = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z = input[(row, col)];
                            if z != nodata {
                                // is z in the hashmap?
                                if assign_map.contains_key(&((z * multiplier).round() as i64)) {
                                    z = *assign_map
                                        .get(&((z * multiplier).round() as i64))
                                        .unwrap();
                                }
                                data[col as usize] = z;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(row, data);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Reclass values: {:?}", reclass_vals));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
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
