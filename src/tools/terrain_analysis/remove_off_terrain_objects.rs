extern crate time;

use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::f64;
use std::collections::VecDeque;
use raster::*;
use structures::fixed_radius_search::FixedRadiusSearch2D;
use structures::array2d::Array2D;
use tools::WhiteboxTool;

pub struct RemoveOffTerrainObjects {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl RemoveOffTerrainObjects {
    pub fn new() -> RemoveOffTerrainObjects { // public constructor
        let name = "RemoveOffTerrainObjects".to_string();
        
        let description = "Removes off-terrain objects from a raster digital elevation model (DEM).".to_string();
        
        let mut parameters = "-i, --input        Input raster file.".to_owned();
        parameters.push_str("-o, --output       Output raster file.\n");
        parameters.push_str("--filter           Filter size (cells); default is 11.\n");
        parameters.push_str("--slope            Slope threshold; default is 15.0.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" -i=DEM.dep -o=bare_earth_DEM.dep --filter=25 --slope=10.0", short_exe, name).replace("*", &sep);
    
        RemoveOffTerrainObjects { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for RemoveOffTerrainObjects {
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
        let mut filter_size = 11usize;
        let mut slope_threshold = 15f64;
        let mut keyval: bool;
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            keyval = false;
            if vec.len() > 1 { keyval = true; }
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
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
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
                if keyval {
                    filter_size = vec[1].to_string().parse::<usize>().unwrap();
                } else {
                    filter_size = args[i+1].to_string().parse::<usize>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-slope" || vec[0].to_lowercase() == "--slope" {
                if keyval {
                    slope_threshold = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    slope_threshold = args[i+1].to_string().parse::<f64>().unwrap();
                }
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

        let (mut z, mut z_n): (f64, f64);
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let midpoint = (filter_size as f64 / 2f64).floor() as isize;
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

        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let cell_size_diag = (cell_size_x*cell_size_x + cell_size_y* cell_size_y).sqrt();
        let slope = slope_threshold.to_radians().tan();
        let height_diff_threshold = [
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y,
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y
        ];
        let columns = input.configs.columns as isize;
        let rows = input.configs.rows as isize;
        let mut opening: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut tophat: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        // Perform the white tophat transform
        { // This additional scope is simply to ensure that erosion is cleaned up at the end of the white tophat transform.
            if verbose { println!("Performing tophat transform...") };
            let mut erosion: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
            for row in 0..rows {
                let mut filter_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size);
                let start_row = row - midpoint;
                let end_row = row + midpoint;
                for col in 0..columns {
                    if col > 0 {
                        filter_vals.pop_front();
                        let mut min_val = f64::INFINITY;
                        for row2 in start_row..end_row+1 {
                            z_n = input.get_value(row2, col + midpoint);
                            if z_n < min_val && z_n != nodata { min_val = z_n; }
                        }
                        filter_vals.push_back(min_val);
                    } else {
                        // initialize the filter_vals
                        let start_col = col - midpoint;
                        let end_col = col + midpoint;
                        for col2 in start_col..end_col+1 {
                            let mut min_val = f64::INFINITY;
                            for row2 in start_row..end_row+1 {
                                z_n = input.get_value(row2, col2);
                                if z_n < min_val && z_n != nodata { min_val = z_n; }
                            }
                            filter_vals.push_back(min_val);
                        }
                    }
                    z = input.get_value(row, col);
                    if z != nodata {
                        let mut min_val = f64::INFINITY;
                        for v in filter_vals.iter() {
                            if *v < min_val { min_val = *v; }
                        }
                        if min_val < f64::INFINITY {
                            erosion[(row, col)] = min_val;
                        } else {
                            erosion[(row, col)] = min_val;
                        }
                    } else {
                        erosion[(row, col)] = nodata;
                        opening[(row, col)] = nodata;
                        tophat[(row, col)] = nodata;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing erosion: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            for row in 0..rows {
                let mut filter_vals: VecDeque<f64> = VecDeque::with_capacity(filter_size);
                let start_row = row - midpoint;
                let end_row = row + midpoint;
                for col in 0..columns {
                    if col > 0 {
                        filter_vals.pop_front();
                        let mut max_val = f64::NEG_INFINITY;
                        for row2 in start_row..end_row+1 {
                            z_n = erosion[(row2, col + midpoint)];
                            if z_n > max_val && z_n != nodata { max_val = z_n; }
                        }
                        filter_vals.push_back(max_val);
                    } else {
                        // initialize the filter_vals
                        let start_col = col - midpoint;
                        let end_col = col + midpoint;
                        for col2 in start_col..end_col+1 {
                            let mut max_val = f64::NEG_INFINITY;
                            for row2 in start_row..end_row+1 {
                                z_n = erosion[(row2, col2)];
                                if z_n > max_val && z_n != nodata { max_val = z_n; }
                            }
                            filter_vals.push_back(max_val);
                        }
                    }
                    z = input.get_value(row, col);
                    if z != nodata {
                        let mut max_val = f64::NEG_INFINITY;
                        for v in filter_vals.iter() {
                            if *v > max_val { max_val = *v; }
                        }
                        if max_val > f64::NEG_INFINITY {
                            tophat[(row, col)] = z - max_val;
                            opening[(row, col)] = max_val;
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing dilation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        // Back-fill the shallow hills using region growing
        if verbose { println!("Backfilling hills...") };
        let initial_value = f64::NEG_INFINITY;
        let mut out: Array2D<f64> = Array2D::new(rows, columns, initial_value, nodata)?;
        let mut stack: Vec<GridCell> = vec![];
        let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        for row in 0..rows {
            for col in 0..columns {
                out[(row, col)] = initial_value;
                if tophat[(row, col)] != nodata {
                    if tophat[(row, col)] <= height_diff_threshold[1] { // == 0f64 {
                        stack.push(GridCell { row: row, column: col });
                        out[(row, col)] = tophat[(row, col)];
                    }
                } else {
                    out[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding seed cells: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        while stack.len() > 0 {
            let gc = stack.pop().unwrap();
            row = gc.row;
            col = gc.column;
            z = tophat[(row, col)];
            for i in 0..8 {
                row_n = row + d_y[i];
                col_n = col + d_x[i];
                z_n = tophat[(row_n, col_n)];
                if z_n != nodata && out[(row_n, col_n)] == initial_value {
                    if z_n - z < height_diff_threshold[i] {
                        out[(row_n, col_n)] = z_n;
                        stack.push(GridCell { row: row_n, column: col_n });
                    }
                }
            }
        }

        // Interpolate the data holes. Start by locating all the edge cells.
        if verbose { println!("Interpolating data holes...") };
        let mut frs: FixedRadiusSearch2D<f64> = FixedRadiusSearch2D::new(filter_size as f64 / 1.5f64);
        for row in 0..rows {
            for col in 0..columns {
                if tophat[(row, col)] != nodata && out[(row, col)] != initial_value {
                    for i in 0..8 {
                        row_n = row + d_y[i];
                        col_n = col + d_x[i];
                        if tophat[(row_n, col_n)] != nodata && out[(row_n, col_n)] == initial_value {
                            frs.insert(col as f64, row as f64, opening[(row, col)] + tophat[(row, col)]);
                            break;
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding OTO edge cells: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut sum_weights: f64;
        let mut dist: f64;
        for row in 0..rows {
            for col in 0..columns {
                if out[(row, col)] == initial_value {
                    sum_weights = 0f64;
                    let ret = frs.search(col as f64, row as f64);
                    for j in 0..ret.len() {
                        dist = ret[j].1;
                        if dist > 0.0 {
                            sum_weights += 1.0 / (dist * dist);
                        }
                    }
                    z = 0.0;
                    for j in 0..ret.len() {
                        dist = ret[j].1;
                        if dist > 0.0 {
                            z += ret[j].0 * (1.0 / (dist * dist)) / sum_weights;
                        }
                    }
                    out[(row, col)] = z;
                } else {
                    out[(row, col)] = opening[(row, col)] + tophat[(row, col)];
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

        let end = time::now();
        let elapsed_time = end - start;

        // Finally, output the new raster
        let mut output = Raster::initialize_using_file(&output_file, &input);
        for row in 0..rows {
            for col in 0..columns {
                if out[(row, col)] != initial_value && input.get_value(row, col) != nodata {
                    output.set_value(row, col, out[(row, col)]);
                } else {
                    output.set_value(row, col, nodata);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Outputing data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        output.add_metadata_entry("Created by whitebox_tools\' remove_off_terrain_objects tool".to_owned());
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size: {}", filter_size));
        output.add_metadata_entry(format!("Slope threshold: {}", slope_threshold));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

         println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));



        ///////////////////////////////////////////////////////////////////////////////////////////////
        // NOTE:
        // The following disused code is for calculating a tophat transform with a circular shaped
        // structuring element (SE). It's no longer used because the square SE can be used in a way
        // that saves intermediate values and improves performance very considerably.
        ///////////////////////////////////////////////////////////////////////////////////////////////
        //fill the filter kernel cell offset values
        // let num_pixels_in_filter = filter_size * filter_size;
        // let mut d_x = vec![0isize; num_pixels_in_filter];
        // let mut d_y = vec![0isize; num_pixels_in_filter];
        // let mut filter_shape = vec![false; num_pixels_in_filter];
        //
        //see which pixels in the filter lie within the largest ellipse
        //that fits in the filter box
        // let sq = midpoint * midpoint;
        // let mut a = 0usize;
        // for row in 0..filter_size {
        //     for col in 0..filter_size {
        //         d_x[a] = col as isize - midpoint as isize;
        //         d_y[a] = row as isize - midpoint as isize;
        //         z = (d_x[a] * d_x[a]) as f64 / sq as f64 + (d_y[a] * d_y[a]) as f64 / sq as f64;
        //         if z <= 1f64 {
        //             filter_shape[a] = true;
        //         }
        //         a += 1;
        //     }
        // }
        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = input.get_value(row, col);
        //         if z != nodata {
        //             let mut min_val = f64::INFINITY;
        //             for i in 0..num_pixels_in_filter {
        //                 z_n = input.get_value(row + d_y[i], col + d_x[i]);
        //                 if z_n < min_val && filter_shape[i] && z_n != nodata { min_val = z_n }
        //             }
        //             erosion[(row, col)] = min_val;
        //         } else {
        //             erosion[(row, col)] = nodata;
        //             opening[(row, col)] = nodata;
        //             tophat[(row, col)] = nodata;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Performing Erosion: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }
        //
        // let (mut row_n, mut col_n): (isize, isize);
        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = input.get_value(row, col);
        //         if z != nodata {
        //             let mut max_val = f64::NEG_INFINITY;
        //             for i in 0..num_pixels_in_filter {
        //                 col_n = col + d_x[i];
        //                 row_n = row + d_y[i];
        //                 z_n = erosion[(row_n, col_n)];
        //                 if z_n > max_val && filter_shape[i] && z_n != nodata { max_val = z_n }
        //             }
        //             tophat[(row, col)] = z - max_val;
        //             opening[(row, col)] = max_val;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Performing Dilation: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }



        ///////////////////////////////////////////////////////////////////////////////////////////////
        // NOTE:
        // This disused code perfomed peak cleaving using a modified depression filling algorithm on
        // the tophat transform. The current method of region growing is more straight forward.
        ///////////////////////////////////////////////////////////////////////////////////////////////
        // find grid cells with nodata neighbours
        // let multiplier = 10000f64;
        // let mut heap = BinaryHeap::new();
        // let initial_value = f64::NEG_INFINITY;
        // let mut num_solved_cells = 0usize;
        // let num_cells = rows * columns;
        // let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        // let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        // for row in 0..rows as isize {
        //     for col in 0..columns as isize {
        //         output.set_value(row, col, initial_value);
        //         z = input.get_value(row, col);
        //         if z != nodata {
        //             let mut flag = false;
        //             for i in 0..8 {
        //                 z_n = input.get_value(row + d_y[i], col + d_x[i]);
        //                 if z_n == nodata {
        //                     flag = true;
        //                 }
        //             }
        //             if flag {
        //                 heap.push(GridCell { priority: -(tophat[row as usize][col as usize] * multiplier).floor() as isize, row: row, column: col });
        //                 output.set_value(row, col, tophat[row as usize][col as usize]);
        //                 num_solved_cells += 1;
        //             }
        //         } else {
        //             output.set_value(row, col, nodata);
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }
        //
        // let (mut row, mut col): (isize, isize);
        // let mut frs: FixedRadiusSearch<f64> = FixedRadiusSearch::new(filter_size as f64);
        // let mut modified = vec![vec![false; columns]; rows];
        // while heap.len() > 0 {
        //     let gc = heap.pop().unwrap();
        //     row = gc.row;
        //     col = gc.column;
        //     z = -(gc.priority as f64 / multiplier);
        //     for i in 0..8 {
        //         row_n = row + d_y[i];
        //         col_n = col + d_x[i];
        //         if col_n >= 0 && col_n < columns as isize && row_n >= 0 && row_n < rows as isize {
        //             z_n = tophat[row_n as usize][col_n as usize];
        //             if z_n != nodata && output.get_value(row_n, col_n) == initial_value {
        //                 if z_n - z >= height_diff_threshold { //z_n >= z {
        //                     z_n = z;
        //                     modified[row_n as usize][col_n as usize] = true;
        //                     if !modified[row as usize][col as usize] {
        //                         frs.insert(col as f64, row as f64, tophat[row as usize][col as usize]);
        //                     }
        //                 }
        //                 output.set_value(row_n, col_n, z_n);
        //                 num_solved_cells += 1;
        //                 heap.push(GridCell { priority: -(z_n * multiplier).floor() as isize, row: row_n, column: col_n });
        //             }
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }
        //
        // let mut sum_weights: f64;
        // let mut dist: f64;
        // for row in 0..rows as isize {
        //     for col in 0..columns as isize {
        //         if opening[row as usize][col as usize] != nodata {
        //             if modified[row as usize][col as usize] {
        //                 sum_weights = 0f64;
        //                 let ret = frs.search(col as f64, row as f64);
        //                 for j in 0..ret.len() {
        //                     dist = ret[j].1;
        //                     if dist > 0.0 {
        //                         sum_weights += 1.0 / (dist * dist);
        //                     }
        //                 }
        //                 z = 0.0;
        //                 for j in 0..ret.len() {
        //                     dist = ret[j].1;
        //                     if dist > 0.0 {
        //                         z += ret[j].0 * (1.0 / (dist * dist)) / sum_weights;
        //                     }
        //                 }
        //                 output.set_value(row, col, -z);
        //             }
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }
        //
        // let output_dem = true;
        // if output_dem {
        //     for row in 0..rows as isize {
        //         for col in 0..columns as isize {
        //             // if opening[row as usize][col as usize] != nodata {
        //             //     z = output.get_value(row, col);
        //             //     output.set_value(row, col, opening[row as usize][col as usize] + z);
        //             // }
        //             if !modified[row as usize][col as usize] {
        //                 z = output.get_value(row, col);
        //                 output.set_value(row, col, opening[row as usize][col as usize] + z);
        //             } else {
        //                 output.set_value(row, col, nodata);
        //             }
        //         }
        //         if verbose {
        //             progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Progress: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }
        // }

        // println!("Saving data...");
        // let _ = match output.write() {
        //     Ok(_) => println!("Output file written"),
        //     Err(e) => return Err(e),
        // };

        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct GridCell {
    // priority: isize,
    row: isize,
    column: isize,
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap instead of a max-heap.
// impl Ord for GridCell {
//     fn cmp(&self, other: &GridCell) -> Ordering {
//         // Notice that the we flip the ordering here
//         other.priority.cmp(&self.priority)
//     }
// }
//
// // `PartialOrd` needs to be implemented as well.
// impl PartialOrd for GridCell {
//     fn partial_cmp(&self, other: &GridCell) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }
