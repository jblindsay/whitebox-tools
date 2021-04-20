/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/07/2020
Last Modified: 27/07/2020
License: MIT
*/

use whitebox_raster::*;
// use crate::structures::Array2D;
use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::{f32, f64};

/// This tool can be used to map off-terrain objects in a digital surface model (DSM) based on cell-to-cell differences
/// in elevations and local slopes. The algorithm works by using a region-growing operation to connect neighbouring grid
/// cells outwards from seed cells. Two neighbouring cells are considered connected if the slope between the two cells
/// is less than the user-specified maximum slope value (`--max_slope`). Mapped segments that are less than the minimum
/// feature size (`--min_size`), in grid cells, are assigned a common background value. Note that this method of mapping
/// off-terrain objects, and thereby separating ground cells from non-ground objects in DSMs, works best with fine-resolution
/// DSMs that have been interpolated using a non-smoothing method, such as triangulation (TINing) or nearest-neighbour
/// interpolation.
///
/// # See Also
/// `RemoveOffTerrainObjects`
pub struct MapOffTerrainObjects {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MapOffTerrainObjects {
    pub fn new() -> MapOffTerrainObjects {
        // public constructor
        let name = "MapOffTerrainObjects".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Maps off-terrain objects in a digital elevation model (DEM).".to_string();

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
            name: "Maximum Slope".to_owned(),
            flags: vec!["--max_slope".to_owned()],
            description: "Maximum inter-cell absolute slope.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("40.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Feature Size".to_owned(),
            flags: vec!["--min_size".to_owned()],
            description: "Minimum feature size, in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_owned()),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --max_diff=1.0",
            short_exe, name
        )
        .replace("*", &sep);

        MapOffTerrainObjects {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MapOffTerrainObjects {
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
        let mut max_slope = f32::INFINITY;
        let mut min_size = 0usize;

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
            } else if flag_val == "-max_slope" {
                max_slope = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-min_size" {
                min_size = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<usize>()
                        .expect(&format!("Error parsing {}", flag_val))
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if max_slope < 1f32 {
            max_slope = 1f32;
        }
        if max_slope > 90f32 {
            max_slope = 90f32;
        }
        max_slope = max_slope.to_radians().tan();

        if verbose {
            println!("Reading data...")
        }

        let input_dem = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let input = input_dem.get_data_as_f32_array2d();
        let configs = input_dem.configs.clone();
        drop(input_dem);

        let rows = input.rows as isize;
        let columns = input.columns as isize;
        let nodata = input.nodata;
        let res_x = configs.resolution_x as f32;
        let res_y = configs.resolution_y as f32;
        let res_diag = res_x.hypot(res_y);

        /////////////////////////////////////////////
        // Performing the region-growing operation //
        /////////////////////////////////////////////

        if verbose {
            println!("Performing the region-growing operation...");
        }

        let mut output = Raster::initialize_using_config(&output_file, &configs);
        output.configs.data_type = DataType::F32;
        output.configs.nodata = -32768.0;
        output.reinitialize_values(-1f64);
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let cellsize = [
            res_diag, res_x, res_diag, res_y, res_diag, res_x, res_diag, res_y,
        ];
        let mut fid = 1f64;
        let mut z: f32;
        let mut zn: f32;
        let (mut col_n, mut row_n): (isize, isize);
        let mut num_cells_popped = 0f64;
        let num_cells = (rows * columns - 1) as f64; // actually less one.
        let mut size: usize;

        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata {
                    if output.get_value(row, col) == -1f64 {
                        let mut stack = vec![];
                        stack.push(GridCell::new(row, col));
                        output.set_value(row, col, fid);
                        size = 1;

                        // Perform a region-growing operation.
                        while let Some(cell) = stack.pop() {
                            z = input.get_value(cell.row, cell.column);
                            for n in 0..8 {
                                row_n = cell.row + dy[n];
                                col_n = cell.column + dx[n];
                                zn = input.get_value(row_n, col_n);
                                if zn != nodata && output.get_value(row_n, col_n) == -1f64 {
                                    if (z - zn).abs() / cellsize[n] < max_slope {
                                        stack.push(GridCell::new(row_n, col_n));
                                        output.set_value(row_n, col_n, fid);
                                        size += 1;
                                    }
                                }
                            }

                            num_cells_popped += 1f64;
                            if verbose {
                                progress = (100.0_f64 * num_cells_popped / num_cells) as usize;
                                if progress != old_progress {
                                    println!("Region growing: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }

                        if size < min_size {
                            stack.push(GridCell::new(row, col));
                            output.set_value(row, col, 1f64);

                            // Perform a region-growing operation.
                            while let Some(cell) = stack.pop() {
                                for n in 0..8 {
                                    row_n = cell.row + dy[n];
                                    col_n = cell.column + dx[n];
                                    if input.get_value(row_n, col_n) as f64 == fid {
                                        stack.push(GridCell::new(row_n, col_n));
                                        output.set_value(row_n, col_n, 1f64);
                                    }
                                }
                            }
                        } else {
                            fid += 1f64;
                        }
                    }
                } else {
                    output.set_value(row, col, -32768.0);
                    num_cells_popped += 1f64;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Max. absolute slope: {}", max_slope));
        output.add_metadata_entry(format!("Min. feature size: {}", min_size));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        }
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

#[derive(Clone, Copy, Debug)]
struct GridCell {
    row: isize,
    column: isize,
}

impl GridCell {
    fn new(row: isize, column: isize) -> GridCell {
        GridCell { row, column }
    }
}
