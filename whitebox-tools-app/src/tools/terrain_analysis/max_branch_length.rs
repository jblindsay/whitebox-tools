/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/07/2017
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::Raster;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Maximum branch length (`Bmax`) is the longest branch length between a grid cell's flowpath
/// and the flowpaths initiated at each of its neighbours. It can be conceptualized as the
/// downslope distance that a volume of water that is split into two portions by a drainage
/// divide would travel before reuniting.
///
/// If the two flowpaths of neighbouring grid cells do not intersect, `Bmax` is simply the
/// flowpath length from the starting cell to its terminus at the edge of the grid or a cell
/// with undefined flow direction (i.e. a pit cell either in a topographic depression or at
/// the edge of a major body of water).
///
/// The pattern of `Bmax` derived from a DEM should be familiar to anyone who has interpreted
/// upslope contributing area images. In fact, `Bmax` can be thought of as the complement of
/// upslope contributing area. Whereas contributing area is greatest along valley bottoms and lowest at
/// drainage divides, `Bmax` is greatest at divides and lowest along channels. The two topographic
/// attributes are also distinguished by their units of measurements; `Bmax` is a length rather
/// than an area. The presence of a major drainage divide between neighbouring grid cells is apparent in
/// a `Bmax` image as a linear feature, often two grid cells wide, of relatively high values. This
/// property makes `Bmax` a useful land surface parameter for mapping ridges and divides.
///
/// `Bmax` is useful in the study of landscape structure, particularly with respect to drainage patterns.
/// The index gives the relative significance of a specific location along a divide, with respect to the
/// dispersion of materials across the landscape, in much the same way that stream ordering can be used
/// to assess stream size.
///
/// ![](../../doc_img/MaxBranchLength_fig1.png)
///
/// # See Also
/// `FlowLengthDiff`
///
/// # Reference
/// Lindsay JB, Seibert J. 2013. Measuring the significance of a divide to local drainage patterns.
/// International Journal of Geographical Information Science, 27: 1453-1468. DOI:
/// 10.1080/13658816.2012.705289
pub struct MaxBranchLength {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MaxBranchLength {
    pub fn new() -> MaxBranchLength {
        // public constructor
        let name = "MaxBranchLength".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Lindsay and Seibert's (2013) branch length index is used to map drainage divides or ridge lines.".to_string();

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
            name: "Log-transform the output?".to_owned(),
            flags: vec!["--log".to_owned()],
            description: "Optional flag to request the output be log-transformed.".to_owned(),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        MaxBranchLength {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MaxBranchLength {
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
        let mut log_transform = false;

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
            } else if vec[0].to_lowercase() == "-log" || vec[0].to_lowercase() == "--log" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    log_transform = true;
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        // calculate the flow direction
        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        let flow_nodata = -2i8;

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
                    let mut data: Vec<i8> = vec![flow_nodata; columns as usize];
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
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, flow_nodata, flow_nodata)?;
        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().expect("Error receiving data from thread.");
            flow_dir.set_row_data(row, data);
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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(0f64);
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
        let mut dir: i8;
        let (mut dist1, mut dist2): (f64, f64);
        let mut flag1: bool;
        let mut flag2: bool;
        let (mut r1, mut c1): (isize, isize);
        let (mut r2, mut c2): (isize, isize);
        let mut idx: isize;
        let mut paths: Array2D<isize> = Array2D::new(rows, columns, 0, 0)?;
        let mut path_lengths: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        for row in 0..rows {
            for col in 0..columns {
                if flow_dir[(row, col)] >= 0i8 {
                    idx = row * rows as isize + col + 1;

                    // right cell
                    r2 = row;
                    c2 = col + 1;
                    if flow_dir[(r2, c2)] >= 0i8 {
                        r1 = row;
                        c1 = col;
                        dist1 = 0f64;
                        dist2 = 0f64;
                        flag1 = true;
                        flag2 = true;
                        while flag1 || flag2 {
                            if flag1 {
                                if paths[(r1, c1)] == idx {
                                    // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist2 = path_lengths[(r1, c1)];
                                }
                                paths[(r1, c1)] = idx;
                                path_lengths[(r1, c1)] = dist1;
                                dir = flow_dir[(r1, c1)];
                                if dir >= 0 {
                                    r1 += dy[dir as usize];
                                    c1 += dx[dir as usize];
                                    dist1 += grid_lengths[dir as usize];
                                } else {
                                    flag1 = false;
                                }
                            }

                            if flag2 {
                                if paths[(r2, c2)] == idx {
                                    // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist1 = path_lengths[(r2, c2)];
                                }
                                paths[(r2, c2)] = idx;
                                path_lengths[(r2, c2)] = dist2;
                                dir = flow_dir[(r2, c2)];
                                if dir >= 0 {
                                    r2 += dy[dir as usize];
                                    c2 += dx[dir as usize];
                                    dist2 += grid_lengths[dir as usize];
                                } else {
                                    flag2 = false;
                                }
                            }
                        }
                        if dist1 > output[(row, col)] {
                            output.set_value(row, col, dist1);
                        }
                        if dist2 > output[(row, col + 1)] {
                            output.set_value(row, col + 1, dist2);
                        }
                    }

                    // lower cell
                    r2 = row + 1;
                    c2 = col;
                    if flow_dir[(r2, c2)] >= 0i8 {
                        idx = -idx;
                        r1 = row;
                        c1 = col;
                        dist1 = 0f64;
                        dist2 = 0f64;
                        flag1 = true;
                        flag2 = true;
                        while flag1 || flag2 {
                            if flag1 {
                                if paths[(r1, c1)] == idx {
                                    // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist2 = path_lengths[(r1, c1)];
                                }
                                paths[(r1, c1)] = idx;
                                path_lengths[(r1, c1)] = dist1;
                                dir = flow_dir[(r1, c1)];
                                if dir >= 0 {
                                    r1 += dy[dir as usize];
                                    c1 += dx[dir as usize];
                                    dist1 += grid_lengths[dir as usize];
                                } else {
                                    flag1 = false;
                                }
                            }

                            if flag2 {
                                if paths[(r2, c2)] == idx {
                                    // intersection
                                    flag1 = false;
                                    flag2 = false;
                                    dist1 = path_lengths[(r2, c2)];
                                }
                                paths[(r2, c2)] = idx;
                                path_lengths[(r2, c2)] = dist2;
                                dir = flow_dir[(r2, c2)];
                                if dir >= 0 {
                                    r2 += dy[dir as usize];
                                    c2 += dx[dir as usize];
                                    dist2 += grid_lengths[dir as usize];
                                } else {
                                    flag2 = false;
                                }
                            }
                        }
                        if dist1 > output[(row, col)] {
                            output.set_value(row, col, dist1);
                        }
                        if dist2 > output[(row + 1, col)] {
                            output.set_value(row + 1, col, dist2);
                        }
                    }
                } else if input[(row, col)] == nodata {
                    output[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if log_transform {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] != nodata {
                        if output[(row, col)] > 0f64 {
                            output[(row, col)] = output[(row, col)].ln();
                        } else {
                            output[(row, col)] = nodata;
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Log transformation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        output.configs.palette = "grey.plt".to_string();
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
