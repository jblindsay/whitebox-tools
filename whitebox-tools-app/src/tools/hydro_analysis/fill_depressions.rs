/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 28/06/2017
Last Modified: 12/12/2019
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
use std::i32;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the
/// flat areas. This is a common pre-processing step required by many flow-path analysis tools to ensure continuous
/// flow from each grid cell to an outlet located along the grid edge. The `FillDepressions` algorithm operates
/// by first identifying single-cell pits, that is, interior grid cells with no lower neighbouring cells. Each pit
/// cell is then visited from highest to lowest and a priority region-growing operation is initiated. The area of
/// monotonically increasing elevation, starting from the pit cell and growing based on flood order, is identified.
/// Once a cell, that has not been previously visited and possessing a lower elevation than its discovering neighbour
/// cell, is identified the discovering neighbour is labelled as an outlet (spill point) and the outlet elevation is
/// noted. The algorithm then back-fills the labelled region, raising the elevation in the output DEM (`--output`) to
/// that of the outlet. Once this process is completed for each pit cell (noting that nested pit cells are often
/// solved by prior pits) the flat regions of filled pits are optionally treated (`--fix_flats`) with an applied
/// small slope gradient away from outlets (note, more than one outlet cell may exist for each depression). The user
/// may optionally specify the size of the elevation increment used to solve flats (`--flat_increment`), although
/// **it is best to not specify this optional value and to let the algorithm determine the most suitable value itself**.
/// If a flat increment value isn't specified, the output DEM will use 64-bit floating point values in order
/// to make sure that the very small elevation increment value determined will be accurately stored. Consequently,
/// it may double the storage requirements as DEMs are often stored with 32-bit precision. However, if a flat increment
/// value is specified, the output DEM will keep the same data type as the input assuming the user chose its value wisely.
/// The flat-fixing method applies a small gradient away from outlets using another priority region-growing operation (i.e.
/// based on a priority queue operation), where priorities are set by the elevations in the input DEM (`--input`). This
/// in effect ensures a gradient away from outlet cells but also following the natural pre-conditioned topography internal
/// to depression areas. For example, if a large filled area occurs upstream of a damming road-embankment, the filled
/// DEM will possess flow directions that are similar to the un-flooded valley, with flow following the valley bottom.
/// In fact, the above case is better handled using the `BreachDepressionsLeastCost` tool, which would simply cut through
/// the road embankment at the likely site of a culvert. However, the flat-fixing method of `FillDepressions` does mean
/// that this common occurrence in LiDAR DEMs is less problematic.
///
/// The `BreachDepressionsLeastCost`, while slightly less efficient than either other hydrological preprocessing methods,
/// often provides a lower impact solution to topographic depressions and should be preferred in most applications. In comparison
/// with the `BreachDepressionsLeastCost` tool, the depression filling method often provides a less satisfactory, higher impact
/// solution. **It is advisable that users try the `BreachDepressionsLeastCost` tool to remove depressions from their DEMs
/// before using `FillDepressions`**. Nonetheless, there are applications for which full depression filling using the  
/// `FillDepressions` tool may be preferred.
///
/// Note that this tool will not fill in NoData regions within the DEM. It is advisable to remove such regions using the
/// `FillMissingData` tool prior to application.
///
/// # See Also
/// `BreachDepressionsLeastCost`, `BreachDepressions`, `Sink`, `DepthInSink`, `FillMissingData`
pub struct FillDepressions {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FillDepressions {
    pub fn new() -> FillDepressions {
        // public constructor
        let name = "FillDepressions".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.".to_string();

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
            name: "Fix flat areas?".to_owned(),
            flags: vec!["--fix_flats".to_owned()],
            description:
                "Optional flag indicating whether flat areas should have a small gradient applied."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Flat increment value (z units)".to_owned(),
            flags: vec!["--flat_increment".to_owned()],
            description: "Optional elevation increment applied to flat areas.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum depth (z units)".to_owned(),
            flags: vec!["--max_depth".to_owned()],
            description: "Optional maximum depression depth to fill.".to_owned(),
            parameter_type: ParameterType::Float,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --fix_flats",
            short_exe, name
        )
        .replace("*", &sep);

