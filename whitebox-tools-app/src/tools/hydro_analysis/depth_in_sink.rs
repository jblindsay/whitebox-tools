/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/07/2017
Last Modified: 05/12/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::cmp::Ordering;
use std::cmp::Ordering::Equal;
use std::collections::{BinaryHeap, VecDeque};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool measures the depth that each grid cell in an input (`--dem`) raster digital elevation model (DEM)
/// lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like
/// landscape feature, which is characterized by interior drainage and groundwater recharge. The `DepthInSink` tool
/// operates by differencing a filled DEM, using the same depression filling method as `FillDepressions`, and the
/// original surface model.
///
/// In addition to the names of the input DEM (`--dem`) and the output raster (`--output`), the user must specify
/// whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be
/// set to 0.0 (`--zero_background`) Without this optional parameter specified, the tool will use the NoData value
/// as the background value.
///
/// # Reference
/// Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological
/// Modelling, 138(1-3), 247-254.
///
/// # See Also
/// `FillDepressions`
pub struct DepthInSink {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl DepthInSink {
    pub fn new() -> DepthInSink {
        // public constructor
        let name = "DepthInSink".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Measures the depth of sinks (depressions) in a DEM.".to_string();

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
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Should a background value of zero be used?".to_owned(),
            flags: vec!["--zero_background".to_owned()],
            description: "Flag indicating whether the background value of zero should be used."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --zero_background", short_exe, name).replace("*", &sep);

        DepthInSink {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for DepthInSink {
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
        let mut zero_background = false;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                input_file = if keyval {
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
            } else if flag_val == "-zero_background" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    zero_background = true;
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

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut filled_dem = input.get_data_as_array2d();

        let (mut col, mut row): (isize, isize);
        let (mut rn, mut cn): (isize, isize);
        let (mut z, mut zn): (f64, f64);
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        // Find pit cells. This step is parallelized.
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let filled_dem2 = Arc::new(filled_dem);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let filled_dem2 = filled_dem2.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut zn: f64;
                let mut flag: bool;
                let mut pits = vec![];
                for row in (1..rows - 1).filter(|r| r % num_procs == tid) {
                    for col in 1..columns - 1 {
                        z = filled_dem2.get_value(row, col);
                        if z != nodata {
                            flag = true;
                            for n in 0..8 {
                                zn = filled_dem2.get_value(row + dy[n], col + dx[n]);
                                if zn < z || zn == nodata {
                                    // It either has a lower neighbour or is an edge cell.
                                    flag = false;
                                    break;
                                }
                            }
                            if flag {
                                // it's a cell with undefined flow
                                pits.push((row, col, z));
                            }
                        }
                    }
                }
                tx.send(pits).unwrap();
            });
        }

