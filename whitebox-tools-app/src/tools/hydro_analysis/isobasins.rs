/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 03/12/2017
Last Modified: 24/07/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to divide a landscape into a group of nearly equal-sized watersheds, known as *isobasins*.
/// The user must specify the name (`--dem`) of a digital elevation model (DEM), the output raster name (`--output`),
/// and the isobasin target area (`--size`) specified in units of grid cells. The DEM must have been hydrologically
/// corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using either
/// the `BreachDepressions` or `FillDepressions` tool. Several temporary rasters are created during the execution
/// and stored in memory of this tool.
///
/// The tool can optionally (`--connections`) output a CSV table that contains the upstream/downstream connections
/// among isobasins. That is, this table will identify the downstream basin of each isobasin, or will list N/A in
/// the event that there is no downstream basin, i.e. if it drains to an edge. Additionally, the CSV file will contain
/// information about the number of grid cells in each isobasin and the isobasin outlet's row and column number and  
/// flow direction. The output CSV file will have the same name as the output raster, but with a *.csv file extension.
///
/// # See Also
/// `Watershed`, `Basins`, `BreachDepressions`, `FillDepressions`
pub struct Isobasins {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Isobasins {
    pub fn new() -> Isobasins {
        // public constructor
        let name = "Isobasins".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Divides a landscape into nearly equal sized drainage basins (i.e. watersheds)."
                .to_string();

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
            name: "Target Basin Size (grid cells)".to_owned(),
            flags: vec!["--size".to_owned()],
            description: "Target basin size, in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output basin upstream-downstream connections?".to_owned(),
            flags: vec!["--connections".to_owned()],
            description: "Output upstream-downstream flow connections among basins?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --size=1000",
            short_exe, name
        )
        .replace("*", &sep);

