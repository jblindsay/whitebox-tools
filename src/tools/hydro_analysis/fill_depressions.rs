/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 28, 2017
Last Modified: June 28, 2017
License: MIT
*/
extern crate time;

use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::cmp::Ordering;
use std::env;
use std::path;
use std::i32;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct FillDepressions {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl FillDepressions {
    pub fn new() -> FillDepressions { // public constructor
        let name = "FillDepressions".to_string();
        
        let description = "Fills all of the depressions in a DEM. Depression breaching should be preferred in most cases.".to_string();
        
        let mut parameters = "--dem           Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--fix_flats     Optional flag indicating whether flat areas should have a small gradient applied.\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep --fix_flats", short_exe, name).replace("*", &sep);
    
        FillDepressions { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for FillDepressions {
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
        let mut output_file = String::new();
        let mut fix_flats = false;
        
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
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-fix_flats" || vec[0].to_lowercase() == "--fix_flats" {
                fix_flats = true;
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
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;

        let min_val = input.configs.minimum;
        let elev_digits = ((input.configs.maximum - min_val) as i64).to_string().len();
        let elev_multiplier = 10.0_f64.powi((7 - elev_digits) as i32);
        let mut small_num = 0.0; 
        if fix_flats {
            small_num = 1.0 / elev_multiplier as f64;
        }
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let background_val = (i32::min_value() + 1) as f64;
        output.reinitialize_values(background_val);

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
                        output[(row_n, col_n)] = zin_n;
                        // Push it onto the priority queue for the priority flood operation
                        minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
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

        /* 
        The following code follows the scenario of a priority-flood method without the extra
        complication of an embedded region-growing operation for in-depression sites.
        */

        // Perform the priority flood operation.
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
                        if zin_n < (zout + small_num) { zin_n = zout + small_num; } // We're in a depression. Raise the elevation.
                        output[(row_n, col_n)] = zin_n;
                        minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                    } else {
                        // Interior nodata cells are still treated as nodata and are not filled.
                        output[(row_n, col_n)] = nodata;
                        num_solved_cells += 1;
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

        /*
        This code uses a slightly more complex priority flood approach that uses an embedded
        region-growing operation for cells within depressions. It offers a slight speed up
        over the traditional approach, but I have noticed that sometimes it doesn't work
        as expected. I'm not sure why and it would require some effort to track down the bug. 
        Given that most DEMs have relatively few cells within depressions, the speed up of 
        this approach is perhaps not worthwhile and it is certainly more complex.
        */
        // // Perform the priority flood operation.
        // while !minheap.is_empty() {
        //     let cell = minheap.pop().unwrap();
        //     row = cell.row;
        //     col = cell.column;
        //     zout = output[(row, col)];
        //     for n in 0..8 {
        //         row_n = row + dy[n];
        //         col_n = col + dx[n];
        //         zout_n = output[(row_n, col_n)];
        //         if zout_n == background_val {
        //             zin_n = input[(row_n, col_n)];
        //             if zin_n != nodata {
        //                 if zin_n < (zout + small_num) {
        //                     // We're in a depression. Raise the elevation.
        //                     zout_n = zout + small_num;
        //                     output[(row_n, col_n)] = zout_n;
        //                     /*
        //                     Cells that are in the depression don't need to be discovered by
        //                     the more expensive priority-flood operation. Instead, perform
        //                     an efficient region-growing operation to find cells connected 
        //                     to this cell that have elevations in the input DEM that are 
        //                     less than the adjusted zout_n.
        //                     */
        //                     queue.push_back((row_n, col_n));
        //                     while !queue.is_empty() {
        //                         let cell = queue.pop_front().unwrap();
        //                         row = cell.0;
        //                         col = cell.1;
        //                         zout = output[(row, col)];
        //                         for n2 in 0..8 {
        //                             row_n = row + dy[n2];
        //                             col_n = col + dx[n2];
        //                             zout_n = output[(row_n, col_n)];
        //                             if zout_n == background_val {
        //                                 zin_n = input[(row_n, col_n)];
        //                                 if zin_n != nodata {
        //                                     if zin_n < (zout + small_num) { 
        //                                         zout_n = zout + small_num;
        //                                         output[(row_n, col_n)] = zout_n;
        //                                         queue.push_back((row_n, col_n));
        //                                     } else {
        //                                         minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
        //                                         output[(row_n, col_n)] = zin_n;
        //                                     }
        //                                 } else {
        //                                     // Interior nodata cells are still treated as nodata and are not filled.
        //                                     output[(row_n, col_n)] = nodata;
        //                                     num_solved_cells += 1;
        //                                 }
        //                             }
        //                         }
        //                         if verbose {
        //                             num_solved_cells += 1;
        //                             progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
        //                             if progress != old_progress {
        //                                 println!("Progress: {}%", progress);
        //                                 old_progress = progress;
        //                             }
        //                         }
        //                     }
        //                 } else {
        //                     minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
        //                     output[(row_n, col_n)] = zin_n;
        //                 }
        //             } else {
        //                 // Interior nodata cells are still treated as nodata and are not filled.
        //                 output[(row_n, col_n)] = nodata;
        //                 num_solved_cells += 1;
        //             }
        //         }
        //     }

        //     if verbose {
        //         num_solved_cells += 1;
        //         progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }


        /* This was an experiement with an approach that reduced the reliance on the priority queue.
        It works well but is slightly less efficient that the traditional approach. */

        // let mut output = Raster::initialize_using_file(&output_file, &input);
        // let background_val = (i32::max_value() - 1) as f64;
        // output.reinitialize_values(background_val);

        // /*
        // Find the data edges. This is complicated by the fact that DEMs frequently
        // have nodata edges, whereby the DEM does not occupy the full extent of 
        // the raster. One approach to doing this would be simply to scan the
        // raster, looking for cells that neighbour nodata values. However, this
        // assumes that there are no interior nodata holes in the dataset. Instead,
        // the approach used here is to perform a region-growing operation, looking
        // for nodata values along the raster's edges.
        // */

        // //let mut stack = Vec::with_capacity((rows * columns) as usize);
        // let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity((rows * columns) as usize);
        // for row in 0..rows {
        //     /*
        //     Note that this is only possible because Whitebox rasters
        //     allow you to address cells beyond the raster extent but
        //     return the nodata value for these regions.
        //     */
        //     queue.push_back((row, -1));
        //     queue.push_back((row, columns));
        // }

        // for col in 0..columns {
        //     queue.push_back((-1, col));
        //     queue.push_back((rows, col));
        // }

        // /* 
        // minheap is the priority queue. Note that I've tested using integer-based
        // priority values, by multiplying the elevations, but this didn't result
        // in a significant performance gain over the use of f64s.
        // */
        // let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
        // let mut num_solved_cells = 0;
        // let mut zin_n: f64; // value of neighbour of row, col in input raster
        // let mut zout: f64; // value of row, col in output raster
        // let mut zout_n: f64; // value of neighbour of row, col in output raster
        // let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        // let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        // let (mut row, mut col): (isize, isize);
        // let (mut row_n, mut col_n): (isize, isize);
        // while !queue.is_empty() {
        //     let cell = queue.pop_front().unwrap();
        //     row = cell.0;
        //     col = cell.1;
        //     for n in 0..8 {
        //         row_n = row + dy[n];
        //         col_n = col + dx[n];
        //         zin_n = input[(row_n, col_n)];
        //         zout_n = output[(row_n, col_n)];
        //         if zout_n == background_val {
        //             if zin_n == nodata {
        //                 output[(row_n, col_n)] = nodata;
        //                 queue.push_back((row_n, col_n));
        //             } else {
        //                 output[(row_n, col_n)] = zin_n;
        //                 // Push it onto the priority queue for the priority flood operation
        //                 minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
        //             }
        //             num_solved_cells += 1;
        //         }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // // Perform the priority flood operation.
        // // let initial_heap_size = minheap.len();
        // // num_solved_cells = 0;
        // while !minheap.is_empty() {
        //     let cell = minheap.pop().unwrap();
        //     queue.push_back((cell.row, cell.column));
        //     while !queue.is_empty() {
        //         let cell = queue.pop_front().unwrap();
        //         row = cell.0;
        //         col = cell.1;
        //         zout = output[(row, col)];
        //         for n in 0..8 {
        //             row_n = row + dy[n];
        //             col_n = col + dx[n];
        //             zout_n = output[(row_n, col_n)];
        //             zin_n = input[(row_n, col_n)];
        //             if zout_n > zin_n {
        //                 if zin_n != nodata {
        //                     if zin_n > zout + small_num {
        //                         output[(row_n, col_n)] = zin_n;
        //                         queue.push_back((row_n, col_n));
        //                         num_solved_cells += 1;
        //                     } else if zout + small_num < zout_n {
        //                         output[(row_n, col_n)] = zout + small_num;
        //                         queue.push_back((row_n, col_n));
        //                     }
        //                 } else {
        //                     output[(row_n, col_n)] = nodata;
        //                     queue.push_back((row_n, col_n));
        //                     num_solved_cells += 1;
        //                 }
        //             }
        //         }

        //         if verbose {
        //             progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Progress: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }
        // }

        // if verbose { println!("Progress: 100%"); }

        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Fix flats: {}", fix_flats));
        if fix_flats {
            output.add_metadata_entry(format!("Flat increment value: {}", small_num));
        }
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
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
    fn partial_cmp(&self, other: &GridCell) -> Option<Ordering> {
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