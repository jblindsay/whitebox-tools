/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/06/2017
Last Modified: 13/02/2020
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool is used to generate a flow pointer grid (i.e. flow direction) using the D-infinity
/// (Tarboton, 1997) algorithm. Dinf is a multiple-flow-direction (MFD) method because the flow
/// entering each grid cell is routed one or two downslope neighbours, i.e. flow divergence is permitted.
/// The user must specify the name of a digital elevation model (DEM; `--dem`) that has been hydrologically
/// corrected to remove all spurious depressions and flat areas (`BreachDepressions`, `FillDepressions`).
/// DEM pre-processing is usually achieved using the `BreachDepressions` or `FillDepressions` tool1. Flow
/// directions are specified in the output flow-pointer grid (`--output`) as azimuth degrees measured from
/// north, i.e. any value between 0 and 360 degrees is possible. A pointer value of -1 is used to designate
/// a grid cell with no flow-pointer. This occurs when a grid cell has no downslope neighbour, i.e. a pit
/// cell or topographic depression. Like aspect grids, Dinf flow-pointer grids are best visualized using
/// a circular greyscale palette.
///
/// Grid cells possessing the NoData value in the input DEM are assigned the NoData value in the output
/// image. The output raster is of the float data type and continuous data scale.
///
/// # Reference
/// Tarboton, D. G. (1997). A new method for the determination of flow directions and upslope areas in
/// grid digital elevation models. Water resources research, 33(2), 309-319.
///
/// # See Also
/// `DInfFlowAccumulation`, `BreachDepressions`, `FillDepressions`
pub struct DInfPointer {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DInfPointer {
    pub fn new() -> DInfPointer {
        // public constructor
        let name = "DInfPointer".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates a D-infinity flow pointer (flow direction) raster from an input DEM."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif",
            short_exe, name
        )
        .replace("*", &sep);

        DInfPointer {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DInfPointer {
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
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
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
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // calculate the flow directions
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let grid_res = (cell_size_x + cell_size_y) / 2.0;
                let mut dir: f64;
                let mut max_slope: f64;
                let mut e0: f64;
                let mut af: f64;
                let mut ac: f64;
                let (mut e1, mut r, mut s1, mut s2, mut s, mut e2): (f64, f64, f64, f64, f64, f64);

                let ac_vals = [0f64, 1f64, 1f64, 2f64, 2f64, 3f64, 3f64, 4f64];
                let af_vals = [1f64, -1f64, 1f64, -1f64, 1f64, -1f64, 1f64, -1f64];

                let e1_col = [1, 0, 0, -1, -1, 0, 0, 1];
                let e1_row = [0, -1, -1, 0, 0, 1, 1, 0];

                let e2_col = [1, 1, -1, -1, -1, -1, 1, 1];
                let e2_row = [-1, -1, -1, -1, 1, 1, 1, 1];

                let atanof1 = 1.0f64.atan();

                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                const HALF_PI: f64 = PI / 2f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        e0 = input[(row, col)];
                        if e0 != nodata {
                            dir = 360.0;
                            max_slope = f64::MIN;
                            neighbouring_nodata = false;
                            for i in 0..8 {
                                ac = ac_vals[i];
                                af = af_vals[i];
                                e1 = input[(row + e1_row[i], col + e1_col[i])];
                                e2 = input[(row + e2_row[i], col + e2_col[i])];
                                if e1 != nodata && e2 != nodata {
                                    if e0 > e1 && e0 > e2 {
                                        s1 = (e0 - e1) / grid_res;
                                        s2 = (e1 - e2) / grid_res;
                                        r = if s1 != 0f64 {
                                            (s2 / s1).atan()
                                        } else {
                                            PI / 2.0
                                        };
                                        s = (s1 * s1 + s2 * s2).sqrt();
                                        if s1 < 0.0 && s2 < 0.0 {
                                            s *= -1.0;
                                        }
                                        if s1 < 0.0 && s2 == 0.0 {
                                            s *= -1.0;
                                        }
                                        if s1 == 0.0 && s2 < 0.0 {
                                            s *= -1.0;
                                        }
                                        if r < 0.0 || r > atanof1 {
                                            if r < 0.0 {
                                                r = 0.0;
                                                s = s1;
                                            } else {
                                                r = atanof1;
                                                s = (e0 - e2) / diag_cell_size;
                                            }
                                        }
                                        if s >= max_slope && s != 0.00001 {
                                            max_slope = s;
                                            dir = af * r + ac * HALF_PI;
                                        }
                                    } else if e0 > e1 || e0 > e2 {
                                        if e0 > e1 {
                                            r = 0.0;
                                            s = (e0 - e1) / grid_res;
                                        } else {
                                            r = atanof1;
                                            s = (e0 - e2) / diag_cell_size;
                                        }
                                        if s >= max_slope && s != 0.00001 {
                                            max_slope = s;
                                            dir = af * r + ac * HALF_PI;
                                        }
                                    }
                                } else {
                                    neighbouring_nodata = true;
                                }
                            }

                            if max_slope > 0f64 {
                                // dir = Math.round((dir * (180 / Math.PI)) * 10) / 10;
                                dir = 360.0 - dir.to_degrees() + 90.0;
                                if dir > 360.0 {
                                    dir = dir - 360.0;
                                }
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1f64;
                                if !neighbouring_nodata {
                                    interior_pit_found = true;
                                }
                            }
                        } else {
                            data[col as usize] = -1f64;
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);
            if pit {
                interior_pit_found = true;
            }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        output.configs.palette = "circular_bw.plt".to_string();
        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
        if interior_pit_found {
            println!("**********************************************************************************");
            println!("WARNING: Interior pit cells were found within the input DEM. It is likely that the 
            DEM needs to be processed to remove topographic depressions and flats prior to
            running this tool.");
            println!("**********************************************************************************");
        }

        Ok(())
    }
}
