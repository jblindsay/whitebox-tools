/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 22 2017
Last Modified: 25/11/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool assigns grid cells in the output image the value of the nearest target cell in
/// the input image, measured by the Euclidean distance (i.e. straight-line distance). Thus,
/// `EuclideanAllocation` essentially creates the Voronoi diagram for a set of target cells.
/// Target cells are all non-zero, non-NoData grid cells in the input image. Distances are
/// calculated using the same efficient algorithm (Shih and Wu, 2003) as the `EuclideanDistance`
/// tool.
///
/// # References
/// Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3
/// neighborhood, *Computer Vision and Image Understanding*, 93: 195-205.
///
/// # See Also
/// `EuclideanDistance`, `VoronoiDiagram`, `CostAllocation`
pub struct EuclideanAllocation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EuclideanAllocation {
    pub fn new() -> EuclideanAllocation {
        // public constructor
        let name = "EuclideanAllocation".to_string();
        let toolbox = "GIS Analysis/Distance Tools".to_string();
        let description = "Assigns grid cells in the output raster the value of the nearest target cell in the input image, measured by the Shih and Wu (2004) Euclidean distance transform.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        EuclideanAllocation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EuclideanAllocation {
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let mut r_x: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut r_y: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut distance: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        let start = Instant::now();

        let mut allocation = Raster::initialize_using_file(&output_file, &input);

        let mut h: f64;
        let mut which_cell: usize;
        let inf_val = f64::INFINITY;
        let d_x = [-1, -1, 0, 1, 1, 1, 0, -1];
        let d_y = [0, -1, -1, -1, 0, 1, 1, 1];
        let g_x = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let g_y = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let (mut x, mut y): (isize, isize);
        let (mut z, mut z2, mut z_min): (f64, f64, f64);

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != 0.0 {
                    distance[(row, col)] = 0.0;
                    allocation[(row, col)] = input[(row, col)];
                } else {
                    distance[(row, col)] = inf_val;
                    allocation[(row, col)] = inf_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing Rasters: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = distance[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 0..4 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = distance[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                0 => 2.0 * r_x[(y, x)] + 1.0,
                                1 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                2 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 3
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        distance[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                        allocation[(row, col)] = allocation[(y, x)];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in (0..rows).rev() {
            for col in (0..columns).rev() {
                z = distance[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 4..8 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = distance[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                5 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                4 => 2.0 * r_x[(y, x)] + 1.0,
                                6 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 7
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        distance[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                        allocation[(row, col)] = allocation[(y, x)];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z == nodata {
                    allocation[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        allocation.configs.palette = input.configs.palette;
        allocation.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        allocation.add_metadata_entry(format!("Input file: {}", input_file));
        allocation.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match allocation.write() {
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
