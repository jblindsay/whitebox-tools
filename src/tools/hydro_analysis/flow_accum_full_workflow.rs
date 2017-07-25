/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 28, 2017
Last Modified: June 28, 2017
License: MIT

NOTES: This tool provides a full workflow D8 flow operation. This includes removing depressions, calculating 
the D8 pointer raster and finally the D8 flow accumulation operation. 
*/
extern crate time;
extern crate num_cpus;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::cmp::Ordering;
use std::env;
use std::path;
use std::i32;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use structures::Array2D;
use tools::WhiteboxTool;

pub struct FlowAccumulationFullWorkflow {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl FlowAccumulationFullWorkflow {
    pub fn new() -> FlowAccumulationFullWorkflow { // public constructor
        let name = "FlowAccumulationFullWorkflow".to_string();
        
        let description = "Resolves all of the depressions in a DEM, outputing an aspect-aligned flow pointer, then performs a flow accumulation operation".to_string();
        
        let mut parameters = "--dem           Input raster DEM file.\n".to_owned();
        parameters.push_str("--out_dem       Output hydrologically corrected DEM file.\n");
        parameters.push_str("--out_pntr      Output flow pointer raster file.\n");
        parameters.push_str("--out_accum     Output flow accumulation raster file.\n");
        parameters.push_str("--out_type      Output type; one of 'cells', 'sca' (default), and 'ca'.\n");
        parameters.push_str("--log           Optional flag to request the output be log-transformed.\n");
        parameters.push_str("--clip          Optional flag to request clipping the display max by 1%.\n");
        parameters.push_str("--esri_style    Uses the ESRI style D8 pointer output (default is false).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem='DEM.dep' --out_dem='DEM_filled.dep' --out_pntr='pointer.dep' --out_accum='accum.dep' --out_type=sca --log --clip", short_exe, name).replace("*", &sep);
    
        FlowAccumulationFullWorkflow { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for FlowAccumulationFullWorkflow {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut outdem_file = String::new();
        let mut pntr_file = String::new();
        let mut accum_file = String::new();
        let mut out_type = String::from("sca");
        let mut log_transform = false;
        let mut clip_max = false;
        let mut esri_style = false;

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_dem" || vec[0].to_lowercase() == "--out_dem" {
                if keyval {
                    outdem_file = vec[1].to_string();
                } else {
                    outdem_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_pntr" || vec[0].to_lowercase() == "--out_pntr" {
                if keyval {
                    pntr_file = vec[1].to_string();
                } else {
                    pntr_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_accum" || vec[0].to_lowercase() == "--out_accum" {
                if keyval {
                    accum_file = vec[1].to_string();
                } else {
                    accum_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_type" || vec[0].to_lowercase() == "--out_type" {
                if keyval {
                    out_type = vec[1].to_lowercase();
                } else {
                    out_type = args[i+1].to_lowercase();
                }
                if out_type.contains("specific") || out_type.contains("sca") {
                    out_type = String::from("sca");
                } else if out_type.contains("cells") {
                    out_type = String::from("cells");
                } else {
                    out_type = String::from("ca");
                }
            } else if vec[0].to_lowercase() == "-log" || vec[0].to_lowercase() == "--log" {
                log_transform = true;
            } else if vec[0].to_lowercase() == "-clip" || vec[0].to_lowercase() == "--clip" {
                clip_max = true;
            } else if vec[0].to_lowercase() == "-esri_style" || vec[0].to_lowercase() == "--esri_style" {
                esri_style = true;
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

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !outdem_file.contains(&sep) {
            outdem_file = format!("{}{}", working_directory, outdem_file);
        }
        if !pntr_file.contains(&sep) {
            pntr_file = format!("{}{}", working_directory, pntr_file);
        }
        if !accum_file.contains(&sep) {
            accum_file = format!("{}{}", working_directory, accum_file);
        }

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // Calculate aspect from the DEM. This will be used in calculating flow directions.
        let mut z_factor = 1.0;
        if input.is_in_geographic_coordinates() {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (113200.0 * mid_lat.cos());
            }
        }

        let eight_grid_res = input.configs.resolution_x * 8.0;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let (mut fx, mut fy): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            for c in 0..8 {
                                n[c] = input[(row + dy[c], col + dx[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z * z_factor;
                                }
                            }
                            // calculate slope
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            if fx != 0f64 {
                                data[col as usize] = 180f64 - ((fy / fx).atan()).to_degrees() + 90f64 * (fx / (fx).abs());
                                
                            } else {
                                data[col as usize] = nodata;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut aspect: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        for row in 0..rows {
            let data = rx.recv().unwrap();
            aspect.set_row_data(data.0, data.1);
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating aspect: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        let min_val = input.configs.minimum;
        let elev_digits = ((input.configs.maximum - min_val) as i64).to_string().len();
        let elev_multiplier = 10.0_f64.powi((7 - elev_digits) as i32);
        let small_num = 1.0 / elev_multiplier as f64;
        
        let mut output = Raster::initialize_using_file(&outdem_file, &input);
        let background_val = (i32::min_value() + 1) as f64;
        output.reinitialize_values(background_val);

        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;

        /*
        Find the data edges. This is complicated by the fact that DEMs frequently
        have nodata edges, whereby the DEM does not occupy the full extent of 
        the raster. One approach to doing this would be simply to scan the
        raster, looking for cells that neighbour nodata values. However, this
        assumes that there are no interior nodata holes in the dataset. Instead,
        the approach used here is to perform a region-growing operation, looking
        for nodata values along the raster's edges.
        */

        let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity((rows * columns) as usize);
        for row in 0..rows {
            /*
            Note that this is only possible because Whitebox rasters
            allow you to address cells beyond the raster extent but
            return the nodata value for these regions.
            */
            queue.push_back((row, -1));
            queue.push_back((row, columns));
        }

        for col in 0..columns {
            queue.push_back((-1, col));
            queue.push_back((rows, col));
        }

        /* 
        minheap is the priority queue. Note that I've tested using integer-based
        priority values, by multiplying the elevations, but this didn't result
        in a significant performance gain over the use of f64s.
        */
        let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        let mut zin_n: f64; // value of neighbour of row, col in input raster
        let mut zout: f64; // value of row, col in output raster
        let mut zout_n: f64; // value of neighbour of row, col in output raster
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let (mut x, mut y): (isize, isize);
        let mut is_lowest: bool;
        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            row = cell.0;
            col = cell.1;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                zin_n = input[(row_n, col_n)];
                zout_n = output[(row_n, col_n)];
                if zout_n == background_val {
                    if zin_n == nodata {
                        output[(row_n, col_n)] = nodata;
                        queue.push_back((row_n, col_n));
                    } else {
                        // see if it's the lowest of its neighbours
                        is_lowest = true;
                        for p in 0..8 {
                            y = row_n + dy[p];
                            x = col_n + dx[p];
                            if input[(y, x)] < zin_n && input[(y, x)] != nodata {
                                is_lowest = false;
                                break;
                            }
                        }
                        if is_lowest {
                            output[(row_n, col_n)] = zin_n;
                            // Push it onto the priority queue for the priority flood operation
                            minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                            // flow_dir[(row_n, col_n)] = 0;
                        }
                    }
                    num_solved_cells += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Perform the priority flood operation.
        let back_link = [ 4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8 ];
        let (mut x, mut y): (isize, isize);
        let mut z_target: f64;
        let mut dir: i8;
        let mut flag: bool;
        let directions = [ 45f64, 90f64, 135f64, 180f64, 225f64, 270f64, 315f64, 360f64 ];

        while !minheap.is_empty() {
            let cell = minheap.pop().unwrap();
            row = cell.row;
            col = cell.column;
            zout = output[(row, col)];
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                zout_n = output[(row_n, col_n)];
                if zout_n == background_val {
                    zin_n = input[(row_n, col_n)];
                    if zin_n != nodata {
                        flow_dir[(row_n, col_n)] = back_link[n];

                        // if zin_n < (zout + small_num) { zin_n = zout + small_num; } // We're in a depression. Raise the elevation.
                        // output[(row_n, col_n)] = zin_n;
                        // minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });

                        output[(row_n, col_n)] = zin_n;
                        minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                        if zin_n < (zout + small_num) {
                            // Trace the flowpath back to a lower cell, if it exists.
                            x = col_n;
                            y = row_n;
                            z_target = output[(row_n, col_n)];
                            flag = true;
                            while flag {
                                dir = flow_dir[(y, x)];
                                if dir >= 0 {
                                    y += dy[dir as usize];
                                    x += dx[dir as usize];
                                    z_target -= small_num;
                                    if output[(y, x)] > z_target {
                                        output[(y, x)] = z_target;
                                    } else {
                                        flag = false;
                                    }
                                } else {
                                    flag = false;
                                }
                            }
                        }
                    } else {
                        // Interior nodata cells are still treated as nodata and are not filled.
                        output[(row_n, col_n)] = nodata;
                        num_solved_cells += 1;
                    }
                } else if zout_n > zout && zout_n != nodata && aspect[(row_n, col_n)] != nodata {
                    /* Check to see if the flow direction could be improved; if so, capture its flow.
                    This is the main logic for the flow direction calculation. Basically,
                    we link cells to the neighbour that has the closest flow direction to the
                    cell's aspect and is connected by a continuous downward path to an edge cell. 
                    We are looking to minimize the absolute difference between the aspect and the 
                    D8 flow direction. */
                    if flow_dir[(row_n, col_n)] >= 0 {
                        let prospective_fd = directions[back_link[n] as usize];
                        let mut diff1 = prospective_fd - aspect[(row_n, col_n)];
                        if diff1 > 180f64 { diff1 -= 360f64 }
                        if diff1 < -180f64 { diff1 += 360f64 }
                        diff1 = diff1.abs();
                        
                        let current_fd = directions[flow_dir[(row_n, col_n)] as usize];
                        let mut diff2 = current_fd - aspect[(row_n, col_n)];
                        if diff2 > 180f64 { diff2 -= 360f64 }
                        if diff2 < -180f64 { diff2 += 360f64 }
                        diff2 = diff2.abs();
                        
                        if diff1 < diff2 { // if this cell is closer to the aspect of the neighbouring cell then the current pointer value.
                            flow_dir[(row_n, col_n)] = back_link[n];
                        }
                    }
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", time::now() - start).replace("PT", ""));

        if verbose { println!("Saving DEM data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };


        // calculate the number of inflowing cells
        let flow_dir = Arc::new(flow_dir);
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let flow_dir = flow_dir.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                let inflowing_vals: [i8; 8] = [ 4, 5, 6, 7, 0, 1, 2, 3 ];
                let mut z: f64;
                let mut count: i8;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            count = 0i8;
							for i in 0..8 {
                                if flow_dir[(row + dy[i], col + dx[i])] == inflowing_vals[i] {
                                    count += 1;
                                }
                            }
                            data[col as usize] = count;
                        } else {
                            data[col as usize] = -1i8;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&accum_file, &input);
        output.reinitialize_values(1.0);
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
            num_inflowing.set_row_data(row, data);
            for col in 0..columns {
                if num_inflowing[(row, col)] == 0i8 {
                    stack.push((row, col));
                } else if num_inflowing[(row, col)] == -1i8 {
                    num_solved_cells += 1;
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

        // let mut dir: i8;
        let mut fa: f64;
        while !stack.is_empty() {
            let cell = stack.pop().unwrap();
            row = cell.0;
            col = cell.1;
            fa = output[(row, col)];
            num_inflowing.decrement(row, col, 1i8);
            dir = flow_dir[(row, col)];
            if dir >= 0 {
                row_n = row + dy[dir as usize];
                col_n = col + dx[dir as usize];
                output.increment(row_n, col_n, fa);
                num_inflowing.decrement(row_n, col_n, 1i8);
                if num_inflowing[(row_n, col_n)] == 0i8 {
                    stack.push((row_n, col_n));
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow accumulation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut cell_area = cell_size_x * cell_size_y;
        let mut flow_widths = [diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x];
        if out_type == "cells" {
            cell_area = 1.0;
            flow_widths = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        } else if out_type == "ca" {
            flow_widths = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        }

        let mut pntr = Raster::initialize_using_file(&pntr_file, &input);
        let pntr_vals = match esri_style {
            true => [ 128f64, 1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64 ],
            false => [ 1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64, 128f64 ],
        };

        if log_transform {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] == nodata {
                        output[(row, col)] = nodata;
                    } else {
                        let dir = flow_dir[(row, col)];
                        if dir >= 0 {
                            output[(row, col)] = (output[(row, col)] * cell_area / flow_widths[dir as usize]).ln();
                            pntr[(row, col)] = pntr_vals[flow_dir[(row, col)] as usize];
                        } else {
                            output[(row, col)] = (output[(row, col)] * cell_area / flow_widths[3]).ln();
                            pntr[(row, col)] = 0f64;
                        }
                    }
                }
                
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Correcting values: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            for row in 0..rows {
                for col in 0..columns {
                    if input[(row, col)] == nodata {
                        output[(row, col)] = nodata;
                    } else {
                        let dir = flow_dir[(row, col)];
                        if dir >= 0 {
                            output[(row, col)] = output[(row, col)] * cell_area / flow_widths[dir as usize];
                            pntr[(row, col)] = pntr_vals[flow_dir[(row, col)] as usize];
                        } else {
                            output[(row, col)] = output[(row, col)] * cell_area / flow_widths[3];
                            pntr[(row, col)] = 0f64;
                        }
                    }
                }
                
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Correcting values: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        
        pntr.configs.palette = "qual.plt".to_string();
        pntr.configs.photometric_interp = PhotometricInterpretation::Categorical;
        pntr.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        pntr.add_metadata_entry(format!("Input file: {}", input_file));
        pntr.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving flow pointer data...") };
        let _ = match pntr.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        output.configs.palette = "blueyellow.plt".to_string();
        if clip_max { 
            output.clip_display_max(1.0); 
        }
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving accumulation data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
struct GridCell {
    row: isize,
    column: isize,
    // priority: usize,
    priority: f64,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Some(other.priority.cmp(&self.priority))
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