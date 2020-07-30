/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 14/06/2017
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::{Array2D, DistanceMetric, FixedRadiusSearch2D};
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to fill in small gaps in a raster or digital elevation model (DEM). The gaps,
/// or holes, must have recognized NoData values. If gaps do not currently have this characteristic, use
/// the `SetNodataValue` tool and ensure that the data are stored using a raster format that supports NoData
/// values. All valid, non-NoData values in the input raster will be assigned the same value in the output image.
///
/// The algorithm uses an inverse-distance weighted (IDW) scheme based on the valid values on the edge of
/// NoData gaps to estimate gap values. The user must specify the filter size (`--filter`), which determines
/// the size of gap that is filled, and the IDW weight (`--weight`).
///
/// The filter size, specified in grid cells, is used to determine how far the algorithm will search for valid,
/// non-NoData values. Therefore, setting a larger filter size allows for the filling of larger gaps in the input
/// raster.
///
/// The `--no_edges` flag can be used to exclude NoData values that are connected to the edges of the raster. It is
/// usually the case that irregularly shaped DEMs have large regions of NoData values along the containing raster
/// edges. This flag can be used to exclude these regions from the gap-filling operation, leaving only interior gaps
/// for filling.
///
/// # See Also
/// `SetNodataValue`
pub struct FillMissingData {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FillMissingData {
    pub fn new() -> FillMissingData {
        // public constructor
        let name = "FillMissingData".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Fills NoData holes in a DEM.".to_string();

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

        parameters.push(ToolParameter {
            name: "Filter Dimension".to_owned(),
            flags: vec!["--filter".to_owned()],
            description: "Filter size (cells).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "IDW Weight (Exponent) Value".to_owned(),
            flags: vec!["--weight".to_owned()],
            description: "IDW weight value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Exclude edge-of-raster-connected NoData cells?".to_owned(),
            flags: vec!["--no_edges".to_owned()],
            description:
                "Optional flag indicating whether to exclude NoData cells in edge regions."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif --filter=25 --weight=1.0 --no_edges",
            short_exe, name
        ).replace("*", &sep);

        FillMissingData {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FillMissingData {
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
        let mut output_file = String::new();
        let mut filter_size = 11usize;
        let mut weight = 2.0f64;
        let mut exclude_edge_nodata = false;
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
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-filter" {
                if keyval {
                    filter_size = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
            } else if flag_val == "-weight" {
                weight = if keyval {
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
            } else if flag_val == "-no_edges" {
                exclude_edge_nodata = true;
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        // The filter dimensions must be odd numbers such that there is a middle pixel
        if (filter_size as f64 / 2f64).floor() == (filter_size as f64 / 2f64) {
            filter_size += 1;
        }

        // let mut z: f64;
        let (mut row_n, mut col_n): (isize, isize);
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
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let start = Instant::now();

        let nodata = input.configs.nodata;
        let columns = input.configs.columns as isize;
        let rows = input.configs.rows as isize;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut edge_connected_nodata: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;

        if exclude_edge_nodata {
            if verbose {
                println!("Finding edge-connected NoData values...");
            }
            // Scan the raster edges, placing NoData values in a stack
            let mut stack = vec![]; // Vec::with_capacity(((rows + columns) * 5) as usize);
            for row in 0..rows {
                if input.get_value(row, 0) == nodata {
                    stack.push((row, 0));
                    edge_connected_nodata.set_value(row, 0, 1);
                }

                if input.get_value(row, columns - 1) == nodata {
                    stack.push((row, columns - 1));
                    edge_connected_nodata.set_value(row, columns - 1, 1);
                }
            }

            for col in 0..columns {
                if input.get_value(0, col) == nodata {
                    stack.push((0, col));
                    edge_connected_nodata.set_value(0, col, 1);
                }

                if input.get_value(rows - 1, col) == nodata {
                    stack.push((rows - 1, col));
                    edge_connected_nodata.set_value(rows - 1, col, 1);
                }
            }

            // Perform a region-growing operation locating all connected NoData cells.
            let mut num_cells_popped = 0;
            while let Some(cell) = stack.pop() {
                for n in 0..8 {
                    if input.get_value(cell.0 + dy[n], cell.1 + dx[n]) == nodata
                        && edge_connected_nodata.get_value(cell.0 + dy[n], cell.1 + dx[n]) == 0
                    {
                        stack.push((cell.0 + dy[n], cell.1 + dx[n]));
                        edge_connected_nodata.set_value(cell.0 + dy[n], cell.1 + dx[n], 1);
                    }
                }
                if verbose {
                    num_cells_popped += 1;
                    progress = (100.0_f64 * num_cells_popped as f64 / (rows * columns - 1) as f64)
                        as usize;
                    if progress != old_progress {
                        println!("Finding edge-connected NoData cells: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        // Interpolate the data holes. Start by locating all the edge cells.
        if verbose {
            println!("Interpolating data holes...")
        };
        let mut frs: FixedRadiusSearch2D<f64> =
            FixedRadiusSearch2D::new(filter_size as f64, DistanceMetric::Euclidean);
        if weight == 2f64 {
            frs.set_distance_metric(DistanceMetric::SquaredEuclidean);
        }
        for row in 0..rows {
            for col in 0..columns {
                if input.get_value(row, col) != nodata {
                    for i in 0..8 {
                        row_n = row + dy[i];
                        col_n = col + dx[i];
                        if input.get_value(row_n, col_n) == nodata
                            && edge_connected_nodata.get_value(row_n, col_n) == 0
                        {
                            frs.insert(col as f64, row as f64, input[(row, col)]);
                            break;
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Interpolating data holes: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let input = Arc::new(input);
        let edge_connected_nodata = Arc::new(edge_connected_nodata);
        let frs = Arc::new(frs);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let frs = frs.clone();
            let tx1 = tx.clone();
            let edge_connected_nodata = edge_connected_nodata.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let mut z: f64;
                let mut sum_weights: f64;
                let mut dist: f64;
                match weight {
                    x if (x == 1f64 || x == 2f64) => {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                if input.get_value(row, col) == nodata
                                    && edge_connected_nodata.get_value(row, col) == 0
                                {
                                    sum_weights = 0f64;
                                    let ret = frs.search(col as f64, row as f64);
                                    for j in 0..ret.len() {
                                        dist = ret[j].1 as f64;
                                        if dist > 0.0 {
                                            sum_weights += 1.0 / dist;
                                        }
                                    }
                                    z = 0.0;
                                    for j in 0..ret.len() {
                                        dist = ret[j].1 as f64;
                                        if dist > 0.0 {
                                            z += ret[j].0 * (1.0 / dist) / sum_weights;
                                        }
                                    }
                                    if ret.len() > 0 {
                                        data[col as usize] = z;
                                    } else {
                                        data[col as usize] = nodata;
                                    }
                                } else {
                                    data[col as usize] = input[(row, col)];
                                }
                            }
                            tx1.send((row, data)).unwrap();
                        }
                    }
                    _ => {
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                if input.get_value(row, col) == nodata
                                    && edge_connected_nodata.get_value(row, col) == 0
                                {
                                    sum_weights = 0f64;
                                    let ret = frs.search(col as f64, row as f64);
                                    for j in 0..ret.len() {
                                        dist = ret[j].1 as f64;
                                        if dist > 0.0 {
                                            sum_weights += 1.0 / dist.powf(weight);
                                        }
                                    }
                                    z = 0.0;
                                    for j in 0..ret.len() {
                                        dist = ret[j].1 as f64;
                                        if dist > 0.0 {
                                            z += ret[j].0 * (1.0 / dist.powf(weight)) / sum_weights;
                                        }
                                    }
                                    if ret.len() > 0 {
                                        data[col as usize] = z;
                                    } else {
                                        data[col as usize] = nodata;
                                    }
                                } else {
                                    data[col as usize] = input[(row, col)];
                                }
                            }
                            tx1.send((row, data)).unwrap();
                        }
                    }
                }
                // for row in (0..rows).filter(|r| r % num_procs == tid) {
                //     let mut data = vec![nodata; columns as usize];
                //     for col in 0..columns {
                //         if input[(row, col)] == nodata {
                //             sum_weights = 0f64;
                //             let ret = frs.search(col as f64, row as f64);
                //             for j in 0..ret.len() {
                //                 dist = ret[j].1 as f64;
                //                 if dist > 0.0 {
                //                     sum_weights += 1.0 / dist.powf(weight); //(dist * dist);
                //                 }
                //             }
                //             z = 0.0;
                //             for j in 0..ret.len() {
                //                 dist = ret[j].1 as f64;
                //                 if dist > 0.0 {
                //                     z += ret[j].0 * (1.0 / dist.powf(weight)) / sum_weights; //(dist * dist)) / sum_weights;
                //                 }
                //             }
                //             if ret.len() > 0 {
                //                 data[col as usize] = z;
                //             } else {
                //                 data[col as usize] = nodata;
                //             }
                //         } else {
                //             data[col as usize] = input[(row, col)];
                //         }
                //     }
                //     tx1.send((row, data)).unwrap();
                // }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
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
        output.add_metadata_entry(format!("Filter size x: {}", filter_size));
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