        let mut undefined_flow_cells = vec![];
        for p in 0..num_procs {
            let mut pits = rx.recv().expect("Error receiving data from thread.");
            undefined_flow_cells.append(&mut pits);

            if verbose {
                progress = (100.0_f64 * (p + 1) as f64 / num_procs as f64) as usize;
                if progress != old_progress {
                    println!("Finding pit cells: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut input_configs = input.configs.clone();

        filled_dem = match Arc::try_unwrap(filled_dem2) {
            Ok(val) => val,
            Err(_) => panic!("Error unwrapping 'filled_dem'"),
        };

        let num_deps = undefined_flow_cells.len();

        // Now we need to perform an in-place depression filling
        let mut minheap = BinaryHeap::new();
        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let mut flats: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let mut possible_outlets = vec![];
        // solve from highest to lowest
        undefined_flow_cells.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Equal));
        let mut pit_id = 1;
        let mut flag: bool;
        while let Some(cell) = undefined_flow_cells.pop() {
            row = cell.0;
            col = cell.1;
            if flats.get_value(row, col) != 1 {
                // if it's already in a solved site, don't do it a second time.
                // First there is a priority region-growing operation to find the outlets.
                z = filled_dem.get_value(row, col);
                minheap.clear();
                minheap.push(GridCell {
                    row: row,
                    column: col,
                    priority: z,
                });
                visited.set_value(row, col, 1);
                let mut outlet_found = false;
                let mut outlet_z = f64::INFINITY;
                let mut queue = VecDeque::new();
                while let Some(cell2) = minheap.pop() {
                    z = cell2.priority;
                    if outlet_found && z > outlet_z {
                        break;
                    }
                    if !outlet_found {
                        for n in 0..8 {
                            cn = cell2.column + dx[n];
                            rn = cell2.row + dy[n];
                            if visited.get_value(rn, cn) == 0 {
                                zn = filled_dem.get_value(rn, cn);
                                if !outlet_found {
                                    if zn >= z && zn != nodata {
                                        minheap.push(GridCell {
                                            row: rn,
                                            column: cn,
                                            priority: zn,
                                        });
                                        visited.set_value(rn, cn, 1);
                                    } else if zn != nodata {
                                        // zn < z
                                        // 'cell' has a lower neighbour that hasn't already passed through minheap.
                                        // Therefore, 'cell' is a pour point cell.
                                        outlet_found = true;
                                        outlet_z = z;
                                        queue.push_back((cell2.row, cell2.column));
                                        possible_outlets.push((cell2.row, cell2.column));
                                    }
                                } else if zn == outlet_z {
                                    // We've found the outlet but are still looking for additional outlets.
                                    minheap.push(GridCell {
                                        row: rn,
                                        column: cn,
                                        priority: zn,
                                    });
                                    visited.set_value(rn, cn, 1);
                                }
                            }
                        }
                    } else {
                        if z == outlet_z {
                            flag = false;
                            for n in 0..8 {
                                cn = cell2.column + dx[n];
                                rn = cell2.row + dy[n];
                                if visited.get_value(rn, cn) == 0 {
                                    zn = filled_dem.get_value(rn, cn);
                                    if zn < z {
                                        flag = true;
                                    } else if zn == outlet_z {
                                        minheap.push(GridCell {
                                            row: rn,
                                            column: cn,
                                            priority: zn,
                                        });
                                        visited.set_value(rn, cn, 1);
                                    }
                                }
                            }
                            if flag {
                                // it's an outlet
                                queue.push_back((cell2.row, cell2.column));
                                possible_outlets.push((cell2.row, cell2.column));
                            } else {
                                visited.set_value(cell2.row, cell2.column, 1);
                            }
                        }
                    }
                }

                // Now that we have the outlets, raise the interior of the depression
                if outlet_found {
                    while let Some(cell2) = queue.pop_front() {
                        for n in 0..8 {
                            rn = cell2.0 + dy[n];
                            cn = cell2.1 + dx[n];
                            if visited.get_value(rn, cn) == 1 {
                                visited.set_value(rn, cn, 0);
                                queue.push_back((rn, cn));
                                z = filled_dem.get_value(rn, cn);
                                if z < outlet_z {
                                    filled_dem.set_value(rn, cn, outlet_z);
                                    flats.set_value(rn, cn, 1);
                                } else if z == outlet_z {
                                    flats.set_value(rn, cn, 1);
                                }
                            }
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * pit_id as f64 / num_deps as f64) as usize;
                if progress != old_progress {
                    println!("Finding depressions: {}%", progress);
                    old_progress = progress;
                }
            }
            pit_id += 1;
        }

        drop(visited);

        input_configs.nodata = -32768f64;
        let mut output = Raster::initialize_using_config(&output_file, &input_configs);
        if zero_background {
            output.reinitialize_values(0f64);
        }
        output.configs.data_type = DataType::F32;
        let num_outlets = possible_outlets.len();
        let mut diff: f64;
        while let Some(cell) = possible_outlets.pop() {
            if flats.get_value(cell.0, cell.1) == 1 {
                z = filled_dem.get_value(cell.0, cell.1);
                output.set_value(cell.0, cell.1, 0f64);
                let mut queue = VecDeque::new();
                flats.set_value(cell.0, cell.1, 0);
                queue.push_back((cell.0, cell.1));
                while let Some(cell2) = queue.pop_front() {
                    for n in 0..8 {
                        rn = cell2.0 + dy[n];
                        cn = cell2.1 + dx[n];
                        if flats.get_value(rn, cn) == 1 {
                            if filled_dem.get_value(rn, cn) == z {
                                flats.set_value(rn, cn, 0);
                                diff = z - input.get_value(rn, cn);
                                output.set_value(rn, cn, diff);
                                queue.push_back((rn, cn));
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (1.0 - possible_outlets.len() as f64 / num_outlets as f64))
                    as usize;
                if progress != old_progress {
                    println!("Estimating depths: {}%", progress);
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

#[derive(PartialEq, Debug)]
struct GridCell {
    row: isize,
    column: isize,
    priority: f64,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

// /*
// This tool is part of the WhiteboxTools geospatial analysis library.
// Authors: Dr. John Lindsay
// Created: 11/07/2017
// Last Modified: 18/10/2019
// License: MIT
// */
// use crate::raster::*;
// use crate::tools::*;
// use std::cmp::Ordering;
// use std::collections::BinaryHeap;
// use std::collections::VecDeque;
// use std::env;
// use std::f64;
// use std::i32;
// use std::io::{Error, ErrorKind};
// use std::path;

// /// This tool measures the depth that each grid cell in an input (`--dem`) raster digital elevation model (DEM)
// /// lies within a sink feature, i.e. a closed topographic depression. A sink, or depression, is a bowl-like
// /// landscape feature, which is characterized by interior drainage and groundwater recharge. The `DepthInSink` tool
// /// operates by differencing a filled DEM, using the same depression filling method as `FillDepressions`, and the
// /// original surface model.
// ///
// /// In addition to the names of the input DEM (`--dem`) and the output raster (`--output`), the user must specify
// /// whether the background value (i.e. the value assigned to grid cells that are not contained within sinks) should be
// /// set to 0.0 (`--zero_background`) Without this optional parameter specified, the tool will use the NoData value
// /// as the background value.
// ///
// /// # Reference
// /// Antonić, O., Hatic, D., & Pernar, R. (2001). DEM-based depth in sink as an environmental estimator. Ecological
// /// Modelling, 138(1-3), 247-254.
// ///
// /// # See Also
// /// `FillDepressions`
// pub struct DepthInSink {
//     name: String,
//     description: String,
//     toolbox: String,
//     parameters: Vec<ToolParameter>,
//     example_usage: String,
// }

// impl DepthInSink {
//     pub fn new() -> DepthInSink {
//         // public constructor
//         let name = "DepthInSink".to_string();
//         let toolbox = "Hydrological Analysis".to_string();
//         let description = "Measures the depth of sinks (depressions) in a DEM.".to_string();

//         let mut parameters = vec![];
//         parameters.push(ToolParameter {
//             name: "Input DEM File".to_owned(),
//             flags: vec!["-i".to_owned(), "--dem".to_owned()],
//             description: "Input raster DEM file.".to_owned(),
//             parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
//             default_value: None,
//             optional: false,
//         });

//         parameters.push(ToolParameter {
//             name: "Output File".to_owned(),
//             flags: vec!["-o".to_owned(), "--output".to_owned()],
//             description: "Output raster file.".to_owned(),
//             parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
//             default_value: None,
//             optional: false,
//         });

//         parameters.push(ToolParameter {
//             name: "Should a background value of zero be used?".to_owned(),
//             flags: vec!["--zero_background".to_owned()],
//             description: "Flag indicating whether the background value of zero should be used."
//                 .to_owned(),
//             parameter_type: ParameterType::Boolean,
//             default_value: None,
//             optional: true,
//         });

//         let sep: String = path::MAIN_SEPARATOR.to_string();
//         let p = format!("{}", env::current_dir().unwrap().display());
//         let e = format!("{}", env::current_exe().unwrap().display());
//         let mut short_exe = e
//             .replace(&p, "")
//             .replace(".exe", "")
//             .replace(".", "")
//             .replace(&sep, "");
//         if e.contains(".exe") {
//             short_exe += ".exe";
//         }
//         let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --zero_background", short_exe, name).replace("*", &sep);

//         DepthInSink {
//             name: name,
//             description: description,
//             toolbox: toolbox,
//             parameters: parameters,
//             example_usage: usage,
//         }
//     }
// }

// impl WhiteboxTool for DepthInSink {
//     fn get_source_file(&self) -> String {
//         String::from(file!())
//     }

//     fn get_tool_name(&self) -> String {
//         self.name.clone()
//     }

//     fn get_tool_description(&self) -> String {
//         self.description.clone()
//     }

//     fn get_tool_parameters(&self) -> String {
//         match serde_json::to_string(&self.parameters) {
//             Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
//             Err(err) => return format!("{:?}", err),
//         }
//     }

//     fn get_example_usage(&self) -> String {
//         self.example_usage.clone()
//     }

//     fn get_toolbox(&self) -> String {
//         self.toolbox.clone()
//     }

//     fn run<'a>(
//         &self,
//         args: Vec<String>,
//         working_directory: &'a str,
//         verbose: bool,
//     ) -> Result<(), Error> {
//         let mut input_file = String::new();
//         let mut output_file = String::new();
//         let mut zero_background = false;

//         if args.len() == 0 {
//             return Err(Error::new(
//                 ErrorKind::InvalidInput,
//                 "Tool run with no parameters.",
//             ));
//         }
//         for i in 0..args.len() {
//             let mut arg = args[i].replace("\"", "");
//             arg = arg.replace("\'", "");
//             let cmd = arg.split("="); // in case an equals sign was used
//             let vec = cmd.collect::<Vec<&str>>();
//             let mut keyval = false;
//             if vec.len() > 1 {
//                 keyval = true;
//             }
//             if vec[0].to_lowercase() == "-i"
//                 || vec[0].to_lowercase() == "--input"
//                 || vec[0].to_lowercase() == "--dem"
//             {
//                 if keyval {
//                     input_file = vec[1].to_string();
//                 } else {
//                     input_file = args[i + 1].to_string();
//                 }
//             } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
//                 if keyval {
//                     output_file = vec[1].to_string();
//                 } else {
//                     output_file = args[i + 1].to_string();
//                 }
//             } else if vec[0].to_lowercase() == "-zero_background"
//                 || vec[0].to_lowercase() == "--zero_background"
//                 || vec[0].to_lowercase() == "--esri_style"
//             {
//                 if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
//                     zero_background = true;
//                 }
//             }
//         }

//         if verbose {
//             println!("***************{}", "*".repeat(self.get_tool_name().len()));
//             println!("* Welcome to {} *", self.get_tool_name());
//             println!("***************{}", "*".repeat(self.get_tool_name().len()));
//         }

//         let sep: String = path::MAIN_SEPARATOR.to_string();

//         let mut progress: usize;
//         let mut old_progress: usize = 1;

//         if !input_file.contains(&sep) && !input_file.contains("/") {
//             input_file = format!("{}{}", working_directory, input_file);
//         }
//         if !output_file.contains(&sep) && !output_file.contains("/") {
//             output_file = format!("{}{}", working_directory, output_file);
//         }

//         if verbose {
//             println!("Reading data...")
//         };

//         let input = Raster::new(&input_file, "r")?;

//         let start = Instant::now();
//         let rows = input.configs.rows as isize;
//         let columns = input.configs.columns as isize;
//         let num_cells = rows * columns;
//         let nodata = input.configs.nodata;

//         let mut output = Raster::initialize_using_file(&output_file, &input);
//         let mut background_val = (i32::min_value() + 1) as f64;
//         output.reinitialize_values(background_val);

//         /*
//         Find the data edges. This is complicated by the fact that DEMs frequently
//         have nodata edges, whereby the DEM does not occupy the full extent of
//         the raster. One approach to doing this would be simply to scan the
//         raster, looking for cells that neighbour nodata values. However, this
//         assumes that there are no interior nodata holes in the dataset. Instead,
//         the approach used here is to perform a region-growing operation, looking
//         for nodata values along the raster's edges.
//         */
//         let mut queue: VecDeque<(isize, isize)> =
//             VecDeque::with_capacity((rows * columns) as usize);
//         for row in 0..rows {
//             /*
//             Note that this is only possible because Whitebox rasters
//             allow you to address cells beyond the raster extent but
//             return the nodata value for these regions.
//             */
//             queue.push_back((row, -1));
//             queue.push_back((row, columns));
//         }

//         for col in 0..columns {
//             queue.push_back((-1, col));
//             queue.push_back((rows, col));
//         }

//         /*
//         minheap is the priority queue. Note that I've tested using integer-based
//         priority values, by multiplying the elevations, but this didn't result
//         in a significant performance gain over the use of f64s.
//         */
//         let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
//         let mut num_solved_cells = 0;
//         let mut zin_n: f64; // value of neighbour of row, col in input raster
//         let mut zout: f64; // value of row, col in output raster
//         let mut zout_n: f64; // value of neighbour of row, col in output raster
//         let dx = [1, 1, 1, 0, -1, -1, -1, 0];
//         let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
//         let (mut row, mut col): (isize, isize);
//         let (mut row_n, mut col_n): (isize, isize);
//         while !queue.is_empty() {
//             let cell = queue.pop_front().unwrap();
//             row = cell.0;
//             col = cell.1;
//             for n in 0..8 {
//                 row_n = row + dy[n];
//                 col_n = col + dx[n];
//                 zin_n = input[(row_n, col_n)];
//                 zout_n = output[(row_n, col_n)];
//                 if zout_n == background_val {
//                     if zin_n == nodata {
//                         output[(row_n, col_n)] = nodata;
//                         queue.push_back((row_n, col_n));
//                     } else {
//                         output[(row_n, col_n)] = zin_n;
//                         // Push it onto the priority queue for the priority flood operation
//                         minheap.push(GridCell {
//                             row: row_n,
//                             column: col_n,
//                             priority: zin_n,
//                         });
//                     }
//                     num_solved_cells += 1;
//                 }
//             }

//             if verbose {
//                 progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
//                 if progress != old_progress {
//                     println!("progress: {}%", progress);
//                     old_progress = progress;
//                 }
//             }
//         }

//         // Perform the priority flood operation.
//         while !minheap.is_empty() {
//             let cell = minheap.pop().expect("Error during pop operation.");
//             row = cell.row;
//             col = cell.column;
//             zout = output[(row, col)];
//             for n in 0..8 {
//                 row_n = row + dy[n];
//                 col_n = col + dx[n];
//                 zout_n = output[(row_n, col_n)];
//                 if zout_n == background_val {
//                     zin_n = input[(row_n, col_n)];
//                     if zin_n != nodata {
//                         if zin_n < zout {
//                             zin_n = zout;
//                         } // We're in a depression. Raise the elevation.
//                         output[(row_n, col_n)] = zin_n;
//                         minheap.push(GridCell {
//                             row: row_n,
//                             column: col_n,
//                             priority: zin_n,
//                         });
//                     } else {
//                         // Interior nodata cells are still treated as nodata and are not filled.
//                         output[(row_n, col_n)] = nodata;
//                         num_solved_cells += 1;
//                     }
//                 }
//             }

//             if verbose {
//                 num_solved_cells += 1;
//                 progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
//                 if progress != old_progress {
//                     println!("Progress (Loop 1 of 2): {}%", progress);
//                     old_progress = progress;
//                 }
//             }
//         }

//         background_val = nodata;
//         if zero_background {
//             background_val = 0f64;
//         }
//         for row in 0..rows {
//             for col in 0..columns {
//                 if output[(row, col)] > input[(row, col)] {
//                     output[(row, col)] = output[(row, col)] - input[(row, col)];
//                 } else {
//                     if input[(row, col)] != nodata {
//                         output[(row, col)] = background_val;
//                     } else {
//                         output[(row, col)] = nodata;
//                     }
//                 }
//             }
//             if verbose {
//                 progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
//                 if progress != old_progress {
//                     println!("Progress (Loop 2 of 2): {}%", progress);
//                     old_progress = progress;
//                 }
//             }
//         }

//         let elapsed_time = get_formatted_elapsed_time(start);
//         output.configs.data_type = DataType::F32;
//         output.configs.palette = "qual.plt".to_string();
//         output.configs.photometric_interp = PhotometricInterpretation::Categorical;
//         output.add_metadata_entry(format!(
//             "Created by whitebox_tools\' {} tool",
//             self.get_tool_name()
//         ));
//         output.add_metadata_entry(format!("Input file: {}", input_file));
//         output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

//         if verbose {
//             println!("Saving data...")
//         };
//         let _ = match output.write() {
//             Ok(_) => {
//                 if verbose {
//                     println!("Output file written")
//                 }
//             }
//             Err(e) => return Err(e),
//         };
//         if verbose {
//             println!(
//                 "{}",
//                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
//             );
//         }

//         Ok(())
//     }
// }

// #[derive(PartialEq, Debug)]
// struct GridCell {
//     row: isize,
//     column: isize,
//     priority: f64,
// }

// impl Eq for GridCell {}

// impl PartialOrd for GridCell {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         other.priority.partial_cmp(&self.priority)
//     }
// }

// impl Ord for GridCell {
//     fn cmp(&self, other: &GridCell) -> Ordering {
//         let ord = self.partial_cmp(other).unwrap();
//         match ord {
//             Ordering::Greater => Ordering::Less,
//             Ordering::Less => Ordering::Greater,
//             Ordering::Equal => ord,
//         }
//     }
// }
