/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/03/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to calculate the elevation of each grid cell in a raster above the nearest stream cell, 
/// measured along the straight-line distance. This terrain index, a measure of relative topographic position, is 
/// related to the 'height above drainage' (HAND), as described by Renno et al. (2008). HAND is generally estimated 
/// with distances measured along drainage flow-paths, which can be calculated using the `ElevationAboveStream` tool. 
/// The user must specify the name of an input digital elevation model (`--dem`) and streams raster (`--streams`). 
/// Stream cells are designated in this raster as all non-zero values. The output of this tool, 
/// along with the `DownslopeDistanceToStream` tool, can be useful for preliminary flood plain mapping when combined 
/// with high-accuracy DEM data.
/// 
/// The difference between `ElevationAboveStream` and `ElevationAboveStreamEuclidean` is that the former calculates 
/// distances along drainage flow-paths while the latter calculates straight-line distances to streams channels.
/// 
/// # Reference
/// Renno, C. D., Nobre, A. D., Cuartas, L. A., Soares, J. V., Hodnett, M. G., Tomasella, J., & Waterloo, M. J. 
/// (2008). HAND, a new terrain descriptor using SRTM-DEM: Mapping terra-firme rainforest environments in Amazonia. 
/// Remote Sensing of Environment, 112(9), 3469-3481.
/// 
/// # See Also
/// `ElevationAboveStream`, `DownslopeDistanceToStream`, `ElevAbovePit`
pub struct ElevationAboveStreamEuclidean {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ElevationAboveStreamEuclidean {
    pub fn new() -> ElevationAboveStreamEuclidean {
        // public constructor
        let name = "ElevationAboveStreamEuclidean".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates the elevation of cells above the nearest (Euclidean distance) stream cell."
                .to_string();

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
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif --streams=streams.tif -o=output.tif", short_exe, name).replace("*", &sep);

        ElevationAboveStreamEuclidean {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ElevationAboveStreamEuclidean {
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
        let mut dem_file = String::new();
        let mut streams_file = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
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

        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let dem = Raster::new(&dem_file, "r")?;
        let input = Raster::new(&streams_file, "r")?;

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let mut r_x: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut r_y: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut distance: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        if dem.configs.rows as isize != rows || dem.configs.columns as isize != columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input DEM and streams file must have the same extent (rows and columns).",
            ));
        }

        let start = Instant::now();

        let mut allocation = Raster::initialize_using_file(&output_file, &dem);

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
                    distance.set_value(row, col, 0.0);
                    allocation.set_value(row, col, dem.get_value(row, col));
                } else {
                    distance.set_value(row, col, inf_val);
                    allocation.set_value(row, col, inf_val);
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
                        distance.set_value(row, col, z_min);
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x.set_value(row, col, r_x.get_value(y, x) + g_x[which_cell]);
                        r_y.set_value(row, col, r_y.get_value(y, x) + g_y[which_cell]);
                        allocation.set_value(row, col, allocation.get_value(y, x));
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
                        distance.set_value(row, col, z_min);
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x.set_value(row, col, r_x.get_value(y, x) + g_x[which_cell]);
                        r_y.set_value(row, col, r_y.get_value(y, x) + g_y[which_cell]);
                        allocation.set_value(row, col, allocation.get_value(y, x));
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
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
                    allocation.set_value(row, col, nodata);
                } else {
                    allocation.set_value(row, col, dem.get_value(row, col) - allocation.get_value(row, col));
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
        allocation.configs.palette = dem.configs.palette;
        allocation.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        allocation.add_metadata_entry(format!("Input DEM file: {}", dem_file));
        allocation.add_metadata_entry(format!("Input streams file: {}", streams_file));
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
