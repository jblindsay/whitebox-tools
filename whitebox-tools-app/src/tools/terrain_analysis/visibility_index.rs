/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 07/04/2018
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::Raster;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use whitebox_common::utils::get_formatted_elapsed_time;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool can be used to calculate a measure of landscape visibility based on the
/// topography of an input digital elevation model (DEM). The user must specify the name of
/// the input DEM (`--dem`), the output file name (`--output`), the viewing height (`--height`),
/// and a resolution factor (`--res_factor`).
/// Viewsheds are calculated for a subset of grid cells in the DEM based on the resolution
/// factor. The visibility index value (0.0-1.0) indicates the proportion of tested stations
/// (determined by the resolution factor) that each cell is visible from. The viewing height
/// is in the same units as the elevations of the DEM and represent a height above the ground
/// elevation. Each tested grid cell's viewshed will be calculated in parallel. However, visibility
/// index is one of the most computationally intensive geomorphometric indices to calculate.
/// Depending on the size of the input DEM grid and the resolution factor, this operation may take
/// considerable time to complete. If the task is too long-running, it is advisable to raise the
/// resolution factor. A resolution factor of 2 will skip every second row and every second column
/// (effectively evaluating the viewsheds of a quarter of the DEM's grid cells). Increasing this
/// value decreases the number of calculated viewshed but will result in a lower accuracy estimate
/// of overall visibility. In addition to the high computational costs of this index, the tool
/// also requires substantial memory resources to operate. Each of these limitations should be
/// considered before running this tool on a particular data set. This tool is best to apply
/// on computer systems with high core-counts and plenty of memory.
///
/// ![](../../doc_img/VisiibilityIndex_fig1.png)
///
/// # See Also
/// `Viewshed`
pub struct VisibilityIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VisibilityIndex {
    /// public constructor
    pub fn new() -> VisibilityIndex {
        let name = "VisibilityIndex".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Estimates the relative visibility of sites in a DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
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
            name: "Station Height (in z units)".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "Viewing station height, in z units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Resolution Factor".to_owned(),
            flags: vec!["--res_factor".to_owned()],
            description: "The resolution factor determines the density of measured viewsheds."
                .to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("2".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=dem.tif -o=output.tif --height=10.0 --res_factor=4", short_exe, name).replace("*", &sep);

        VisibilityIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VisibilityIndex {
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
        let mut height = 2f32;
        let mut res_factor = 2isize;

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
            } else if flag_val == "-height" {
                height = if keyval {
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
            } else if flag_val == "-res_factor" {
                res_factor = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as isize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as isize
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

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
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
        let dem = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        if height < 0f32 {
            println!("Warning: Input station height cannot be less than zero.");
            height = 0f32;
        }

        if res_factor < 1 {
            res_factor = 1;
        }
        if res_factor > 25 {
            res_factor = 25;
        }

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;

        if verbose {
            println!("Performing analysis. Please be patient...")
        };

        let num_cells_tested =
            (rows as f64 / res_factor as f64).ceil() * (columns as f64 / res_factor as f64).ceil();
        let target_cells_count = (num_cells_tested / 100f64) as usize;
        let num_cells_completed = Arc::new(Mutex::new(0));
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let dem = dem.clone();
            let num_cells_completed = num_cells_completed.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut return_data: Array2D<usize> =
                    Array2D::new(rows, columns, 0usize, 0usize).unwrap();
                let mut view_angle: Array2D<f32> =
                    Array2D::new(rows, columns, -32768f32, -32768f32).unwrap();
                let mut max_view_angle: Array2D<f32> =
                    Array2D::new(rows, columns, -32768f32, -32768f32).unwrap();

                let mut row_y_values = vec![0f32; rows as usize];
                let mut col_x_values = vec![0f32; columns as usize];
                for row in 0..rows {
                    row_y_values[row as usize] = dem.get_y_from_row(row) as f32;
                }
                for col in 0..columns {
                    col_x_values[col as usize] = dem.get_x_from_column(col) as f32;
                }

                let mut stn_x: f32;
                let mut stn_y: f32;
                let mut stn_z: f32;
                let (mut x, mut y): (f32, f32);
                let mut dz: f32;
                let mut dist: f32;
                let mut z: f32;
                let mut tva: f32;
                let mut va: f32;
                let mut t1: f32;
                let mut t2: f32;
                let mut vert_count: f32;
                let mut horiz_count: f32;
                let mut max_va: f32;
                let mut cells_completed_by_thread = 0usize;
                for stn_row in (0..rows)
                    .filter(|r| (r % res_factor == 0) && ((r / res_factor) % num_procs == tid))
                {
                    for stn_col in (0..columns).filter(|c| c % res_factor == 0) {
                        stn_x = col_x_values[stn_col as usize];
                        stn_y = row_y_values[stn_row as usize];
                        stn_z = dem.get_value(stn_row, stn_col) as f32 + height;

                        // now calculate the view angle
                        for row in 0..rows {
                            for col in 0..columns {
                                x = col_x_values[col as usize];
                                y = row_y_values[row as usize];
                                dz = dem.get_value(row, col) as f32 - stn_z;
                                dist =
                                    ((x - stn_x) * (x - stn_x) + (y - stn_y) * (y - stn_y)).sqrt();
                                if dist != 0.0 {
                                    view_angle.set_value(row, col, dz / dist * 1000f32);
                                } else {
                                    view_angle.set_value(row, col, 0f32);
                                }
                            }
                        }

                        // perform the simple scan lines.
                        for row in stn_row - 1..stn_row + 2 {
                            for col in stn_col - 1..stn_col + 2 {
                                max_view_angle.set_value(row, col, view_angle.get_value(row, col));
                            }
                        }

                        max_va = view_angle.get_value(stn_row - 1, stn_col);
                        for row in (0..stn_row - 1).rev() {
                            z = view_angle.get_value(row, stn_col);
                            if z > max_va {
                                max_va = z;
                                return_data.increment(row, stn_col, 1usize);
                            }
                            max_view_angle.set_value(row, stn_col, max_va);
                        }

                        max_va = view_angle.get_value(stn_row + 1, stn_col);
                        for row in stn_row + 2..rows {
                            z = view_angle.get_value(row, stn_col);
                            if z > max_va {
                                max_va = z;
                                return_data.increment(row, stn_col, 1usize);
                            }
                            max_view_angle.set_value(row, stn_col, max_va);
                        }

                        max_va = view_angle.get_value(stn_row, stn_col + 1);
                        for col in stn_col + 2..columns {
                            z = view_angle.get_value(stn_row, col);
                            if z > max_va {
                                max_va = z;
                                return_data.increment(stn_row, col, 1usize);
                            }
                            max_view_angle.set_value(stn_row, col, max_va);
                        }

                        max_va = view_angle.get_value(stn_row, stn_col - 1);
                        for col in (0..stn_col - 1).rev() {
                            z = view_angle.get_value(stn_row, col);
                            if z > max_va {
                                max_va = z;
                                return_data.increment(stn_row, col, 1usize);
                            }
                            max_view_angle.set_value(stn_row, col, max_va);
                        }

                        //solve the first triangular facet
                        vert_count = 1f32;
                        for row in (0..stn_row - 1).rev() {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for col in stn_col + 1..stn_col + (vert_count as isize) + 1 {
                                if col <= columns {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row + 1, col - 1);
                                        t2 = max_view_angle.get_value(row + 1, col);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row + 1, col - 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        //solve the second triangular facet
                        vert_count = 1f32;
                        for row in (0..stn_row - 1).rev() {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for col in (stn_col - (vert_count as isize)..stn_col).rev() {
                                if col >= 0 {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row + 1, col + 1);
                                        t2 = max_view_angle.get_value(row + 1, col);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row + 1, col + 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the third triangular facet
                        vert_count = 1f32;
                        for row in stn_row + 2..rows {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for col in (stn_col - (vert_count as isize)..stn_col).rev() {
                                if col >= 0 {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row - 1, col + 1);
                                        t2 = max_view_angle.get_value(row - 1, col);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row - 1, col + 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the fourth triangular facet
                        vert_count = 1f32;
                        for row in stn_row + 2..rows {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for col in stn_col + 1..stn_col + (vert_count as isize) + 1 {
                                if col < columns {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row - 1, col - 1);
                                        t2 = max_view_angle.get_value(row - 1, col);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row - 1, col - 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the fifth triangular facet
                        vert_count = 1f32;
                        for col in stn_col + 2..columns {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for row in (stn_row - (vert_count as isize)..stn_row).rev() {
                                if row >= 0 {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row + 1, col - 1);
                                        t2 = max_view_angle.get_value(row, col - 1);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row + 1, col - 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the sixth triangular facet
                        vert_count = 1f32;
                        for col in stn_col + 2..columns {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for row in stn_row + 1..stn_row + (vert_count as isize) + 1 {
                                if row < rows {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row - 1, col - 1);
                                        t2 = max_view_angle.get_value(row, col - 1);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row - 1, col - 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the seventh triangular facet
                        vert_count = 1f32;
                        for col in (0..stn_col - 1).rev() {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for row in stn_row + 1..stn_row + (vert_count as isize) + 1 {
                                if row < rows {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row - 1, col + 1);
                                        t2 = max_view_angle.get_value(row, col + 1);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row - 1, col + 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        // solve the eighth triangular facet
                        vert_count = 1f32;
                        for col in (0..stn_col - 1).rev() {
                            vert_count += 1f32;
                            horiz_count = 0f32;
                            for row in (stn_row - (vert_count as isize)..stn_row).rev() {
                                if row < rows {
                                    va = view_angle.get_value(row, col);
                                    horiz_count += 1f32;
                                    if horiz_count != vert_count {
                                        t1 = max_view_angle.get_value(row + 1, col + 1);
                                        t2 = max_view_angle.get_value(row, col + 1);
                                        tva = t2 + horiz_count / vert_count * (t1 - t2);
                                    } else {
                                        tva = max_view_angle.get_value(row + 1, col + 1);
                                    }
                                    if tva > va {
                                        max_view_angle.set_value(row, col, tva);
                                    } else {
                                        max_view_angle.set_value(row, col, va);
                                        return_data.increment(row, col, 1usize);
                                    }
                                } else {
                                    break;
                                }
                            }
                        }

                        cells_completed_by_thread += 1usize;
                        if cells_completed_by_thread == target_cells_count {
                            let mut num_cells_completed = num_cells_completed.lock().unwrap();
                            *num_cells_completed += cells_completed_by_thread;
                            cells_completed_by_thread = 0;
                            if verbose {
                                let progress = (100.0_f64 * *num_cells_completed as f64
                                    / (num_cells_tested - 1f64))
                                    as usize;
                                println!("Progress (Loop 1 of 2): {}%", progress);
                            }
                        }
                    }
                }
                let mut num_cells_completed = num_cells_completed.lock().unwrap();
                *num_cells_completed += cells_completed_by_thread;
                if verbose {
                    let progress = (100.0_f64 * *num_cells_completed as f64
                        / (num_cells_tested - 1f64)) as usize;
                    println!("Progress (Loop 1 of 2): {}%", progress);
                }
                tx.send(return_data).unwrap();
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &dem);
        let mut z: f64;
        for _p in 0..num_procs {
            let data = rx.recv().expect("Error receiving data from thread.");
            for row in 0..rows {
                for col in 0..columns {
                    if dem.get_value(row, col) != nodata {
                        z = data.get_value(row, col) as f64;
                        output.increment(row, col, z);
                    }
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if dem.get_value(row, col) != nodata {
                    z = output.get_value(row, col);
                    output.set_value(row, col, z / num_cells_tested)
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("DEM file: {}", input_file));
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
