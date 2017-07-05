/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 4, 2017
Last Modified: July 4, 2017
License: MIT

NOTES: Add anisotropy option.
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::i32;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct CostDistance {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl CostDistance {
    pub fn new() -> CostDistance { // public constructor
        let name = "CostDistance".to_string();
        
        let description = "Performs cost-distance accumulation on a cost surface and a group of source cells.".to_string();
        
        let mut parameters = "--source        Input source raster file.\n".to_owned();
        parameters.push_str("--cost          Input cost (friction) raster file.\n");
        parameters.push_str("--out_accum     Output cost accumulation raster file.\n");
        parameters.push_str("--out_backlink  Output backlink raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" --source=src.dep --cost=cost.dep --out_accum=accum.dep --out_backlink=backlink.dep", short_exe, name).replace("*", &sep);
    
        CostDistance { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for CostDistance {
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
        let mut source_file = String::new();
        let mut cost_file = String::new();
        let mut accum_file = String::new();
        let mut backlink_file = String::new();
        
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
            if vec[0].to_lowercase() == "-source" || vec[0].to_lowercase() == "--source" {
                if keyval {
                    source_file = vec[1].to_string();
                } else {
                    source_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-cost" || vec[0].to_lowercase() == "--cost" {
                if keyval {
                    cost_file = vec[1].to_string();
                } else {
                    cost_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_accum" || vec[0].to_lowercase() == "--out_accum" {
                if keyval {
                    accum_file = vec[1].to_string();
                } else {
                    accum_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_backlink" || vec[0].to_lowercase() == "--out_backlink" {
                if keyval {
                    backlink_file = vec[1].to_string();
                } else {
                    backlink_file = args[i+1].to_string();
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

        if !source_file.contains(&sep) {
            source_file = format!("{}{}", working_directory, source_file);
        }
        if !cost_file.contains(&sep) {
            cost_file = format!("{}{}", working_directory, cost_file);
        }
        if !accum_file.contains(&sep) {
            accum_file = format!("{}{}", working_directory, accum_file);
        }
        if !backlink_file.contains(&sep) {
            backlink_file = format!("{}{}", working_directory, backlink_file);
        }

        if verbose { println!("Reading source data...") };
        let source = Raster::new(&source_file, "r")?;

        if verbose { println!("Reading cost data...") };
        let cost = Raster::new(&cost_file, "r")?;

        // make sure the input files have the same size
        if source.configs.rows != cost.configs.rows || source.configs.columns != cost.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let start = time::now();
        let rows = source.configs.rows as isize;
        let columns = source.configs.columns as isize;
        let nodata = cost.configs.nodata;
        
        let mut output = Raster::initialize_using_file(&accum_file, &cost);
        let background_val = (i32::max_value() - 1) as f64;
        output.reinitialize_values(background_val);

        let mut backlink = Raster::initialize_using_file(&backlink_file, &cost);

        for row in 0..rows {
            for col in 0..columns {
                if source[(row, col)] > 0.0 && cost[(row, col)] != nodata {
                    output[(row, col)] = 0.0;
                    backlink[(row, col)] = -1.0;
                } else if cost[(row, col)] == nodata {
                    output[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut new_cost: f64;
        let mut accum_val: f64;
        let (mut cost1, mut cost2): (f64, f64);
        let (mut row_n, mut col_n): (isize, isize);
        let cell_size_x = source.configs.resolution_x;
        let cell_size_y = source.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dist = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
        let dx = [ 1, 1, 0, -1, -1, -1, 0, 1 ];
        let dy = [ 0, 1, 1, 1, 0, -1, -1, -1 ];
        let backlink_dir = [ 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0, 16.0 ];
        let mut did_something = true;
        let mut loop_num = 0;
        while did_something {

            // Row major scans

            loop_num += 1;
            did_something = false;
            for row in 0..rows {
                for col in 0..columns {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }
            
            loop_num += 1;
            did_something = false;
            for row in (0..rows).rev() {
                for col in (0..columns).rev() {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for row in 0..rows {
                for col in (0..columns).rev() {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for row in (0..rows).rev() {
                for col in 0..columns {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }


            // Column major scans
            
            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for col in 0..columns {
                for row in 0..rows {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * col as f64 / (columns - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for col in (0..columns).rev() {
                for row in (0..rows).rev() {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * col as f64 / (columns - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for col in (0..columns).rev() {
                for row in 0..rows {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * col as f64 / (columns - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }

            if !did_something { break; }

            loop_num += 1;
            did_something = false;
            for col in 0..columns {
                for row in (0..rows).rev() {
                    accum_val = output[(row, col)];
                    if accum_val < background_val && accum_val != nodata {
                        cost1 = cost[(row, col)];
                        for n in 0..8 {
                            col_n = col + dx[n];
                            row_n = row + dy[n];
                            cost2 = cost[(row_n, col_n)];
                            new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                            if new_cost < output[(row_n, col_n)] {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                did_something = true;
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * col as f64 / (columns - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Loop {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }
        }

        /*
        The following was an experiment to use a region-growing operation to perform the cost-distance
        accumulation. It worked fine but was no more efficient that the iterative scan approach and
        had the disadvantage of not being suitable for updating progress for long periods.
        */

        // let mut zout: f64; // value of row, col in output raster
        // let (mut row, mut col): (isize, isize);
        // let (mut row_n, mut col_n): (isize, isize);
        // let mut rg_queue: VecDeque<(isize, isize)> = VecDeque::with_capacity((rows * columns) as usize);
        // let mut new_cost: f64;
        // let mut cost_val: f64;
        // let mut cost_n: f64;


        // let num_src_cells = starting_pts.len() as f64;
        // let mut i = 0.0;
        // for cell in starting_pts {
        //     i += 1.0;
        //     row = cell.0;
        //     col = cell.1;
        //     rg_queue.push_back((row, col));
        //     while !rg_queue.is_empty() {
        //         let cell = rg_queue.pop_front().unwrap();
        //         row = cell.0;
        //         col = cell.1;
        //         zout = output[(row, col)];
        //         cost_val = cost[(row, col)];
        //         if zout < 0.0 || col < 0 || row < 0 || col >= columns || row >= rows  {
        //             println!("{} {} {}", zout, row, col);
        //             break;
        //         }
        //         for n in 0..8 {
        //             row_n = row + dy[n];
        //             col_n = col + dx[n];
        //             cost_n = cost[(row_n, col_n)];
        //             if cost_n != nodata {
        //                 new_cost = zout + (cost_val + cost_n) / 2.0 * grid_lengths[n];
        //                 if new_cost < output[(row_n, col_n)] {
        //                     output.set_value(row_n, col_n, new_cost);
        //                     backlink.set_value(row_n, col_n, backlink_dir[n]);
        //                     rg_queue.push_back((row_n, col_n));
        //                 }
        //             }
        //         }

        //         if verbose {
        //             progress = (100.0_f64 * i / num_src_cells) as usize;
        //             if progress != old_progress {
        //                 println!("Progress: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }
        // }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Source raster file: {}", source_file));
        output.add_metadata_entry(format!("Cost raster: {}", cost_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        backlink.configs.palette = "qual.plt".to_string();
        backlink.configs.photometric_interp = PhotometricInterpretation::Categorical;
        backlink.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        backlink.add_metadata_entry(format!("Source raster file: {}", source_file));
        backlink.add_metadata_entry(format!("Cost raster: {}", cost_file));
        backlink.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        let _ = match backlink.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
