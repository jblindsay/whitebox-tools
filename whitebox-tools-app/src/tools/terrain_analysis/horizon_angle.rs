/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 07/07/2017
Last Modified: 03/09/2020
License: MIT

NOTES: The tool should have the option to output a distance raster as well.
*/

use whitebox_raster::Raster;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool calculates the horizon angle (*Sx*), i.e. the maximum slope along a specified azimuth (0-360 degrees) for
/// each grid cell in an input digital elevation model (DEM). Horizon angle is sometime referred to as the maximum upwind
/// slope in wind exposure/sheltering studies. Positive values can be considered sheltered with respect to the azimuth and
/// negative values are exposed. Thus, *Sx* is a measure of exposure to a wind from a specific direction. The algorithm works
/// by tracing a ray from each grid cell in the direction of interest and evaluating the slope for each location in which the
/// DEM grid is intersected by the ray. Linear interpolation is used to estimate the elevation of the surface where a ray does
/// not intersect the DEM grid precisely at one of its nodes.
///
/// The user is able to constrain the maximum search distance (`--max_dist`) for the ray tracing by entering a valid maximum
/// search distance value (in the same units as the X-Y coordinates of the input raster DEM). If the maximum search distance
/// is left blank, each ray will be traced to the edge of the DEM, which will add to the computational time.
///
/// Maximum upwind slope should not be calculated for very extensive areas over which the Earth's curvature must be taken into
/// account. Also, this index does not take into account the deflection of wind by topography. However, averaging the horizon
/// angle over a window of directions can yield a more robust measure of exposure, compensating for the deflection of wind from
/// its regional average by the topography. For example, if you are interested in measuring the exposure of a landscape to a
/// northerly wind, you could perform the following calculation:
///
/// > Sx(N) = [Sx(345)+Sx(350)+Sx(355)+Sx(0)+Sx(5)+Sx(10)+Sx(15)] / 7.0
///
/// Ray-tracing is a highly computationally intensive task and therefore this tool may take considerable time to operate for
/// larger sized DEMs. Maximum upwind slope is best displayed using a Grey scale palette that is inverted.
///
/// Horizon angle is best visualized using a white-to-black palette and rescaled from approximately -10 to 70 (see below for
/// an example of horizon angle calculated at a 150-degree azimuth).
///
/// ![](../../doc_img/HorizonAngle.png)
///
/// # See Also
/// `TimeInDaylight`
pub struct HorizonAngle {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HorizonAngle {
    /// public constructor
    pub fn new() -> HorizonAngle {
        let name = "HorizonAngle".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates horizon angle (maximum upwind slope) for each grid cell in an input DEM."
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
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Azimuth".to_owned(),
            flags: vec!["--azimuth".to_owned()],
            description: "Azimuth, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Search Distance".to_owned(),
            flags: vec!["--max_dist".to_owned()],
            description: "Optional maximum search distance (unspecified if none; in xy units). Minimum value is 5 x cell size."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("100.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --azimuth=315.0", short_exe, name).replace("*", &sep);

        HorizonAngle {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HorizonAngle {
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
        let mut azimuth = 0.0f32;
        let mut max_dist = f32::INFINITY;

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
            } else if flag_val == "-azimuth" {
                if keyval {
                    azimuth = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    azimuth = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
                azimuth = azimuth % 360f32;
            } else if flag_val == "-max_dist" {
                max_dist = if keyval {
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
        let inputf64 = Arc::new(Raster::new(&input_file, "r")?);
        let configs = inputf64.configs.clone();
        let mut cell_size_x = configs.resolution_x as f32;
        let mut cell_size_y = configs.resolution_y as f32;
        if inputf64.is_in_geographic_coordinates() {
            let mut mid_lat = ((configs.north - configs.south) / 2.0) as f32;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                cell_size_x = cell_size_x * (111320.0 * mid_lat.cos());
                cell_size_y = cell_size_y * (111320.0 * mid_lat.cos());
            }
        }

        if max_dist <= 5f32 * cell_size_x {
            panic!("The maximum search distance parameter (--max_dist) must be larger than 5 x cell size.");
        }

        // The longest that max_dist ever needs to be is the raster diagonal length.
        let diag_length = ((configs.north - configs.south) * (configs.north - configs.south)
            + (configs.east - configs.west) * (configs.east - configs.west))
            .sqrt() as f32;
        if max_dist > diag_length {
            max_dist = diag_length;
        }

        let input = inputf64.get_data_as_f32_array2d();

        let start = Instant::now();

        let line_slope: f32 = if azimuth < 180f32 {
            ((90f32 - azimuth) as f64).to_radians().tan() as f32
        } else {
            ((270f32 - azimuth) as f64).to_radians().tan() as f32
        };

        let rows = configs.rows as isize;
        let columns = configs.columns as isize;
        let nodata = configs.nodata;
        let nodata_f32 = nodata as f32;

        drop(inputf64);

        // Now perform the filter
        let mut num_procs = num_cpus::get() as isize;
        let configurations = whitebox_common::configs::get_configs()?;
        let max_procs = configurations.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                // The ray-tracing operation can be viewed as a linear maximum filter. The first step
                // is to create the filter offsets and calculate the offset distances.

                let x_step: isize;
                let y_step: isize;
                if azimuth > 0f32 && azimuth <= 90f32 {
                    x_step = 1;
                    y_step = 1;
                } else if azimuth <= 180f32 {
                    x_step = 1;
                    y_step = -1;
                } else if azimuth <= 270f32 {
                    x_step = -1;
                    y_step = -1;
                } else {
                    x_step = -1;
                    y_step = 1;
                }

                let mut flag: bool;
                let (mut delta_x, mut delta_y): (f32, f32);
                let (mut x, mut y): (f32, f32);
                let (mut x1, mut y1): (isize, isize);
                let (mut x2, mut y2): (isize, isize);
                let (mut z1, mut z2): (f32, f32);
                let mut dist: f32;
                let mut weight: f32;
                let mut offsets = vec![];

                // Find all of the horizontal grid intersections.
                if line_slope != 0f32 {
                    // Otherwise, there are no horizontal intersections.
                    y = 0f32;
                    flag = true;
                    while flag {
                        y += y_step as f32;
                        x = y / line_slope;

                        // calculate the distance
                        delta_x = x * cell_size_x;
                        delta_y = -y * cell_size_y;
                        dist = delta_x.hypot(delta_y);
                        if dist <= max_dist {
                            x1 = x.floor() as isize;
                            x2 = x1 + 1;
                            y1 = -y as isize;
                            weight = x - x1 as f32;
                            offsets.push((x1, y1, x2, y1, weight, dist));
                        } else {
                            flag = false;
                        }
                    }
                }

                // Find all of the vertical grid intersections.
                x = 0f32;
                flag = true;
                while flag {
                    x += x_step as f32;
                    y = -(line_slope * x); // * -1f32;

                    // calculate the distance
                    delta_x = x * cell_size_x;
                    delta_y = y * cell_size_y;

                    dist = delta_x.hypot(delta_y);
                    if dist <= max_dist {
                        y1 = y.floor() as isize;
                        y2 = y1 + 1; // - y_step;
                        x1 = x as isize;
                        weight = y - y1 as f32;
                        offsets.push((x1, y1, x1, y2, weight, dist));
                    } else {
                        flag = false;
                    }
                }

                // Sort by distance.
                offsets.sort_by(|a, b| a.5.partial_cmp(&b.5).unwrap());

                let num_offsets = offsets.len();
                let mut z: f32;
                let mut slope: f32;
                let early_stopping_slope = 80f32.to_radians().tan();
                let mut current_elev: f32;
                let mut current_max_slope: f32;
                let mut current_max_elev: f32;
                let a_small_value = -9999999f32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        current_elev = input.get_value(row, col);
                        if current_elev != nodata_f32 {
                            // Run down the offsets of the ray
                            current_max_slope = a_small_value;
                            current_max_elev = a_small_value;
                            for i in 0..num_offsets {
                                // Where are we on the grid?
                                x1 = col + offsets[i].0;
                                y1 = row + offsets[i].1;
                                x2 = col + offsets[i].2;
                                y2 = row + offsets[i].3;

                                // What is the elevation?
                                z1 = input.get_value(y1, x1);
                                z2 = input.get_value(y2, x2);

                                if z1 == nodata_f32 && z2 == nodata_f32 {
                                    break; // We're likely off the grid.
                                } else if z1 == nodata_f32 {
                                    z1 = z2;
                                } else if z2 == nodata_f32 {
                                    z2 = z2;
                                }

                                z = z1 + offsets[i].4 * (z2 - z1);

                                // All previous cells are nearer, and so if this isn't a higher
                                // cell than the current highest, it can't be the horizon cell.
                                if z > current_max_elev {
                                    current_max_elev = z;

                                    // Calculate the slope
                                    slope = (z - current_elev) / offsets[i].5;
                                    if slope > current_max_slope {
                                        current_max_slope = slope;
                                        if slope > early_stopping_slope {
                                            break; // we're unlikely to find a farther horizon cell.
                                        }
                                    }
                                }
                            }

                            if current_max_slope == a_small_value {
                                data[col as usize] = 0f64; // It's a zero-length scan. We didn't encounter any valid cells.
                            } else {
                                data[col as usize] = current_max_slope.atan().to_degrees() as f64;
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_from_array2d(&output_file, &configs, &input);
        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Azimuth: {}", azimuth));
        output.add_metadata_entry(format!("Max dist: {}", max_dist));
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
