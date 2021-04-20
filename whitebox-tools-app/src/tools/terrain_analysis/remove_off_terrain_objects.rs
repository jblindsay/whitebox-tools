/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/06/2017
Last Modified: 07/08/2020
License: MIT

Note: This algorithm could be parallelized
*/

use whitebox_raster::Raster;
use whitebox_common::structures::{Array2D, DistanceMetric, FixedRadiusSearch2D};
use crate::tools::*;
use num_cpus;
use std::collections::VecDeque;
use std::env;
use std::f32;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to create a bare-earth DEM from a fine-resolution digital surface model. The
/// tool is typically applied to LiDAR DEMs which frequently contain numerous off-terrain objects (OTOs) such
/// as buildings, trees and other vegetation, cars, fences and other anthropogenic objects. The algorithm
/// works by finding and removing steep-sided peaks within the DEM. All peaks within a sub-grid, with a
/// dimension of the user-specified maximum OTO size (`--filter`), in pixels, are identified and removed.
/// Each of the edge cells of the peaks are then examined to see if they have a slope that is less than the
/// user-specified minimum OTO edge slope (`--slope`) and a back-filling procedure is used. This ensures that
/// OTOs are distinguished from natural topographic features such as hills. The DEM is preprocessed using a
/// white top-hat transform, such that elevations are normalized for the underlying ground surface.
///
/// Note that this tool is appropriate to apply to rasterized LiDAR DEMs. Use the `LidarGroundPointFilter`
/// tool to remove or classify OTOs within a LiDAR point-cloud.
///
/// # Reference
/// J.B. Lindsay (2018) A new method for the removal of off-terrain objects from LiDAR-derived raster surface
/// models. Available online, DOI: [10.13140/RG.2.2.21226.62401](https://www.researchgate.net/publication/323003064_A_new_method_for_the_removal_of_off-terrain_objects_from_LiDAR-derived_raster_surface_models)
///
/// # See Also
/// `MapOffTerrainObjects`, `TophatTransform`, `LidarGroundPointFilter`
pub struct RemoveOffTerrainObjects {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RemoveOffTerrainObjects {
    pub fn new() -> RemoveOffTerrainObjects {
        // public constructor
        let name = "RemoveOffTerrainObjects".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Removes off-terrain objects from a raster digital elevation model (DEM).".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned(), "--dem".to_owned()],
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
            name: "Filter Dimension".to_owned(),
            flags: vec!["--filter".to_owned()],
            description: "Filter size (cells).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Slope Threshold".to_owned(),
            flags: vec!["--slope".to_owned()],
            description: "Slope threshold value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("15.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=bare_earth_DEM.tif --filter=25 --slope=10.0", short_exe, name).replace("*", &sep);

        RemoveOffTerrainObjects {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RemoveOffTerrainObjects {
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
        let mut filter_size = 11usize;
        let mut slope_threshold = 15f32;
        let mut keyval: bool;
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
            keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-filter" {
                if keyval {
                    filter_size = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    filter_size = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                }
            } else if flag_val == "-slope" {
                if keyval {
                    slope_threshold = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    slope_threshold = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val));
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
        if (filter_size as f32 / 2f32).floor() == (filter_size as f32 / 2f32) {
            filter_size += 1;
        }

        let (mut z, mut z_n): (f32, f32);
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let midpoint = (filter_size as f32 / 2f32).floor() as isize;
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
        let input_f64 = Raster::new(&input_file, "r")?; // 2X
        let input = Arc::new(input_f64.get_data_as_f32_array2d()); // 3X

        let start = Instant::now();

        let configs = input_f64.configs.clone();
        drop(input_f64); // 1X
        let nodata = configs.nodata as f32;
        let cell_size_x = configs.resolution_x as f32;
        let cell_size_y = configs.resolution_y as f32;
        let cell_size_diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let slope = slope_threshold.to_radians().tan();
        let height_diff_threshold = [
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y,
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y,
        ];
        let columns = configs.columns as isize;
        let rows = configs.rows as isize;

        // Perform the white tophat transform
        if verbose {
            println!("Performing tophat transform...")
        };

        let mut num_procs = num_cpus::get() as isize;
        let configuations = whitebox_common::configs::get_configs()?;
        let max_procs = configuations.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f32;
                let mut z_n: f32;
                let mut min_val: f32;
                // let (mut start_row, mut end_row): (isize, isize);
                // let (mut start_col, mut end_col): (isize, isize);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut erosion = vec![nodata; columns as usize];
                    let mut filter_vals: VecDeque<f32> = VecDeque::with_capacity(filter_size);
                    let start_row = row - midpoint;
                    let end_row = row + midpoint;
                    for col in 0..columns {
                        if col > 0 {
                            filter_vals.pop_front();
                            min_val = f32::INFINITY;
                            for row2 in start_row..end_row + 1 {
                                z_n = input.get_value(row2, col + midpoint);
                                if z_n < min_val && z_n != nodata {
                                    min_val = z_n;
                                }
                            }
                            filter_vals.push_back(min_val);
                        } else {
                            // initialize the filter_vals
                            let start_col = col - midpoint;
                            let end_col = col + midpoint;
                            for col2 in start_col..end_col + 1 {
                                min_val = f32::INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = input.get_value(row2, col2);
                                    if z_n < min_val && z_n != nodata {
                                        min_val = z_n;
                                    }
                                }
                                filter_vals.push_back(min_val);
                            }
                        }
                        z = input.get_value(row, col);
                        if z != nodata {
                            min_val = f32::INFINITY;
                            for v in filter_vals.iter() {
                                if *v < min_val {
                                    min_val = *v;
                                }
                            }
                            erosion[col as usize] = min_val;
                        }
                    }
                    tx.send((row, erosion)).unwrap();
                }
            });
        }

        let mut erosion: Array2D<f32> =
            Array2D::new(rows, columns, 0f32, nodata).expect("Error creating Array2D."); // 2X
        for r in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            erosion.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f32 * r as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Performing erosion: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let erosion = Arc::new(erosion);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let erosion = erosion.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f32;
                let mut z_n: f32;
                let mut max_val: f32;
                // let (mut start_row, mut end_row): (isize, isize);
                // let (mut start_col, mut end_col): (isize, isize);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut tophat = vec![nodata; columns as usize];
                    let mut opening = vec![nodata; columns as usize];
                    let mut filter_vals: VecDeque<f32> = VecDeque::with_capacity(filter_size);
                    let start_row = row - midpoint;
                    let end_row = row + midpoint;
                    for col in 0..columns {
                        if col > 0 {
                            filter_vals.pop_front();
                            max_val = f32::NEG_INFINITY;
                            for row2 in start_row..end_row + 1 {
                                z_n = erosion.get_value(row2, col + midpoint);
                                if z_n > max_val && z_n != nodata {
                                    max_val = z_n;
                                }
                            }
                            filter_vals.push_back(max_val);
                        } else {
                            // initialize the filter_vals
                            let start_col = col - midpoint;
                            let end_col = col + midpoint;
                            for col2 in start_col..end_col + 1 {
                                max_val = f32::NEG_INFINITY;
                                for row2 in start_row..end_row + 1 {
                                    z_n = erosion.get_value(row2, col2);
                                    if z_n > max_val && z_n != nodata {
                                        max_val = z_n;
                                    }
                                }
                                filter_vals.push_back(max_val);
                            }
                        }
                        z = input.get_value(row, col);
                        if z != nodata {
                            max_val = f32::NEG_INFINITY;
                            for v in filter_vals.iter() {
                                if *v > max_val {
                                    max_val = *v;
                                }
                            }
                            if max_val > f32::NEG_INFINITY {
                                tophat[col as usize] = z - max_val;
                                opening[col as usize] = max_val;
                            }
                        }
                    }
                    tx.send((row, tophat, opening)).unwrap();
                }
            });
        }

        let mut opening: Array2D<f32> =
            Array2D::new(rows, columns, 0f32, nodata).expect("Error creating Array2D."); // 3X
        let mut tophat: Array2D<f32> =
            Array2D::new(rows, columns, 0f32, nodata).expect("Error creating Array2D."); // 4X
        for r in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            tophat.set_row_data(data.0, data.1);
            opening.set_row_data(data.0, data.2);

            if verbose {
                progress = (100.0_f32 * r as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Performing dilation: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // drop(erosion); // 3X
        // drop(input); // 2X

        drop(Arc::try_unwrap(erosion).expect("Error unwrapping Arc."));
        drop(Arc::try_unwrap(input).expect("Error unwrapping Arc."));

        // Back-fill the shallow hills using region growing
        if verbose {
            println!("Backfilling hills...")
        };
        let initial_value = f32::NEG_INFINITY;
        let mut out: Array2D<f32> =
            Array2D::new(rows, columns, initial_value, nodata).expect("Error creating Array2D"); // 3X
        let mut stack: Vec<GridCell> = vec![];
        let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
        let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
        for row in 0..rows {
            for col in 0..columns {
                out.set_value(row, col, initial_value);
                if tophat.get_value(row, col) != nodata {
                    if tophat.get_value(row, col) <= height_diff_threshold[1] {
                        stack.push(GridCell {
                            row: row,
                            column: col,
                        });
                        out.set_value(row, col, tophat.get_value(row, col));
                    }
                } else {
                    out.set_value(row, col, nodata);
                }
            }
            if verbose {
                progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Finding seed cells: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        while stack.len() > 0 {
            let gc = stack.pop().expect("Error during pop operation.");
            row = gc.row;
            col = gc.column;
            z = tophat.get_value(row, col);
            for i in 0..8 {
                row_n = row + d_y[i];
                col_n = col + d_x[i];
                z_n = tophat.get_value(row_n, col_n);
                if z_n != nodata && out[(row_n, col_n)] == initial_value {
                    if z_n - z < height_diff_threshold[i] {
                        out[(row_n, col_n)] = z_n;
                        stack.push(GridCell {
                            row: row_n,
                            column: col_n,
                        });
                    }
                }
            }
        }

        drop(stack);

        // Interpolate the data holes. Start by locating all the edge cells.
        if verbose {
            println!("Interpolating data holes...")
        };
        let mut frs: FixedRadiusSearch2D<f32> = FixedRadiusSearch2D::new(
            filter_size as f64 / 1.5f64,
            DistanceMetric::SquaredEuclidean,
        ); // 4X
        for row in 0..rows {
            for col in 0..columns {
                if tophat.get_value(row, col) != nodata && out.get_value(row, col) != initial_value
                {
                    for i in 0..8 {
                        row_n = row + d_y[i];
                        col_n = col + d_x[i];
                        if tophat.get_value(row_n, col_n) != nodata
                            && out.get_value(row_n, col_n) == initial_value
                        {
                            frs.insert(
                                col as f64,
                                row as f64,
                                opening[(row, col)] + tophat[(row, col)],
                            );
                            break;
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Finding OTO edge cells: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut sum_weights: f32;
        let mut dist: f32;
        for row in 0..rows {
            for col in 0..columns {
                if out[(row, col)] == initial_value {
                    sum_weights = 0f32;
                    let ret = frs.search(col as f64, row as f64);
                    for j in 0..ret.len() {
                        dist = ret[j].1 as f32;
                        if dist > 0.0 {
                            sum_weights += 1.0 / dist;
                        }
                    }
                    z = 0.0;
                    for j in 0..ret.len() {
                        dist = ret[j].1 as f32;
                        if dist > 0.0 {
                            z += ret[j].0 * (1.0 / dist) / sum_weights;
                        }
                    }
                    if ret.len() > 0 {
                        out.set_value(row, col, z);
                    } else {
                        out.set_value(row, col, nodata);
                    }
                } else {
                    out.set_value(
                        row,
                        col,
                        opening.get_value(row, col) + tophat.get_value(row, col),
                    );
                }
            }
            if verbose {
                progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Interpolating data holes: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        drop(frs); // 3X
        drop(opening); // 2X

        let elapsed_time = get_formatted_elapsed_time(start);

        // Finally, output the new raster
        let mut output = Raster::initialize_using_config(&output_file, &configs); // 8X
        for row in 0..rows {
            for col in 0..columns {
                if out.get_value(row, col) != initial_value && tophat.get_value(row, col) != nodata
                {
                    output.set_value(row, col, out[(row, col)] as f64);
                } else {
                    output.set_value(row, col, nodata as f64);
                }
            }
            if verbose {
                progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                if progress != old_progress {
                    println!("Outputing data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        output.add_metadata_entry(
            "Created by whitebox_tools\' remove_off_terrain_objects tool".to_owned(),
        );
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Filter size: {}", filter_size));
        output.add_metadata_entry(format!("Slope threshold: {}", slope_threshold));
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

#[derive(Copy, Clone, Eq, PartialEq)]
struct GridCell {
    row: isize,
    column: isize,
}
