/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to calculate the relative stream power (*RSP*) index. This index is directly related
/// to the stream power if the assumption can be made that discharge is directly proportional to upslope
/// contributing area (*A<sub>s</sub>*; `--sca`). The index is calculated as:
///
/// > *RSP* = *A<sub>s</sub>*<sup>*p*</sup> &times; tan(&beta;)
///
/// where *A<sub>s</sub>* is the specific catchment area (i.e. the upslope contributing area per unit
/// contour length) estimated using one of the available flow accumulation algorithms; &beta; is the local
/// slope gradient in degrees (`--slope`); and, *p* (`--exponent`) is a user-defined exponent term that
/// controls the location-specific relation between contributing area and discharge. Notice that
/// *A<sub>s</sub>* must not be log-transformed prior to being used; *A<sub>s</sub>* is commonly
/// log-transformed to enhance visualization of the data. The slope raster can be created from the base
/// digital elevation model (DEM) using the `Slope` tool. The input images must have the same grid dimensions.
///
/// # Reference
/// Moore, I. D., Grayson, R. B., and Ladson, A. R. (1991). Digital terrain modelling:
/// a review of hydrological, geomorphological, and biological applications. *Hydrological
/// processes*, 5(1), 3-30.
///
/// # See Also
/// `SedimentTransportIndex`, `Slope`, `D8FlowAccumulation` `DInfFlowAccumulation`, `FD8FlowAccumulation`
pub struct StreamPowerIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl StreamPowerIndex {
    pub fn new() -> StreamPowerIndex {
        // public constructor
        let name = "StreamPowerIndex".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates the relative stream power index.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Specific Contributing Area (SCA) File".to_owned(),
            flags: vec!["--sca".to_owned()],
            description: "Input raster specific contributing area (SCA) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Slope File".to_owned(),
            flags: vec!["--slope".to_owned()],
            description: "Input raster slope file.".to_owned(),
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

        parameters.push(ToolParameter {
            name: "Specific Contributing Area (SCA) Exponent".to_owned(),
            flags: vec!["--exponent".to_owned()],
            description: "SCA exponent value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --sca='flow_accum.tif' --slope='slope.tif' -o=output.tif --exponent=1.1", short_exe, name).replace("*", &sep);

        StreamPowerIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for StreamPowerIndex {
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
        let mut sca_file = String::new();
        let mut slope_file = String::new();
        let mut output_file = String::new();
        let mut sca_exponent = 1.0;

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
            if flag_val == "-sca" {
                if keyval {
                    sca_file = vec[1].to_string();
                } else {
                    sca_file = args[i + 1].to_string();
                }
            } else if flag_val == "-slope" {
                if keyval {
                    slope_file = vec[1].to_string();
                } else {
                    slope_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-exponent" {
                if keyval {
                    sca_exponent = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    sca_exponent = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
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

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !sca_file.contains(&sep) && !sca_file.contains("/") {
            sca_file = format!("{}{}", working_directory, sca_file);
        }
        if !slope_file.contains(&sep) && !slope_file.contains("/") {
            slope_file = format!("{}{}", working_directory, slope_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let sca = Arc::new(Raster::new(&sca_file, "r")?);
        let slope = Arc::new(Raster::new(&slope_file, "r")?);

        let start = Instant::now();
        let rows = sca.configs.rows as isize;
        let columns = sca.configs.columns as isize;
        let sca_nodata = sca.configs.nodata;
        let slope_nodata = slope.configs.nodata;

        // make sure the input files have the same size
        if sca.configs.rows != slope.configs.rows || sca.configs.columns != slope.configs.columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        // calculate the number of downslope cells
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let sca = sca.clone();
            let slope = slope.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut sca_val: f64;
                let mut slope_val: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![sca_nodata; columns as usize];
                    for col in 0..columns {
                        sca_val = sca[(row, col)];
                        slope_val = slope[(row, col)];
                        if sca_val != sca_nodata && slope_val != slope_nodata {
                            data[col as usize] =
                                sca_val.powf(sca_exponent) * slope_val.to_radians().tan();
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &sca);
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
        output.configs.data_type = DataType::F32;
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.clip_display_min_max(1.0);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("SCA raster: {}", sca_file));
        output.add_metadata_entry(format!("Slope raster: {}", slope_file));
        output.add_metadata_entry(format!("SCA exponent: {}", sca_exponent));
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

        if sca.configs.maximum < 100.0 {
            println!("WARNING: The input SCA data layer contained only low values. It is likely that it has been
            log-transformed. This tool requires non-transformed SCA as an input.")
        }
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
