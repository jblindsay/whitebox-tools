/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/07/2017
Last Modified: 21/01/2018
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

/// This tool can be used to calculate the topographic wetness index, commonly used in the TOPMODEL rainfall-runoff framework.
/// The index describes the propensity for a site to be saturated to the surface given its contributing area and local slope
/// characteristics. It is calculated as:
///
/// > WI = Ln(As / tan(Slope))
///
/// Where `As` is the specific catchment area (i.e. the upslope contributing area per unit contour length) estimated using one of
/// the available flow accumulation algorithms in the Hydrological Analysis toolbox. Notice that `As` must not be log-transformed
/// prior to being used; log-transformation of `As` is a common practice when visualizing the data. The slope image should be
/// measured in degrees and can be created from the base digital elevation model (DEM) using the `Slope` tool. Grid cells with a
/// slope of zero will be assigned **NoData** in the output image to compensate for the fact that division by zero is infinity.
/// These very flat sites likely coincide with the wettest parts of the landscape. The input images must have the same grid dimensions.
///
/// Grid cells possessing the NoData value in either of the input images are assigned NoData value in the output image. The output
/// raster is of the float data type and continuous data scale.
///
/// See Also
/// `Slope`, `D8FlowAccumulation`, `DInfFlowAccumulation`, `FD8FlowAccumulation`, `BreachDepressionsLeastCost`
pub struct WetnessIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl WetnessIndex {
    pub fn new() -> WetnessIndex {
        // public constructor
        let name = "WetnessIndex".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the topographic wetness index, Ln(A / tan(slope)).".to_string();

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
            description: "Input raster slope file (in degrees).".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --sca='flow_accum.tif' --slope='slope.tif' -o=output.tif", short_exe, name).replace("*", &sep);

        WetnessIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for WetnessIndex {
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
            if vec[0].to_lowercase() == "-sca" || vec[0].to_lowercase() == "--sca" {
                if keyval {
                    sca_file = vec[1].to_string();
                } else {
                    sca_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
                if keyval {
                    slope_file = vec[1].to_string();
                } else {
                    slope_file = args[i + 1].to_string();
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
                            if slope_val != 0f64 {
                                data[col as usize] = (sca_val / slope_val.to_radians().tan()).ln();
                            }
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