        FillDepressions {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FillDepressions {
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
        let mut fix_flats = false;
        let mut flat_increment = f64::NAN;
        let mut max_depth = f64::INFINITY;

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
            } else if flag_val == "-fix_flats" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    fix_flats = true;
                }
            } else if flag_val == "-flat_increment" {
                flat_increment = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-max_depth" {
                max_depth = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
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
        let resx = input.configs.resolution_x;
        let resy = input.configs.resolution_y;
        let diagres = (resx * resx + resy * resy).sqrt();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.set_data_from_raster(&input)?;

        let small_num = if fix_flats && !flat_increment.is_nan() {
            output.configs.data_type = input.configs.data_type; // Assume the user knows what he's doing
            flat_increment
        } else if fix_flats {
            output.configs.data_type = DataType::F64; // Don't take any chances and promote to 64-bit
            let elev_digits = (input.configs.maximum as i64).to_string().len();
            let elev_multiplier = 10.0_f64.powi((9 - elev_digits) as i32);
            1.0_f64 / elev_multiplier as f64 * diagres.ceil()
        } else {
            output.configs.data_type = input.configs.data_type;
            0f64
        };


        // drop(input); // input is no longer needed.

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
        let output2 = Arc::new(output);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let output2 = output2.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut zn: f64;
                let mut flag: bool;
                let mut pits = vec![];
                for row in (1..rows - 1).filter(|r| r % num_procs == tid) {
                    for col in 1..columns - 1 {
                        z = output2.get_value(row, col);
                        if z != nodata {
                            flag = true;
                            for n in 0..8 {
                                zn = output2.get_value(row + dy[n], col + dx[n]);
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

        output = match Arc::try_unwrap(output2) {
            Ok(val) => val,
            Err(_) => panic!("Error unwrapping 'output'"),
        };

        let num_deps = undefined_flow_cells.len();

        // Now we need to perform an in-place depression filling
        let mut minheap = BinaryHeap::new();
        let mut minheap2 = BinaryHeap::new();
        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let mut flats: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let mut possible_outlets = vec![];
        // solve from highest to lowest
        undefined_flow_cells.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Equal));
        let mut pit_id = 1;
        let mut flag: bool;
        let mut z_pit: f64;

        let mut outlet_found: bool;
        let mut outlet_z: f64;
        let mut queue = VecDeque::new();

        while let Some(cell) = undefined_flow_cells.pop() {
            row = cell.0;
            col = cell.1;
            // if it's already in a solved site, don't do it a second time.
            if flats.get_value(row, col) != 1 {
                // First there is a priority region-growing operation to find the outlets.
                z_pit = output.get_value(row, col);
                minheap.clear();
                minheap.push(GridCell {
                    row: row,
                    column: col,
                    priority: z_pit,
                });
                visited.set_value(row, col, 1);
                outlet_found = false;
                outlet_z = f64::INFINITY;
                if !queue.is_empty() {
                    queue.clear();
                }
                while let Some(cell2) = minheap.pop() {
                    z = cell2.priority;
                    if outlet_found && z > outlet_z {
                        break;
                    }
                    if z - z_pit > max_depth {
                        // No outlet could be found that was low enough.
                        break;
                    }
                    if !outlet_found {
                        for n in 0..8 {
                            cn = cell2.column + dx[n];
                            rn = cell2.row + dy[n];
                            if visited.get_value(rn, cn) == 0 {
                                zn = output.get_value(rn, cn);
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
                                    // We've found the outlet but are still looking for additional depression cells.
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
                        // We've found the outlet but are still looking for additional depression cells and potential outlets.
                        if z == outlet_z {
                            flag = false;
                            for n in 0..8 {
                                cn = cell2.column + dx[n];
                                rn = cell2.row + dy[n];
                                if visited.get_value(rn, cn) == 0 {
                                    zn = output.get_value(rn, cn);
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

                if outlet_found {
                    // Now that we have the outlets, raise the interior of the depression.
                    // Start from the outlets.
                    while let Some(cell2) = queue.pop_front() {
                        for n in 0..8 {
                            rn = cell2.0 + dy[n];
                            cn = cell2.1 + dx[n];
                            if visited.get_value(rn, cn) == 1 {
                                visited.set_value(rn, cn, 0);
                                queue.push_back((rn, cn));
                                z = output.get_value(rn, cn);
                                if z < outlet_z {
                                    output.set_value(rn, cn, outlet_z);
                                    flats.set_value(rn, cn, 1);
                                } else if z == outlet_z {
                                    flats.set_value(rn, cn, 1);
                                }
                            }
                        }
                    }
                } else {
                    queue.push_back((row, col)); // start at the pit cell and clean up visited
                    while let Some(cell2) = queue.pop_front() {
                        for n in 0..8 {
                            rn = cell2.0 + dy[n];
                            cn = cell2.1 + dx[n];
                            if visited.get_value(rn, cn) == 1 {
                                visited.set_value(rn, cn, 0);
                                queue.push_back((rn, cn));
                            }
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * pit_id as f64 / num_deps as f64) as usize;
                if progress != old_progress {
                    println!("Filling depressions: {}%", progress);
                    old_progress = progress;
                }
            }
            pit_id += 1;
        }

        drop(visited);

        if small_num > 0f64 && fix_flats {
            // fix the flats
            if verbose {
                println!("Fixing flow on flats...");
                println!("Flats increment value: {}", small_num);
            }
            // Some of the potential outlets really will have lower cells.
            // let mut queue = VecDeque::new();
            minheap.clear();
            while let Some(cell) = possible_outlets.pop() {
                z = output.get_value(cell.0, cell.1);
                flag = false;
                for n in 0..8 {
                    rn = cell.0 + dy[n];
                    cn = cell.1 + dx[n];
                    zn = output.get_value(rn, cn);
                    if zn < z && zn != nodata {
                        flag = true;
                        break;
                    }
                }
                if flag {
                    // it's confirmed as an outlet
                    minheap.push(GridCell {
                        row: cell.0,
                        column: cell.1,
                        priority: z,
                    });
                }
            }

            let num_outlets = minheap.len();
            let mut outlets = vec![];
            while let Some(cell) = minheap.pop() {
                if flats.get_value(cell.row, cell.column) != 3 {
                    z = output.get_value(cell.row, cell.column);
                    flats.set_value(cell.row, cell.column, 3);
                    // let mut outlets = vec![];
                    if !outlets.is_empty() {
                        outlets.clear();
                    }
                    outlets.push(cell);
                    // Are there any other outlet cells at the same elevation (likely for the same feature)
                    flag = true;
                    while flag {
                        match minheap.peek() {
                            Some(cell2) => {
                                if cell2.priority == z {
                                    flats.set_value(cell2.row, cell2.column, 3);
                                    outlets
                                        .push(minheap.pop().expect("Error during pop operation."));
                                } else {
                                    flag = false;
                                }
                            }
                            None => {
                                flag = false;
                            }
                        }
                    }
                    if !minheap2.is_empty() {
                        minheap2.clear();
                    }
                    for cell2 in &outlets {
                        z = output.get_value(cell2.row, cell2.column);
                        for n in 0..8 {
                            rn = cell2.row + dy[n];
                            cn = cell2.column + dx[n];
                            if flats.get_value(rn, cn) != 3 {
                                zn = output.get_value(rn, cn);
                                if zn == z && zn != nodata {
                                    // queue.push_back((rn, cn, z));
                                    minheap2.push(GridCell2 {
                                        row: rn,
                                        column: cn,
                                        z: z,
                                        priority: input.get_value(rn, cn),
                                    });
                                    output.set_value(rn, cn, z + small_num);
                                    flats.set_value(rn, cn, 3);
                                }
                            }
                        }
                    }
                    // Now fix the flats
                    while let Some(cell2) = minheap2.pop() {
                        z = output.get_value(cell2.row, cell2.column);
                        for n in 0..8 {
                            rn = cell2.row + dy[n];
                            cn = cell2.column + dx[n];
                            if flats.get_value(rn, cn) != 3 {
                                zn = output.get_value(rn, cn);
                                if zn < z + small_num && zn >= cell2.z && zn != nodata {
                                    // queue.push_back((rn, cn, cell2.2));
                                    minheap2.push(GridCell2 {
                                        row: rn,
                                        column: cn,
                                        z: cell2.z,
                                        priority: input.get_value(rn, cn),
                                    });
                                    output.set_value(rn, cn, z + small_num);
                                    flats.set_value(rn, cn, 3);
                                }
                            }
                        }
                    }
                    // while let Some(cell2) = queue.pop_front() {
                    //     z = output.get_value(cell2.0, cell2.1);
                    //     for n in 0..8 {
                    //         rn = cell2.0 + dy[n];
                    //         cn = cell2.1 + dx[n];
                    //         if flats.get_value(rn, cn) != 3 {
                    //             zn = output.get_value(rn, cn);
                    //             if zn < z + small_num && zn >= cell2.2 && zn != nodata {
                    //                 queue.push_back((rn, cn, cell2.2));
                    //                 output.set_value(rn, cn, z + small_num);
                    //                 flats.set_value(rn, cn, 3);
                    //             }
                    //         }
                    //     }
                    // }
                }

                if verbose {
                    progress =
                        (100.0_f64 * (1f64 - minheap.len() as f64 / num_outlets as f64)) as usize;
                    if progress != old_progress {
                        println!("Fixing flats: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // let mut queue = VecDeque::new();
            // minheap.clear();
            // while let Some(cell) = possible_outlets.pop() {
            //     z = output.get_value(cell.0, cell.1);
            //     flag = false;
            //     for n in 0..8 {
            //         rn = cell.0 + dy[n];
            //         cn = cell.1 + dx[n];
            //         zn = output.get_value(rn, cn);
            //         if zn < z && zn != nodata {
            //             flag = true;
            //             break;
            //         }
            //     }
            //     if flag {
            //         queue.push_back((cell.0, cell.1, z));
            //         flats.set_value(cell.0, cell.1, 2);
            //     } else {
            //         flats.set_value(cell.0, cell.1, 1);
            //     }
            // }

            // let mut flats_value: i8;

            // while let Some(cell) = queue.pop_front() {
            //     z = output.get_value(cell.0, cell.1);
            //     flats_value = flats.get_value(cell.0, cell.1);
            //     if flats_value == 2 { // outlet cell
            //         for n in 0..8 {
            //             rn = cell.0 + dy[n];
            //             cn = cell.1 + dx[n];
            //             if flats.get_value(rn, cn) == 1 {
            //                 zn = output.get_value(rn, cn);
            //                 if zn == z {
            //                     queue.push_back((rn, cn, z));
            //                     output.set_value(rn, cn, z + small_num);
            //                     flats.set_value(rn, cn, 3);
            //                 }
            //             }
            //         }
            //         flats.set_value(cell.0, cell.1, 3);
            //     } else { // non-outlet cell
            //         for n in 0..8 {
            //             rn = cell.0 + dy[n];
            //             cn = cell.1 + dx[n];
            //             flats_value = flats.get_value(rn, cn);
            //             if flats_value == 0 || flats_value == 1 {
            //                 zn = output.get_value(rn, cn);
            //                 if zn < z + small_num && zn >= cell.2 && zn != nodata {
            //                     queue.push_back((rn, cn, cell.2));
            //                     output.set_value(rn, cn, z + small_num);
            //                     flats.set_value(rn, cn, 3);
            //                 } else if flats_value == 1 {
            //                     flats.set_value(rn, cn, 3);
            //                 }
            //             }
            //         }
            //     }
            // }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Fix flats: {}", fix_flats));
        if fix_flats {
            output.add_metadata_entry(format!("Flat increment value: {}", small_num));
        }
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

#[derive(PartialEq, Debug)]
struct GridCell2 {
    row: isize,
    column: isize,
    z: f64,
    priority: f64,
}

impl Eq for GridCell2 {}

impl PartialOrd for GridCell2 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell2 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
