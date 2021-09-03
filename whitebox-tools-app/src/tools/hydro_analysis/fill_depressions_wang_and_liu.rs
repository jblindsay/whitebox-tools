/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 28/06/2017
Last Modified: 05/12/2019
License: MIT

NOTE: This tool was originally named FillDepressions. However, I have updated the algorithm used by the
FillDepressions tool to something that is often more efficient than the Wang and Lui method. As such,
I have created this tool to house the original Wang and Lui based depression filling method for
legacy reasons.
*/

use whitebox_raster::*;
use crate::tools::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::i32;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the
/// flat areas. This is a common pre-processing step required by many flow-path analysis tools to ensure continuous
/// flow from each grid cell to an outlet located along the grid edge. The `FillDepressionsWangAndLiu` algorithm is based on
/// the computationally efficient approach of examining each cell based on its spill elevation, starting from the
/// edge cells, and visiting cells from lowest order using a priority queue. As such, it is based on the algorithm
/// first proposed by Wang and Liu (2006). However, it is currently not the most efficient depression-removal algorithm
/// available in WhiteboxTools; `FillDepressions` and `BreachDepressionsLeastCost` are both more efficient and often
/// produce better, lower-impact results.
///
/// If the input DEM has gaps, or missing-data holes, that contain NoData values, it is better to use the
/// `FillMissingData` tool to repair these gaps. This tool will interpolate values across the gaps and produce
/// a more natural-looking surface than the flat areas that are produced by depression filling. Importantly, the
/// `FillDepressions` tool algorithm implementation assumes that there are no 'donut hole' NoData gaps within the area
/// of valid data. Any NoData areas along the edge of the grid will simply be ignored and will remain NoData areas in
/// the output image.
///
/// The user may optionally specify the size of the elevation increment used to solve flats (`--flat_increment`), although
/// **it is best not to specify this optional value and to let the algorithm determine the most suitable value itself**.
/// If a flat increment value isn't specified, the output DEM will use 64-bit floating point values in order
/// to make sure that the very small elevation increment value determined will be accurately stored. Consequently,
/// it may double the storage requirements as DEMs are often stored with 32-bit precision. However, if a flat increment
/// value is specified, the output DEM will keep the same data type as the input assuming the user chose its value wisely.
///
/// # Reference
/// Wang, L. and Liu, H. 2006. An efficient method for identifying and filling surface depressions in digital elevation
/// models for hydrologic analysis and modelling. International Journal of Geographical Information Science, 20(2): 193-213.
///
/// # See Also
/// `FillDepressions`, `BreachDepressionsLeastCost`, `BreachDepressions`, `FillMissingData`
pub struct FillDepressionsWangAndLiu {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FillDepressionsWangAndLiu {
    pub fn new() -> FillDepressionsWangAndLiu {
        // public constructor
        let name = "FillDepressionsWangAndLiu".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Fills all of the depressions in a DEM using the Wang and Liu (2006) method. Depression breaching should be preferred in most cases.".to_string();

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
            name: "Fix flat areas?".to_owned(),
            flags: vec!["--fix_flats".to_owned()],
            description:
                "Optional flag indicating whether flat areas should have a small gradient applied."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Flat increment value (z units)".to_owned(),
            flags: vec!["--flat_increment".to_owned()],
            description: "Optional elevation increment applied to flat areas.".to_owned(),
            parameter_type: ParameterType::Float,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --fix_flats",
            short_exe, name
        )
        .replace("*", &sep);

        FillDepressionsWangAndLiu {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FillDepressionsWangAndLiu {
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
        let mut fix_flats = false;
        let mut flat_increment = f64::NAN;

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
            } else if flag_val == "-fix_flats" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    fix_flats = true;
                }
            } else if flag_val == "-flat_increment" {
                flat_increment = if keyval {
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

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;

        // let min_val = input.configs.minimum;
        // let elev_digits = ((input.configs.maximum - min_val) as i64).to_string().len();
        // let elev_multiplier = 10.0_f64.powi((7 - elev_digits) as i32);
        // let mut small_num = 0.0;
        // if fix_flats {
        //     small_num = 1.0 / elev_multiplier as f64;
        // }
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let background_val = (i32::min_value() + 1) as f64;
        output.reinitialize_values(background_val);

        let small_num = if fix_flats && !flat_increment.is_nan() {
            output.configs.data_type = input.configs.data_type; // Assume the user knows what he's doing
            flat_increment
        } else if fix_flats {
            output.configs.data_type = DataType::F64; // Don't take any chances and promote to 64-bit
            let resx = input.configs.resolution_x;
            let resy = input.configs.resolution_y;
            let diagres = (resx * resx + resy * resy).sqrt();
            let elev_digits = (input.configs.maximum as i64).to_string().len();
            let elev_multiplier = 10.0_f64.powi((15 - elev_digits) as i32);
            1.0_f64 / elev_multiplier as f64 * diagres.ceil()
        } else {
            output.configs.data_type = input.configs.data_type;
            0f64
        };
        

        /*
        Find the data edges. This is complicated by the fact that DEMs frequently
        have nodata edges, whereby the DEM does not occupy the full extent of
        the raster. One approach to doing this would be simply to scan the
        raster, looking for cells that neighbour nodata values. However, this
        assumes that there are no interior nodata holes in the dataset. Instead,
        the approach used here is to perform a region-growing operation, looking
        for nodata values along the raster's edges.
        */

        let mut queue: VecDeque<(isize, isize)> =
            VecDeque::with_capacity((rows * columns) as usize);
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
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            row = cell.0;
            col = cell.1;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                zin_n = input.get_value(row_n, col_n);
                zout_n = output[(row_n, col_n)];
                if zout_n == background_val {
                    if zin_n == nodata {
                        output.set_value(row_n, col_n, nodata);
                        queue.push_back((row_n, col_n));
                    } else {
                        output[(row_n, col_n)] = zin_n;
                        // Push it onto the priority queue for the priority flood operation
                        minheap.push(GridCell {
                            row: row_n,
                            column: col_n,
                            priority: zin_n,
                        });
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
            let cell = minheap.pop().expect("Error during pop operation.");
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
                        if zin_n < (zout + small_num) {
                            zin_n = zout + small_num;
                        } // We're in a depression. Raise the elevation.
                        output[(row_n, col_n)] = zin_n;
                        minheap.push(GridCell {
                            row: row_n,
                            column: col_n,
                            priority: zin_n,
                        });
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
        //     let cell = minheap.pop().expect("Error during pop operation.");
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

        /* This was an experiment with an approach that reduced the reliance on the priority queue.
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
        //     let cell = minheap.pop().expect("Error during pop operation.");
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Fix flats: {}", fix_flats));
        if fix_flats {
            output.add_metadata_entry(format!("Flat increment value: {}", small_num));
        }
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
