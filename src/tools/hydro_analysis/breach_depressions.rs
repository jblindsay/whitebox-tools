extern crate time;
extern crate num_cpus;

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

pub struct BreachDepressions {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl BreachDepressions {
    pub fn new() -> BreachDepressions { // public constructor
        let name = "BreachDepressions".to_string();
        
        let description = "This tool breaches all of the depressions in a DEM. This should be preferred over depression filling in most cases.".to_string();
        
        let mut parameters = "--dem           Input raster DEM file.\n".to_owned();
        parameters.push_str("-o, --output    Output raster file.\n");
        parameters.push_str("--max_depth     Optional maximum breach depth (default is Inf).\n");
        parameters.push_str("--max_length    Optional maximum breach channel length (in cells; default is Inf).\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -dem=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        BreachDepressions { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for BreachDepressions {
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
        let mut max_depth = f64::INFINITY;
        let mut max_length = f64::INFINITY;
        let mut constrained_mode = false;
        
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
            } else if vec[0].to_lowercase() == "-max_depth" || vec[0].to_lowercase() == "--max_depth" {
                if keyval {
                    max_depth = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_depth = args[i+1].to_string().parse::<f64>().unwrap();
                }
                constrained_mode = true;
            } else if vec[0].to_lowercase() == "-max_length" || vec[0].to_lowercase() == "--max_length" {
                if keyval {
                    max_length = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_length = args[i+1].to_string().parse::<f64>().unwrap();
                }
                constrained_mode = true;
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

        if verbose && constrained_mode {
            println!("Breaching in constrained mode...");
        }

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;

        let min_val = input.configs.minimum;
        let elev_digits = ((input.configs.maximum - min_val) as i64).to_string().len();
        let elev_multiplier = 10.0_f64.powi((7 - elev_digits) as i32);
        let small_num = 1.0 / elev_multiplier as f64;
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
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

        // Perform the priority flood operation.
        let back_link = [ 4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8 ];
        // let (mut row_n2, mut col_n2): (isize, isize);
        let (mut x, mut y): (isize, isize);
        // let mut zin_n2: f64;
        let mut z_target: f64;
        let mut dir: i8;
        // let mut is_pit: bool;
        let mut flag: bool;

        if !constrained_mode {
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
                            output[(row_n, col_n)] = zin_n;
                            minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                            if zin_n < (zout + small_num) {
                                // Is it a pit cell?
                                // is_pit = true;
                                // for n2 in 0..8 {
                                //     row_n2 = row + dy[n2];
                                //     col_n2 = col + dx[n2];
                                //     zin_n2 = input[(row_n2, col_n2)];
                                //     if zin_n2 != nodata && zin_n2 < zin_n {
                                //         is_pit = false;
                                //         break;
                                //     }
                                // }
                                // if is_pit {
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
                                // }
                            }
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
        } else { // constrained mode
            let mut channel_depth: f64;
            let mut channel_length: f64;
            let mut carved_depth: f64;
            let mut unresolved_pits = false;
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
                            output[(row_n, col_n)] = zin_n;
                            minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                            if zin_n < (zout + small_num) {
                                // Trace the flowpath back to a lower cell, if it exists.
                                x = col_n;
                                y = row_n;
                                z_target = output[(row_n, col_n)];
                                channel_depth = 0.0;
                                channel_length = 0.0;
                                flag = true;
                                while flag {
                                    dir = flow_dir[(y, x)];
                                    if dir >= 0 {
                                        y += dy[dir as usize];
                                        x += dx[dir as usize];
                                        z_target -= small_num;
                                        channel_length += 1.0;
                                        if output[(y, x)] > z_target {
                                            carved_depth = input[(y, x)] - z_target;
                                            if carved_depth > channel_depth { channel_depth = carved_depth; }
                                        } else {
                                            flag = false;
                                        }
                                    } else {
                                        flag = false;
                                    }
                                }
                                if channel_depth < max_depth && channel_length < max_length {
                                    // It's okay to breach it.
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
                                } else {
                                    unresolved_pits = true;
                                }
                            }
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

            if unresolved_pits && verbose {
                println!("There were unbreached depressions. The result should be filled to remove additional depressions.");
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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