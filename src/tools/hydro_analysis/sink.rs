/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/07/2017
Last Modified: 05/12/2019
License: MIT

Note: the previous iteration of this tool did not include the outlet cell itself as part of the depression. 
This iteration of the tool does include outlets.
*/

use crate::raster::*;
use crate::tools::*;
use crate::structures::Array2D;
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

/// This tool identifies each sink (i.e. topographic depression) in a raster digital elevation model (DEM). A 
/// sink, or depression, is a bowl-like landscape feature, which is characterized by interior drainage. Each 
/// identified sink in the input DEM is assigned a unique, non-zero, positive value in the ouput raster. The 
/// `Sink` tool essentially runs the `FillDepressions` tool followed by the `Clump` tool on all modified grid
/// cells.
/// 
/// # See Also
/// `FillDepressions`, `Clump`
pub struct Sink {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Sink {
    pub fn new() -> Sink {
        // public constructor
        let name = "Sink".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Identifies the depressions in a DEM, giving each feature a unique identifier."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned(), "--input".to_owned()],
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
            description: "Flag indicating whether a background value of zero should be used."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=filled_dem.tif --zero_background", short_exe, name).replace("*", &sep);

        Sink {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Sink {
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
        let nodata = input.configs.nodata;

        let mut filled_dem = input.get_data_as_array2d();

        let (mut col, mut row): (isize, isize);
        let (mut rn, mut cn): (isize, isize);
        let (mut z, mut zn): (f64, f64);
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        // Find pit cells. This step is parallelized.
        let num_procs = num_cpus::get() as isize;   
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
                for row in (1..rows-1).filter(|r| r % num_procs == tid) {
                    for col in 1..columns-1 {
                        z = filled_dem2.get_value(row, col);
                        if z != nodata {
                            flag = true;
                            for n in 0..8 {
                                zn = filled_dem2.get_value(row + dy[n], col + dx[n]);
                                if zn < z || zn == nodata { // It either has a lower neighbour or is an edge cell.
                                    flag = false;
                                    break;
                                }
                            }
                            if flag { // it's a cell with undefined flow
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
            let mut pits = rx.recv().unwrap();
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
        drop(input);

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
            if flats.get_value(row, col) != 1 { // if it's already in a solved site, don't do it a second time.
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
                                    } else if zn != nodata { // zn < z
                                        // 'cell' has a lower neighbour that hasn't already passed through minheap. 
                                        // Therefore, 'cell' is a pour point cell.
                                        outlet_found = true;
                                        outlet_z = z;
                                        queue.push_back((cell2.row, cell2.column));
                                        possible_outlets.push((cell2.row, cell2.column));
                                    }
                                } else if zn == outlet_z { // We've found the outlet but are still looking for additional outlets.
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
                            if flag { // it's an outlet
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

        input_configs.nodata = i32::MIN as f64;
        let mut output = Raster::initialize_using_config(&output_file, &input_configs);
        if zero_background {
            output.reinitialize_values(0f64);
        }
        output.configs.data_type = DataType::I32;
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        let mut dep_id = 1f64;
        let num_outlets = possible_outlets.len();
        while let Some(cell) = possible_outlets.pop() {
            if flats.get_value(cell.0, cell.1) == 1 {
                z = filled_dem.get_value(cell.0, cell.1);
                output.set_value(cell.0, cell.1, dep_id);
                let mut queue = VecDeque::new();
                flats.set_value(cell.0, cell.1, 0);
                queue.push_back((cell.0, cell.1, dep_id));
                while let Some(cell2) = queue.pop_front() {
                    for n in 0..8 {
                        rn = cell2.0 + dy[n];
                        cn = cell2.1 + dx[n];
                        if flats.get_value(rn, cn) == 1 {
                            if filled_dem.get_value(rn, cn) == z {
                                flats.set_value(rn, cn, 0);
                                output.set_value(rn, cn, dep_id);
                                queue.push_back((rn, cn, dep_id));
                            }
                        }
                    }
                }
                dep_id += 1f64;
            }
            if verbose {
                progress = (100.0_f64 * (1.0 - possible_outlets.len() as f64 / num_outlets as f64)) as usize;
                if progress != old_progress {
                    println!("Labelling depressions: {}%", progress);
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
