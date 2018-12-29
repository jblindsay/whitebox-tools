/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 4, 2017
Last Modified: 15/11/2018
License: MIT

NOTES: Add anisotropy option.
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::f64;
use std::i32;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to perform cost-distance or least-cost pathway analyses. Specifically,
/// this tool can be used to calculate the accumulated cost of traveling from the 'source grid
/// cell' to each other grid cell in a raster dataset. It is based on the costs associated with
/// traveling through each cell along a pathway represented in a cost (or friction) surface. If
/// there are multiple source grid cells, each cell in the resulting cost-accumulation surface
/// will reflect the accumulated cost to the source cell that is connected by the minimum accumulated
/// cost-path. The user must specify the names of the raster file containing the source cells
/// (`--source`), the raster file containing the cost surface information (`--cost`), the output
/// cost-accumulation surface raster (`--out_accum`), and the output back-link raster (`--out_backlink`).
/// Source cells are designated as all positive, non-zero valued grid cells in the source raster.
/// The cost (friction) raster can be created by combining the various cost factors associated with
/// the specific problem (e.g. slope gradient, visibility, etc.) using a raster calculator or the
/// `WeightedOverlay` tool.
///
/// While the cost-accumulation surface raster can be helpful for visualizing
/// the three-dimensional characteristics of the 'cost landscape', it is actually the back-link raster
/// that is used as inputs to the other two cost-distance tools, `CostAllocation` and `CostPathway`, to
/// determine the least-cost linkages among neighbouring grid cells on the cost surface. If the
/// accumulated cost surface is analogous to a digital elevation model (DEM) then the back-link raster
/// is equivalent to the D8 flow-direction pointer. In fact, it is created in a similar way and uses
/// the same convention for designating 'flow directions' between neighbouring grid cells. The algorithm
/// for the cost distance accumulation operation uses a type of priority-flood method similar to
/// what is used for depression filling and flow accumulation operations.
///
/// NoData values in the input cost surface image are ignored during processing and assigned NoData values
/// in the outputs. The output cost accumulation raster is of the float data type and continuous data scale.
///
/// # See Also
/// `CostAllocation`, `CostPathway`, `WeightedOverlay`
pub struct CostDistance {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CostDistance {
    pub fn new() -> CostDistance {
        // public constructor
        let name = "CostDistance".to_string();
        let toolbox = "GIS Analysis/Distance Tools".to_string();
        let description =
            "Performs cost-distance accumulation on a cost surface and a group of source cells."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Source File".to_owned(),
            flags: vec!["--source".to_owned()],
            description: "Input source raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Cost (Friction) File".to_owned(),
            flags: vec!["--cost".to_owned()],
            description: "Input cost (friction) raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Cost Accumulation File".to_owned(),
            flags: vec!["--out_accum".to_owned()],
            description: "Output cost accumulation raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Backlink File".to_owned(),
            flags: vec!["--out_backlink".to_owned()],
            description: "Output backlink raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --source=src.tif --cost=cost.tif --out_accum=accum.tif --out_backlink=backlink.tif", short_exe, name).replace("*", &sep);

        CostDistance {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CostDistance {
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
        let mut source_file = String::new();
        let mut cost_file = String::new();
        let mut accum_file = String::new();
        let mut backlink_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            if flag_val == "-source" {
                source_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-cost" {
                cost_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-out_accum" {
                accum_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-out_backlink" {
                backlink_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
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

        if !source_file.contains(&sep) && !source_file.contains("/") {
            source_file = format!("{}{}", working_directory, source_file);
        }
        if !cost_file.contains(&sep) && !cost_file.contains("/") {
            cost_file = format!("{}{}", working_directory, cost_file);
        }
        if !accum_file.contains(&sep) && !accum_file.contains("/") {
            accum_file = format!("{}{}", working_directory, accum_file);
        }
        if !backlink_file.contains(&sep) && !backlink_file.contains("/") {
            backlink_file = format!("{}{}", working_directory, backlink_file);
        }

        if verbose {
            println!("Reading source data...")
        };
        let source = Raster::new(&source_file, "r")?;

        if verbose {
            println!("Reading cost data...")
        };
        let cost = Raster::new(&cost_file, "r")?;

        // make sure the input files have the same size
        if source.configs.rows != cost.configs.rows
            || source.configs.columns != cost.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let start = Instant::now();
        let rows = source.configs.rows as isize;
        let columns = source.configs.columns as isize;
        let num_cells = (rows * columns) as usize;
        let nodata = cost.configs.nodata;

        let mut output = Raster::initialize_using_file(&accum_file, &cost);
        let background_val = (i32::max_value() - 1) as f64;
        output.reinitialize_values(background_val);

        let mut backlink = Raster::initialize_using_file(&backlink_file, &cost);

        let mut minheap = BinaryHeap::with_capacity(num_cells);

        let mut solved_cells = 0;
        for row in 0..rows {
            for col in 0..columns {
                if source.get_value(row, col) > 0.0 && cost.get_value(row, col) != nodata {
                    output.set_value(row, col, 0.0);
                    backlink.set_value(row, col, 0.0);
                    minheap.push(GridCell {
                        row: row,
                        column: col,
                        priority: 0f64,
                    });
                    solved_cells += 1;
                } else if cost.get_value(row, col) == nodata {
                    output.set_value(row, col, nodata);
                    solved_cells += 1;
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
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let cell_size_x = source.configs.resolution_x;
        let cell_size_y = source.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dist = [
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
        ];
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let backlink_dir = [16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0];
        let mut solved: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        while !minheap.is_empty() {
            let cell = minheap.pop().unwrap();
            row = cell.row;
            col = cell.column;
            if solved.get_value(row, col) == 0 {
                solved.set_value(row, col, 1);
                solved_cells += 1;
                accum_val = output.get_value(row, col);
                cost1 = cost.get_value(row, col);
                for n in 0..8 {
                    col_n = col + dx[n];
                    row_n = row + dy[n];
                    if output.get_value(row_n, col_n) != nodata {
                        cost2 = cost.get_value(row_n, col_n);
                        new_cost = accum_val + (cost1 + cost2) / 2.0 * dist[n];
                        if new_cost < output.get_value(row_n, col_n) {
                            if solved.get_value(row_n, col_n) == 0 {
                                output.set_value(row_n, col_n, new_cost);
                                backlink.set_value(row_n, col_n, backlink_dir[n]);
                                minheap.push(GridCell {
                                    row: row_n,
                                    column: col_n,
                                    priority: new_cost,
                                });
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * solved_cells as f64 / (num_cells - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        /*
        let mut new_cost: f64;
        let mut accum_val: f64;
        let (mut cost1, mut cost2): (f64, f64);
        let (mut row_n, mut col_n): (isize, isize);
        let cell_size_x = source.configs.resolution_x;
        let cell_size_y = source.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dist = [
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
        ];
        let dx = [1, 1, 0, -1, -1, -1, 0, 1];
        let dy = [0, 1, 1, 1, 0, -1, -1, -1];
        let backlink_dir = [32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0, 16.0];
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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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

            if !did_something {
                break;
            }

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
        */

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Source raster file: {}", source_file));
        output.add_metadata_entry(format!("Cost raster: {}", cost_file));
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

        backlink.configs.palette = "qual.plt".to_string();
        backlink.configs.photometric_interp = PhotometricInterpretation::Categorical;
        backlink.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        backlink.add_metadata_entry(format!("Source raster file: {}", source_file));
        backlink.add_metadata_entry(format!("Cost raster: {}", cost_file));
        backlink.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        let _ = match backlink.write() {
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