        Isobasins {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Isobasins {
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
        let mut target_size = -1;
        let mut output_connections = false;

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
            } else if flag_val == "-size" {
                target_size = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            } else if flag_val == "-connections" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    output_connections = true;
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

        if target_size == -1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Target basin size (--size) not specified.",
            ));
        }

        let target_fa = target_size as usize;

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

        //////////////////////////////////
        // Calculate the flow direction //
        //////////////////////////////////
        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let grid_lengths = [
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                ];
                let (mut z, mut z_n): (f64, f64);
                let (mut max_slope, mut slope): (f64, f64);
                let mut dir: i8;
                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            dir = 0i8;
                            max_slope = f64::MIN;
                            neighbouring_nodata = false;
                            for i in 0..8 {
                                z_n = input[(row + dy[i], col + dx[i])];
                                if z_n != nodata {
                                    slope = (z - z_n) / grid_lengths[i];
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i as i8;
                                    }
                                } else {
                                    neighbouring_nodata = true;
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = dir;
                            } else {
                                data[col as usize] = -1i8;
                                if !neighbouring_nodata {
                                    interior_pit_found = true;
                                }
                            }
                        } else {
                            data[col as usize] = -1i8;
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            flow_dir.set_row_data(row, data); //(data.0, data.1);
            if pit {
                interior_pit_found = true;
            }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow directions: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        /////////////////////////////////////////////
        // Calculate the number of inflowing cells //
        /////////////////////////////////////////////
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
        let mut z: f64;
        let mut count: i8;
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata {
                    count = 0i8;
                    for i in 0..8 {
                        if flow_dir.get_value(row + dy[i], col + dx[i]) == inflowing_vals[i] {
                            count += 1;
                        }
                    }
                    num_inflowing.set_value(row, col, count);

                    if count == 0 {
                        stack.push((row, col));
                    }
                } else {
                    num_solved_cells += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Num. inflowing neighbours: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        /////////////////////////////////
        // Find and ID the pour points //
        /////////////////////////////////
        let mut accum: Array2D<usize> = Array2D::new(rows, columns, 1, 0)?;
        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::I32;
        let out_nodata = -32768f64;
        output.configs.nodata = out_nodata;
        output.reinitialize_values(out_nodata);
        let mut outlet_id = 1f64;
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let mut dir: i8;
        let mut fa: usize;
        let mut inla_index: usize;
        let mut inla_mag: usize;
        let mut outlet_row = vec![];
        let mut outlet_col = vec![];
        let mut outlet_fa = vec![];
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            fa = accum.get_value(row, col);
            if fa >= target_fa {
                // find the index of the inflowing neighbour with the largest accumulation
                inla_mag = 0;
                inla_index = 8;
                for i in 0..8 {
                    row_n = row + dy[i];
                    col_n = col + dx[i];
                    if flow_dir.get_value(row_n, col_n) == inflowing_vals[i] {
                        if accum.get_value(row_n, col_n) > inla_mag {
                            inla_mag = accum.get_value(row_n, col_n);
                            inla_index = i;
                        }
                    }
                }
                if (target_fa - inla_mag) < (fa - target_fa) {
                    if inla_index < 8 {
                        row_n = row + dy[inla_index];
                        col_n = col + dx[inla_index];
                        accum.decrement(row, col, inla_mag);
                        fa -= inla_mag;
                        output.set_value(row_n, col_n, outlet_id);
                        outlet_id += 1f64;
                        outlet_row.push(row_n);
                        outlet_col.push(col_n);
                        outlet_fa.push(inla_mag);
                    } else {
                        accum.set_value(row, col, 1);
                        fa = 1;
                        output.set_value(row, col, outlet_id);
                        outlet_id += 1f64;
                        outlet_row.push(row);
                        outlet_col.push(col);
                        outlet_fa.push(inla_mag);
                    }
                } else {
                    outlet_fa.push(fa);
                    accum.set_value(row, col, 1);
                    fa = 0;
                    output.set_value(row, col, outlet_id);
                    outlet_id += 1f64;
                    outlet_row.push(row);
                    outlet_col.push(col);
                }
            }
            num_inflowing.decrement(row, col, 1i8);
            dir = flow_dir[(row, col)];
            if dir >= 0 {
                row_n = row + dy[dir as usize];
                col_n = col + dx[dir as usize];
                accum.increment(row_n, col_n, fa);
                num_inflowing.decrement(row_n, col_n, 1i8);
                if num_inflowing[(row_n, col_n)] == 0i8 {
                    stack.push((row_n, col_n));
                }
            } else {
                if output.get_value(row, col) == out_nodata {
                    output.set_value(row, col, outlet_id);
                    outlet_id += 1f64;
                    outlet_row.push(row);
                    outlet_col.push(col);
                    outlet_fa.push(fa);
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding pour points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let num_outlets = outlet_id as usize - 1;

        //////////////////////////////////////////
        // Trace flowpaths to their pour points //
        //////////////////////////////////////////
        let mut flag: bool;
        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata && output.get_value(row, col) == out_nodata {
                    // trace the flowpath from this cell until you find an outlet ID in the output
                    outlet_id = nodata;
                    flag = true;
                    col_n = col;
                    row_n = row;
                    while flag {
                        // find its downslope neighbour
                        dir = flow_dir.get_value(row_n, col_n);
                        if dir >= 0 {
                            // move x and y accordingly
                            col_n += dx[dir as usize];
                            row_n += dy[dir as usize];

                            // if the new cell already has a value in the output, use that as the outletID
                            z = output.get_value(row_n, col_n);
                            if z != out_nodata {
                                outlet_id = z;
                                flag = false;
                            }
                        } else {
                            flag = false;
                        }
                    }

                    flag = true;
                    col_n = col;
                    row_n = row;
                    output.set_value(row_n, col_n, outlet_id);
                    while flag {
                        // find its downslope neighbour
                        dir = flow_dir.get_value(row_n, col_n);
                        if dir >= 0 {
                            // move x and y accordingly
                            col_n += dx[dir as usize];
                            row_n += dy[dir as usize];

                            // if the new cell already has a value in the output, use that as the outletID
                            z = output.get_value(row_n, col_n);
                            if z != out_nodata {
                                outlet_id = z;
                                flag = false;
                            }
                        } else {
                            flag = false;
                        }
                        output.set_value(row_n, col_n, outlet_id);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Labelling basins: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if output_connections {
            let mut connections_table = vec![-1isize; num_outlets+1];
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let mut z_n: f64;
            for row in 0..rows {
                for col in 0..columns {
                    z = output.get_value(row, col);
                    if z != out_nodata {
                        for i in 0..8 {
                            z_n = output.get_value(row + dy[i], col + dx[i]);
                            if z_n != z && z_n != out_nodata {
                                // neighbouring cell is in a different basin
                                if flow_dir.get_value(row + dy[i], col + dx[i]) == inflowing_vals[i]
                                {
                                    // neighbour cell flows into (row, col)
                                    connections_table[z_n as usize] = z as isize;
                                }
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Labelling basins connections: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let csv_file = path::Path::new(&output_file)
                .with_extension("csv")
                .into_os_string()
                .into_string()
                .expect("Error when trying to create CSV file.");

            let f = File::create(csv_file.clone()).expect("Error while creating CSV file.");
            let mut writer = BufWriter::new(f);
            writer
                .write_all("UPSTREAM,DOWNSTREAM,OUTLET_ROW,OUTLET_COL,ACCUM,FLOW_DIR\n".as_bytes())
                .expect("Error while writing to CSV file.");
            for i in 1..=num_outlets {
                row_n = outlet_row[i-1];
                col_n = outlet_col[i-1];
                fa = outlet_fa[i-1];
                dir = flow_dir.get_value(row_n, col_n);
                let fd = if dir >= 0 {
                    2i32.pow(dir as u32)
                } else {
                    0i32
                };
                if connections_table[i] != -1 {
                    let s = format!("{},{},{},{},{},{}\n", i, connections_table[i], row_n, col_n, fa, fd);
                    writer
                        .write_all(s.as_bytes())
                        .expect("Error while writing to CSV file.");
                } else {
                    let s = format!("{},N/A,{},{},{},{}\n", i, row_n, col_n, fa, fd);
                    writer
                        .write_all(s.as_bytes())
                        .expect("Error while writing to CSV file.");
                }
            }

            let _ = writer.flush();

            if verbose {
                println!("Please see {} for basin connection table.", csv_file);
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.data_type = DataType::F32;
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Target basin size: {}", target_fa));
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
