/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 28/06/2017
Last Modified: 24/11/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::i32;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to remove the depressions in a digital elevation model (DEM), a
/// common requirement of spatial hydrological operations such as flow accumulation
/// and watershed modelling. The tool based on the efficient hybrid depression
/// breaching algorithm described by Lindsay (2016). It uses a breach-first, fill-second
/// approach to resolving continuous flowpaths through depressions.
///
/// Notice that when the input DEM (`--dem`) contains deep, single-cell pits, it can be useful
/// to raise the pits elevation to that of the lowest neighbour (`--fill_pits`), to avoid the
/// creation of deep breach trenches. Deep pits can be common in DEMs containing speckle-type noise.
/// This option, however, does add slightly to the computation time of the tool.
///
/// The user may optionally (`--flat_increment`) override the default value applied to increment elevations on
/// flat areas (often formed by the subsequent depression filling operation). The default value is
/// dependent upon the elevation range in the input DEM and is generally a very small elevation value (e.g.
/// 0.001). It may be necessary to override the default elevation increment value in landscapes where there
/// are extensive flat areas resulting from depression filling (and along breach channels). Values in the range
/// 0.00001 to 0.01 are generally appropriate. Increment values that are too large can result in obvious artifacts
/// along flattened sites, which may extend beyond the flats, and values that are too small (i.e. smaller than the
/// numerical precision) may result in the presence of grid cells with no downslope neighbour in the
/// output DEM. If a flat increment value isn't specified, the output DEM will use 64-bit floating point values in order
/// to make sure that the very small elevation increment value determined will be accurately stored. Consequently,
/// it may double the storage requirements as DEMs are often stored with 32-bit precision. However, if a flat increment
/// value is specified, the output DEM will keep the same data type as the input assuming the user chose its value wisely.
///
/// In comparison with the `BreachDepressionsLeastCost` tool, this breaching method often provides a less
/// satisfactory, higher impact, breaching solution and is often less efficient. **It has been provided to users for
/// legacy reasons and it is advisable that users try the `BreachDepressionsLeastCost` tool to remove depressions from
/// their DEMs first**. The `BreachDepressionsLeastCost` tool is particularly
/// well suited to breaching through road embankments. Nonetheless, there are applications for which full depression filling
/// using the  `FillDepressions` tool may be preferred.
///
/// # Reference
/// Lindsay JB. 2016. *Efficient hybrid breaching-filling sink removal methods for
/// flow path enforcement in digital elevation models.* **Hydrological Processes**,
/// 30(6): 846–857. DOI: 10.1002/hyp.10648
///
/// # See Also
/// `BreachDepressionsLeastCost`, `FillDepressions`, `FillSingleCellPits`
pub struct BreachDepressions {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BreachDepressions {
    pub fn new() -> BreachDepressions {
        // public constructor
        let name = "BreachDepressions".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Breaches all of the depressions in a DEM using Lindsay's (2016) algorithm. This should be preferred over depression filling in most cases.".to_string();

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
            name: "Maximum Breach Depth (z units)".to_owned(),
            flags: vec!["--max_depth".to_owned()],
            description: "Optional maximum breach depth (default is Inf).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Breach Channel Length (grid cells)".to_owned(),
            flags: vec!["--max_length".to_owned()],
            description: "Optional maximum breach channel length (in grid cells; default is Inf)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
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

        parameters.push(ToolParameter {
            name: "Fill single-cell pits?".to_owned(),
            flags: vec!["--fill_pits".to_owned()],
            description: "Optional flag indicating whether to fill single-cell pits.".to_owned(),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        BreachDepressions {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BreachDepressions {
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
        let mut max_depth = f64::INFINITY;
        let mut max_length = f64::INFINITY;
        let mut constrained_mode = false;
        let mut flat_increment = f64::NAN;
        let mut fill_pits = false;

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
            } else if flag_val == "-max_depth" {
                max_depth = if keyval {
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
                constrained_mode = true;
            } else if flag_val == "-max_length" {
                max_length = if keyval {
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
                constrained_mode = true;
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
            } else if flag_val == "-fill_pits" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    fill_pits = true;
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

        if verbose && constrained_mode {
            println!("Breaching in constrained mode...");
        }

        let mut input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let resx = input.configs.resolution_x;
        let resy = input.configs.resolution_y;
        let diagres = (resx * resx + resy * resy).sqrt();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let background_val = (i32::min_value() + 1) as f64;
        output.reinitialize_values(background_val);

        let small_num = if !flat_increment.is_nan() || flat_increment == 0f64 {
            output.configs.data_type = input.configs.data_type; // Assume the user knows what he's doing
            flat_increment
        } else {
            output.configs.data_type = DataType::F64; // Don't take any chances and promote to 64-bit
            let elev_digits = (input.configs.maximum as i64).to_string().len();
            let elev_multiplier = 10.0_f64.powi((6 - elev_digits) as i32);
            1.0_f64 / elev_multiplier as f64 * diagres.ceil()
        };

        let mut z: f64;
        let mut z_n: f64;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        if fill_pits {
            // Fill the single-cell pits before breaching. This can prevent the creation of
            // very deep breach trenches.
            let mut min_zn: f64;
            let mut flag: bool;
            for row in 1..rows - 1 {
                for col in 1..columns - 1 {
                    z = input.get_value(row, col);
                    if z != nodata {
                        flag = true;
                        min_zn = f64::INFINITY;
                        for n in 0..8 {
                            z_n = input.get_value(row + dy[n], col + dx[n]);
                            if z_n < min_zn && z_n != nodata {
                                min_zn = z_n;
                            }
                            if z_n < z && z_n != nodata {
                                flag = false;
                                break;
                            }
                        }
                        if flag {
                            input.set_value(row, col, min_zn - small_num);
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Filling pits: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }


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
                zout_n = output.get_value(row_n, col_n);
                if zout_n == background_val {
                    if zin_n == nodata {
                        output.set_value(row_n, col_n, nodata);
                        queue.push_back((row_n, col_n));
                    } else {
                        output.set_value(row_n, col_n, zin_n);
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
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Perform the priority flood operation.
        let back_link = [4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8];
        let (mut x, mut y): (isize, isize);
        let mut z_target: f64;
        let mut dir: i8;
        let mut flag: bool;

        if !constrained_mode {
            while !minheap.is_empty() {
                let cell = minheap.pop().expect("Error during pop operation.");
                row = cell.row;
                col = cell.column;
                zout = output.get_value(row, col);
                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = col + dx[n];
                    zout_n = output.get_value(row_n, col_n);
                    if zout_n == background_val {
                        zin_n = input.get_value(row_n, col_n);
                        if zin_n != nodata {
                            flow_dir.set_value(row_n, col_n, back_link[n]);
                            output.set_value(row_n, col_n, zin_n);
                            minheap.push(GridCell {
                                row: row_n,
                                column: col_n,
                                priority: zin_n,
                            });
                            if zin_n < (zout + small_num) {
                                // Trace the flowpath back to a lower cell, if it exists.
                                x = col_n;
                                y = row_n;
                                z_target = output.get_value(row_n, col_n);
                                flag = true;
                                while flag {
                                    dir = flow_dir[(y, x)];
                                    if dir >= 0 {
                                        y += dy[dir as usize];
                                        x += dx[dir as usize];
                                        z_target -= small_num;
                                        if output.get_value(y, x) > z_target {
                                            output.set_value(y, x, z_target);
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
                            output.set_value(row_n, col_n, nodata);
                            num_solved_cells += 1;
                            // region growing operation to find all attached nodata cells
                            queue.push_back((row_n, col_n));
                            while !queue.is_empty() {
                                let cell = queue.pop_front().unwrap();
                                for n2 in 0..8 {
                                    let row2 = cell.0 + dy[n2];
                                    let col2 = cell.1 + dx[n2];
                                    if input.get_value(row2, col2) == nodata
                                        && output.get_value(row2, col2) == background_val
                                    {
                                        if row2 >= 0 && row2 < rows && col2 >= 0 && col2 < columns {
                                            output.set_value(row2, col2, nodata);
                                            num_solved_cells += 1;
                                            queue.push_back((row2, col2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if verbose {
                    num_solved_cells += 1;
                    progress =
                        (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // constrained mode
            let mut channel_depth: f64;
            let mut channel_length: f64;
            let mut carved_depth: f64;
            let mut floodorder = Vec::with_capacity((rows * columns) as usize);
            let mut unresolved_pits = false;
            // let mut flood_order_tail = 0usize;
            while !minheap.is_empty() {
                let cell = minheap.pop().expect("Error during pop operation.");
                row = cell.row;
                col = cell.column;
                floodorder.push(row * columns + col);
                // flood_order_tail += 1;
                zout = output.get_value(row, col);
                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = col + dx[n];
                    zout_n = output.get_value(row_n, col_n);
                    if zout_n == background_val {
                        zin_n = input.get_value(row_n, col_n);
                        if zin_n != nodata {
                            flow_dir.set_value(row_n, col_n, back_link[n]);
                            output.set_value(row_n, col_n, zin_n);
                            minheap.push(GridCell {
                                row: row_n,
                                column: col_n,
                                priority: zin_n,
                            });
                            if zin_n < (zout + small_num) {
                                // Trace the flowpath back to a lower cell, if it exists.
                                x = col_n;
                                y = row_n;
                                z_target = output.get_value(row_n, col_n);
                                channel_depth = 0.0;
                                channel_length = 0.0;
                                flag = true;
                                while flag {
                                    dir = flow_dir.get_value(y, x);
                                    if dir >= 0 {
                                        y += dy[dir as usize];
                                        x += dx[dir as usize];
                                        z_target -= small_num;
                                        channel_length += 1.0;
                                        if output.get_value(y, x) > z_target {
                                            carved_depth = input.get_value(y, x) - z_target;
                                            if carved_depth > channel_depth {
                                                channel_depth = carved_depth;
                                            }
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
                                    z_target = output.get_value(row_n, col_n);
                                    flag = true;
                                    while flag {
                                        dir = flow_dir.get_value(y, x);
                                        if dir >= 0 {
                                            y += dy[dir as usize];
                                            x += dx[dir as usize];
                                            z_target -= small_num;
                                            if output.get_value(y, x) > z_target {
                                                output.set_value(y, x, z_target);
                                            } else {
                                                flag = false;
                                            }
                                        } else {
                                            flag = false;
                                        }
                                    }
                                } else {
                                    // let optimal_search = max_length.round() as isize;
                                    // let optimal_filter_size = 2 * optimal_search + 1;
                                    // let (mut j, mut k): (isize, isize);
                                    // let large_value = f64::MAX;
                                    // let mut zn: f64;
                                    // let (mut cost1, mut cost2, mut new_cost): (f64, f64, f64);
                                    // let mut accum_val: f64;
                                    // let mut cost: Array2D<f64> = Array2D::new(optimal_filter_size, optimal_filter_size, f64::MAX, nodata)?;
                                    // let mut accumulatedcost: Array2D<f64> = Array2D::new(optimal_filter_size, optimal_filter_size, f64::MAX, nodata)?;
                                    // let mut backlink: Array2D<i8> = Array2D::new(optimal_filter_size, optimal_filter_size, -1, -1)?;
                                    // let mut solved: Array2D<i8> = Array2D::new(optimal_filter_size, optimal_filter_size, 0, -1)?;
                                    // let mut costheap = BinaryHeap::with_capacity((optimal_filter_size * optimal_filter_size) as usize);
                                    // let cell_size_x = input.configs.resolution_x;
                                    // let cell_size_y = input.configs.resolution_y;
                                    // let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
                                    // let dist = [
                                    //     diag_cell_size,
                                    //     cell_size_x,
                                    //     diag_cell_size,
                                    //     cell_size_y,
                                    //     diag_cell_size,
                                    //     cell_size_x,
                                    //     diag_cell_size,
                                    //     cell_size_y,
                                    // ];
                                    // for row_offset in -optimal_search..=optimal_search {
                                    //     for col_offset in -optimal_search..=optimal_search {
                                    //         zn = output.get_value(row_n + row_offset, col_n + col_offset);
                                    //         j = row_offset + optimal_search;
                                    //         k = col_offset + optimal_search;
                                    //         if zn < zout && zn != nodata && zn != background_val {
                                    //             cost.set_value(j, k, 0f64);
                                    //             accumulatedcost.set_value(j, k, 0f64);
                                    //             costheap.push(GridCell {
                                    //                 row: j,
                                    //                 column: k,
                                    //                 priority: 0f64,
                                    //             });
                                    //             // backlink.set_value(j, k, 0);
                                    //         } else if zn >= zout {
                                    //             cost1 = zn - zout;
                                    //             if cost1 < max_depth {
                                    //                 cost.set_value(j, k, zn - zout);
                                    //             } else {
                                    //                 cost.set_value(j, k, large_value);
                                    //             }
                                    //             accumulatedcost.set_value(j, k, large_value);
                                    //         } else { // nodata, background cell, or lower but not yet flooded.
                                    //             cost.set_value(j, k, nodata);
                                    //             accumulatedcost.set_value(j, k, nodata);
                                    //             solved.set_value(j, k, 1);
                                    //         }
                                    //     }
                                    // }
                                    // if !costheap.is_empty() {
                                    //     // println!("I'm here");
                                    //     while !costheap.is_empty() {
                                    //         let cell = costheap.pop().expect("Error during pop operation.");
                                    //         if solved.get_value(cell.row, cell.column) == 0 {
                                    //             solved.set_value(cell.row, cell.column, 1);
                                    //             accum_val = accumulatedcost.get_value(cell.row, cell.column);
                                    //             cost1 = cost.get_value(cell.row, cell.column);
                                    //             for n in 0..8 {
                                    //                 j = cell.row + dy[n];
                                    //                 k = cell.column + dx[n];
                                    //                 if accumulatedcost.get_value(j, k) != nodata {
                                    //                     cost2 = cost.get_value(j, k);
                                    //                     new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                                    //                     if new_cost < accumulatedcost.get_value(j, k) {
                                    //                         if solved.get_value(j, k) == 0 {
                                    //                             accumulatedcost.set_value(j, k, new_cost);
                                    //                             backlink.set_value(j, k, back_link[n]);
                                    //                             costheap.push(GridCell {
                                    //                                 row: j,
                                    //                                 column: k,
                                    //                                 priority: new_cost,
                                    //                             });
                                    //                         }
                                    //                     }
                                    //                 }
                                    //             }
                                    //         }
                                    //     }
                                    //     // now trace the path from row, col to the nearest source, carving the breach path.
                                    //     j = row;
                                    //     k = col;
                                    //     let mut flag = true;
                                    //     while flag {

                                    //     }
                                    // } else {
                                    unresolved_pits = true;
                                    // }
                                }
                            }
                        } else {
                            // Interior nodata cells are still treated as nodata and are not filled.
                            output.set_value(row_n, col_n, nodata);
                            num_solved_cells += 1;
                            // region growing operation to find all attached nodata cells
                            queue.push_back((row_n, col_n));
                            while !queue.is_empty() {
                                let cell = queue.pop_front().unwrap();
                                for n2 in 0..8 {
                                    let row2 = cell.0 + dy[n2];
                                    let col2 = cell.1 + dx[n2];
                                    if input.get_value(row2, col2) == nodata
                                        && output.get_value(row2, col2) == background_val
                                    {
                                        if row2 >= 0 && row2 < rows && col2 >= 0 && col2 < columns {
                                            output.set_value(row2, col2, nodata);
                                            num_solved_cells += 1;
                                            queue.push_back((row2, col2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if verbose {
                    num_solved_cells += 1;
                    progress =
                        (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // if unresolved_pits && verbose {
            //     println!("There were unbreached depressions. The result should be filled to remove additional depressions.");
            // }
            if unresolved_pits {
                // Fill the DEM.
                num_solved_cells = 0;
                let num_valid_cells = floodorder.len();
                for c in 0..num_valid_cells {
                    row = floodorder[c] / columns;
                    col = floodorder[c] % columns;
                    if row >= 0 && col >= 0 {
                        z = output.get_value(row, col);
                        dir = flow_dir.get_value(row, col);
                        if dir >= 0 {
                            row_n = row + dy[dir as usize];
                            col_n = col + dx[dir as usize];
                            z_n = output.get_value(row_n, col_n);
                            if z_n != nodata {
                                if z <= z_n + small_num {
                                    output.set_value(row, col, z_n + small_num);
                                }
                            }
                        }
                    }
                    if verbose {
                        num_solved_cells += 1;
                        progress =
                            (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Filling DEM: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Fill pits: {}", fill_pits));
        if constrained_mode {
            output.add_metadata_entry(format!("Maximum breach depth: {}", max_depth));
            output.add_metadata_entry(format!("Maximum breach channel length: {}", max_length));
        }
        output.add_metadata_entry(format!("Flat elevation increment: {}", small_num));
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
