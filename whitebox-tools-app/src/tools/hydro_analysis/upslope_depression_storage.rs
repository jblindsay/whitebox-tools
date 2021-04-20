/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/11/2019
Last Modified: 21/11/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::f32;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool estimates the average upslope depression storage depth using the FD8 flow algorithm.
/// The input DEM (`--dem`) need not be hydrologically corrected; the tool will internally map depression
/// storage and resolve flowpaths using depression filling. This input elevation model should be of a
/// fine resolution (< 2 m), and is ideally derived using LiDAR. The tool calculates the total upslope
/// depth of depression storage, which is divided by the number of upslope cells in the final step
/// of the process, yielding the average upslope depression depth. Roughened surfaces tend to have higher
/// values compared with smoothed surfaces. Values, particularly on hillslopes, may be very small (< 0.01 m).
///
/// # See Also
/// `FD8FlowAccumulation`, `FillDepressions`, `DepthInSink`
pub struct UpslopeDepressionStorage {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl UpslopeDepressionStorage {
    pub fn new() -> UpslopeDepressionStorage {
        // public constructor
        let name = "UpslopeDepressionStorage".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Estimates the average upslope depression storage depth.".to_string();

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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        UpslopeDepressionStorage {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for UpslopeDepressionStorage {
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

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        // let cell_area = input.configs.resolution_x * input.configs.resolution_y;
        let (mut col, mut row): (isize, isize);
        let (mut rn, mut cn): (isize, isize);
        let mut z: f32;
        let mut zn: f32;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        // let back_link = [4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8];
        let mut num_solved: usize;

        // let cell_size_x = input.configs.resolution_x as f32;
        // let cell_size_y = input.configs.resolution_y as f32;
        // let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        // let grid_lengths = [
        //     diag_cell_size,
        //     cell_size_x,
        //     diag_cell_size,
        //     cell_size_y,
        //     diag_cell_size,
        //     cell_size_x,
        //     diag_cell_size,
        //     cell_size_y,
        // ];

        let mut filled = input.get_data_as_f32_array2d();
        let nodata = filled.nodata();

        let elev_digits = (input.configs.maximum as i64).to_string().len();
        let elev_multiplier = 10.0_f64.powi((6 - elev_digits) as i32);
        let small_num = 1.0_f32 / elev_multiplier as f32;

        let mut output = Raster::initialize_using_file(&output_file, &input);

        // drop(input); // input is no longer needed.

        // Now we need to perform an in-place depression filling

        // Start by finding all cells that neighbour NoData cells.
        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        // let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
        let mut num_cells_visited = 0;
        for row in 0..rows {
            for col in 0..columns {
                z = filled.get_value(row, col);
                if z != nodata {
                    for n in 0..8 {
                        if filled.get_value(row + dy[n], col + dx[n]) == nodata {
                            minheap.push(GridCell {
                                row: row,
                                column: col,
                                priority: z,
                            });
                            visited.set_value(row, col, 1);
                            output.set_value(row, col, 0f64);
                            num_cells_visited += 1;
                            break;
                        }
                    }
                } else {
                    visited.set_value(row, col, 1);
                    num_cells_visited += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding edges: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        while !minheap.is_empty() {
            let cell = minheap.pop().expect("Error during pop operation.");
            row = cell.row;
            col = cell.column;
            z = filled.get_value(row, col);
            for n in 0..8 {
                rn = row + dy[n];
                cn = col + dx[n];
                if visited.get_value(rn, cn) == 0i8 {
                    zn = filled.get_value(rn, cn);
                    if zn < (z + small_num) {
                        // output.set_value(rn, cn, (z - zn) as f64); // * cell_area);
                        if (zn as f64) < (input.get_value(row, col) + output.get_value(row, col)) {
                            output.set_value(
                                rn,
                                cn,
                                (input.get_value(row, col) + output.get_value(row, col))
                                    - zn as f64,
                            );
                        } else {
                            output.set_value(rn, cn, 0f64);
                        }
                        filled.set_value(rn, cn, z + small_num);
                    } else {
                        output.set_value(rn, cn, 0f64);
                    }
                    minheap.push(GridCell {
                        row: rn,
                        column: cn,
                        priority: zn,
                    });
                    visited.set_value(rn, cn, 1);
                    // flow_dir.set_value(rn, cn, back_link[n]);
                    num_cells_visited += 1;
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * num_cells_visited as f64 / (rows * columns - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Filling: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        drop(input);
        drop(visited);

        // let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        // let filled = Arc::new(filled);
        // let flow_dir = Arc::new(flow_dir);
        // let mut num_procs = num_cpus::get() as isize;
        // let configs = whitebox_common::configs::get_configs()?;
        // let max_procs = configs.max_procs;
        // if max_procs > 0 && max_procs < num_procs {
        //     num_procs = max_procs;
        // }
        // let (tx, rx) = mpsc::channel();
        // for tid in 0..num_procs {
        //     let filled = filled.clone();
        //     let flow_dir = flow_dir.clone();
        //     let tx = tx.clone();
        //     thread::spawn(move || {
        //         let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        //         let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        //         let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
        //         let mut count: i8;
        //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //             let mut data: Vec<i8> = vec![-1i8; columns as usize];
        //             for col in 0..columns {
        //                 if filled.get_value(row, col) != nodata {
        //                     count = 0i8;
        //                     for i in 0..8 {
        //                         if flow_dir.get_value(row + dy[i], col + dx[i]) == inflowing_vals[i] {
        //                             count += 1;
        //                         }
        //                     }
        //                     data[col as usize] = count;
        //                 } else {
        //                     data[col as usize] = -1i8;
        //                 }
        //             }
        //             tx.send((row, data)).unwrap();
        //         }
        //     });
        // }

        // let mut stack = Vec::with_capacity((rows * columns) as usize);
        // num_solved = 0;
        // for r in 0..rows {
        //     let (row, data) = rx.recv().expect("Error receiving data from thread.");
        //     num_inflowing.set_row_data(row, data);
        //     for col in 0..columns {
        //         if num_inflowing.get_value(row, col) == 0i8 {
        //             stack.push((row, col));
        //         } else if num_inflowing[(row, col)] == -1i8 {
        //             num_solved += 1;
        //         }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Num. inflowing neighbours: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // let mut area: Array2D<i32> = Array2D::new(rows, columns, 1, -1)?;
        // let mut fa: f64;
        // let mut fa2: i32;
        // let mut dir: i8;
        // while !stack.is_empty() {
        //     let cell = stack.pop().expect("Error during pop operation.");
        //     row = cell.0;
        //     col = cell.1;
        //     fa = output.get_value(row, col);
        //     fa2 = area.get_value(row, col);
        //     num_inflowing.decrement(row, col, 1i8);
        //     dir = flow_dir.get_value(row, col);
        //     if dir >= 0 {
        //         rn = row + dy[dir as usize];
        //         cn = col + dx[dir as usize];
        //         output.increment(rn, cn, fa);
        //         area.increment(rn, cn, fa2);
        //         num_inflowing.decrement(rn, cn, 1i8);
        //         if num_inflowing.get_value(rn, cn) == 0i8 {
        //             stack.push((rn, cn));
        //         }
        //     }

        //     if verbose {
        //         num_solved += 1;
        //         progress = (100.0_f64 * num_solved as f64 / (rows*columns - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Flow accumulation: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = filled.get_value(row, col);
        //         if z != nodata {
        //             output.set_value(row, col, output.get_value(row, col) / area.get_value(row, col) as f64);
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * num_cells_visited as f64 / (rows*columns - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Final calculation: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // calculate the number of inflowing cells
        let filled = Arc::new(filled);
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let filled = filled.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut z: f32;
                let mut zn: f32;
                let mut count: i8;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = filled.get_value(row, col);
                        if z != nodata {
                            count = 0i8;
                            for n in 0..8 {
                                zn = filled.get_value(row + dy[n], col + dx[n]);
                                if zn > z && zn != nodata {
                                    count += 1;
                                }
                            }
                            data[col as usize] = count;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut stack = Vec::with_capacity((rows * columns) as usize);
        num_solved = 0;
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            num_inflowing.set_row_data(row, data);
            for col in 0..columns {
                if num_inflowing.get_value(row, col) == 0i8 {
                    stack.push((row, col));
                } else if num_inflowing.get_value(row, col) == -1i8 {
                    num_solved += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Num. inflowing neighbours: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut fa: f64;
        let mut fa2: f64;
        let mut area: Array2D<f64> = Array2D::new(rows, columns, 1f64, -1f64)?;
        // let (mut max_slope, mut slope): (f32, f32);
        // let mut dir: i8;
        // let convergence_threshold = f64::INFINITY;
        let exponent = 1.1f32;
        let mut total_weights: f32;
        let mut weights: [f32; 8] = [0.0; 8];
        let mut downslope: [bool; 8] = [false; 8];
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            z = filled.get_value(row, col);
            fa = output.get_value(row, col);
            fa2 = area.get_value(row, col);
            num_inflowing.set_value(row, col, -1i8);
            total_weights = 0.0f32;
            for n in 0..8 {
                rn = row + dy[n];
                cn = col + dx[n];
                zn = filled.get_value(rn, cn);
                if zn < z && zn != nodata {
                    weights[n] = (z - zn).powf(exponent);
                    total_weights += weights[n];
                    downslope[n] = true;
                } else {
                    weights[n] = 0f32;
                    downslope[n] = false;
                }
            }

            if total_weights > 0.0 {
                for n in 0..8 {
                    if downslope[n] {
                        rn = row + dy[n];
                        cn = col + dx[n];
                        output.increment(rn, cn, fa * (weights[n] / total_weights) as f64);
                        area.increment(rn, cn, fa2 * (weights[n] / total_weights) as f64);
                        num_inflowing.decrement(rn, cn, 1i8);
                        if num_inflowing.get_value(rn, cn) == 0i8 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }

            if verbose {
                num_solved += 1;
                progress = (100.0_f64 * num_solved as f64 / (rows * columns - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow accumulation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = filled.get_value(row, col);
                if z != nodata {
                    output.set_value(
                        row,
                        col,
                        output.get_value(row, col) / area.get_value(row, col),
                    );
                }
            }
            if verbose {
                progress =
                    (100.0_f64 * num_cells_visited as f64 / (rows * columns - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Final calculation: {}%", progress);
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
    priority: f32,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell {
    fn cmp(&self, other: &GridCell) -> Ordering {
        // other.priority.cmp(&self.priority)
        let ord = self.partial_cmp(other).unwrap();
        match ord {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => ord,
        }
    }
}
