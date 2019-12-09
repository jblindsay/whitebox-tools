/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 07/07/2017
Last Modified: 12/10/2018
License: MIT

NOTES: The tool should have the option to output a distance raster as well.
*/

use crate::raster::*;
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
            description: "Wind azimuth in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Search Distance".to_owned(),
            flags: vec!["--max_dist".to_owned()],
            description: "Optional maximum search distance (unspecified if none; in xy units)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
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
        let mut azimuth = 0.0;
        let mut max_dist = f64::INFINITY;

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
            if vec[0].to_lowercase() == "-i"
                || vec[0].to_lowercase() == "--input"
                || vec[0].to_lowercase() == "--dem"
            {
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
            } else if vec[0].to_lowercase() == "-azimuth" || vec[0].to_lowercase() == "--azimuth" {
                if keyval {
                    azimuth = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    azimuth = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-max_dist" || vec[0].to_lowercase() == "--max_dist"
            {
                if keyval {
                    max_dist = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_dist = args[i + 1].to_string().parse::<f64>().unwrap();
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
        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        if azimuth > 360f64 || azimuth < 0f64 {
            azimuth = 0.1;
        }
        if azimuth == 0f64 {
            azimuth = 0.1;
        }
        if azimuth == 180f64 {
            azimuth = 179.9;
        }
        if azimuth == 360f64 {
            azimuth = 359.9;
        }
        let line_slope: f64;
        if azimuth < 180f64 {
            line_slope = (90f64 - azimuth).to_radians().tan();
        } else {
            line_slope = (270f64 - azimuth).to_radians().tan();
        }

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        if input.is_in_geographic_coordinates() {
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                cell_size = cell_size * (113200.0 * mid_lat.cos());
            }
        }

        let x_step: isize;
        let y_step: isize;
        if azimuth > 0f64 && azimuth <= 90f64 {
            x_step = 1;
            y_step = 1;
        } else if azimuth <= 180f64 {
            x_step = 1;
            y_step = -1;
        } else if azimuth <= 270f64 {
            x_step = -1;
            y_step = -1;
        } else {
            x_step = -1;
            y_step = 1;
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut current_val: f64;
                let mut y_intercept: f64;
                let mut current_max_val: f64;
                let a_small_value = -9999999f64;
                let mut flag: bool;
                // let mut max_val_dist: f64;
                let (mut delta_x, mut delta_y): (f64, f64);
                let (mut x, mut y): (f64, f64);
                let (mut x1, mut y1): (isize, isize);
                let (mut x2, mut y2): (isize, isize);
                let (mut z1, mut z2): (f64, f64);
                let mut dist: f64;
                let mut slope: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        current_val = input[(row, col)];
                        if current_val != nodata {
                            //calculate the y intercept of the line equation
                            y_intercept = -row as f64 - line_slope * col as f64;

                            //find all of the vertical intersections
                            current_max_val = a_small_value;
                            // max_val_dist = a_small_value;
                            x = col as f64;

                            flag = true;
                            while flag {
                                x = x + x_step as f64;
                                if x < 0.0 || x >= columns as f64 {
                                    flag = false;
                                // break;
                                } else {
                                    //calculate the Y value
                                    y = (line_slope * x + y_intercept) * -1f64;
                                    if y < 0f64 || y >= rows as f64 {
                                        flag = false;
                                    // break;
                                    } else {
                                        //calculate the distance
                                        delta_x = (x - col as f64) * cell_size;
                                        delta_y = (y - row as f64) * cell_size;

                                        dist = (delta_x * delta_x + delta_y * delta_y).sqrt();
                                        if dist > max_dist {
                                            flag = false;
                                        // break;
                                        } else {
                                            //estimate z
                                            y1 = y as isize;
                                            y2 = y1 + y_step * -1isize;
                                            z1 = input[(y1, x as isize)];
                                            z2 = input[(y2, x as isize)];
                                            z = z1 + (y - y1 as f64) * (z2 - z1);
                                            //calculate the slope
                                            slope = (z - current_val) / dist;
                                            if slope > current_max_val {
                                                current_max_val = slope;
                                                // max_val_dist = dist;
                                                // } else if current_max_val < 0f64 {
                                                // max_val_dist = dist;
                                            }
                                        }
                                    }
                                }
                            }

                            //find all of the horizontal intersections
                            y = -row as f64;
                            flag = true;
                            while flag {
                                y = y + y_step as f64;
                                if -y < 0f64 || -y >= rows as f64 {
                                    flag = false;
                                // break;
                                } else {
                                    //calculate the X value
                                    x = (y - y_intercept) / line_slope;
                                    if x < 0f64 || x >= columns as f64 {
                                        flag = false;
                                    //break;
                                    } else {
                                        //calculate the distance
                                        delta_x = (x - col as f64) * cell_size;
                                        delta_y = (-y - row as f64) * cell_size;
                                        dist = (delta_x * delta_x + delta_y * delta_y).sqrt();
                                        if dist > max_dist {
                                            flag = false;
                                        // break;
                                        } else {
                                            //estimate z
                                            x1 = x as isize;
                                            x2 = x1 + x_step;
                                            if x2 < 0 || x2 >= columns {
                                                flag = false;
                                            // break;
                                            } else {
                                                z1 = input[(-y as isize, x1)];
                                                z2 = input[(y as isize, x2)];
                                                z = z1 + (x - x1 as f64) * (z2 - z1);
                                                //calculate the slope
                                                slope = (z - current_val) / dist;
                                                if slope > current_max_val {
                                                    current_max_val = slope;
                                                // max_val_dist = dist;
                                                } else if current_max_val < 0f64 {
                                                    // max_val_dist = dist;
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            z = current_max_val.atan().to_degrees();
                            if z < -89f64 {
                                z = 0f64;
                            }
                            if current_max_val != a_small_value {
                                data[col as usize] = z;
                            // if (saveDistance) {
                            //     if (z < 0) { max_val_dist = max_val_dist * -1; }
                            //     outputDist.setValue(row, col, max_val_dist);
                            // }
                            } else {
                                data[col as usize] = nodata;
                                // if (saveDistance) {
                                //     outputDist.setValue(row, col, noData);
                                // }
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
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
