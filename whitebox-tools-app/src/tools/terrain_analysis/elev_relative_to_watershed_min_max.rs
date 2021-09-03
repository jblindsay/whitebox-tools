/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/07/2017
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::Raster;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to express the elevation of a grid cell in a digital elevation model (DEM) as a percentage
/// of the relief between the watershed minimum and maximum values. As such, it provides a basic
/// measure of relative topographic position. The user must specify the names of DEM (`--dem`) and watersheds (`--watersheds`)
/// raster files.
///
/// # See Also
/// `ElevRelativeToMinMax`, `ElevationAboveStream`, `ElevAbovePit`
pub struct ElevRelativeToWatershedMinMax {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ElevRelativeToWatershedMinMax {
    pub fn new() -> ElevRelativeToWatershedMinMax {
        // public constructor
        let name = "ElevRelativeToWatershedMinMax".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates the elevation of a location relative to the minimum and maximum elevations in a watershed.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Watersheds File".to_owned(),
            flags: vec!["--watersheds".to_owned()],
            description: "Input raster watersheds file.".to_owned(),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --watersheds=watershed.tif -o=output.tif", short_exe, name).replace("*", &sep);

        ElevRelativeToWatershedMinMax {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ElevRelativeToWatershedMinMax {
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
        let mut input_file = String::new();
        let mut watersheds_file = String::new();
        let mut output_file = String::new();

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
            if vec[0].to_lowercase() == "-i"
                || vec[0].to_lowercase() == "--input"
                || vec[0].to_lowercase() == "--dem"
            {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-watersheds"
                || vec[0].to_lowercase() == "--watersheds"
            {
                if keyval {
                    watersheds_file = vec[1].to_string();
                } else {
                    watersheds_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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
        if !watersheds_file.contains(&sep) && !watersheds_file.contains("/") {
            watersheds_file = format!("{}{}", working_directory, watersheds_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        // let min_val = input.configs.minimum;

        let watersheds = Arc::new(Raster::new(&watersheds_file, "r")?);
        let watershed_nodata = watersheds.configs.nodata;

        // make sure the input files have the same size
        if watersheds.configs.rows != input.configs.rows
            || watersheds.configs.columns != input.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);

        let min_watershed = watersheds.configs.minimum;
        let max_watershed = watersheds.configs.maximum;
        let range_watersheds = max_watershed - min_watershed;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let watersheds = watersheds.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut watershed_min_vals = vec![f64::INFINITY; range_watersheds as usize + 1];
                let mut watershed_max_vals = vec![f64::NEG_INFINITY; range_watersheds as usize + 1];
                let mut z: f64;
                let mut watershed: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        z = input[(row, col)];
                        watershed = watersheds[(row, col)];
                        if z != nodata && watershed != watershed_nodata {
                            watershed -= min_watershed;
                            if z < watershed_min_vals[watershed as usize] {
                                watershed_min_vals[watershed as usize] = z;
                            }
                            if z > watershed_max_vals[watershed as usize] {
                                watershed_max_vals[watershed as usize] = z;
                            }
                        }
                    }
                }
                tx.send((watershed_min_vals, watershed_max_vals)).unwrap();
            });
        }

        let mut watershed_min_vals = vec![f64::INFINITY; range_watersheds as usize + 1];
        let mut watershed_max_vals = vec![f64::NEG_INFINITY; range_watersheds as usize + 1];
        for tid in 0..num_procs {
            let (mins, maxs) = rx.recv().expect("Error receiving data from thread.");
            for i in 0..mins.len() {
                //(range_watersheds as usize+1) {
                if mins[i] != f64::INFINITY && mins[i] < watershed_min_vals[i] {
                    watershed_min_vals[i] = mins[i];
                }
                if maxs[i] != f64::NEG_INFINITY && maxs[i] > watershed_max_vals[i] {
                    watershed_max_vals[i] = maxs[i];
                }
            }
            if verbose {
                progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let watersheds = watersheds.clone();
            let watershed_min_vals = watershed_min_vals.clone();
            let watershed_max_vals = watershed_max_vals.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut watershed: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        watershed = watersheds[(row, col)];
                        if z != nodata && watershed != watershed_nodata {
                            watershed -= min_watershed;
                            data[col as usize] = (z - watershed_min_vals[watershed as usize])
                                / (watershed_max_vals[watershed as usize]
                                    - watershed_min_vals[watershed as usize])
                                * 100f64;
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Watersheds file: {}", watersheds_file));
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
