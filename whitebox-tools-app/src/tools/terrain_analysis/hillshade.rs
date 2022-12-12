/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 03/09/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use whitebox_common::utils::{
    haversine_distance,
    vincenty_distance
};

/// This tool performs a hillshade operation (also called shaded relief) on an input digital elevation model (DEM).
/// The user must specify the  name of the input DEM and the output hillshade image name. Other parameters that must
/// be specified include the illumination source azimuth (`--azimuth`), or sun direction (0-360 degrees), the
/// illumination source altitude (`--altitude`; i.e. the elevation of the sun above the horizon, measured as an angle
/// from 0 to 90 degrees) and the Z conversion factor (`--zfactor`). The *Z conversion factor* is only important
/// when the vertical and horizontal units are not the same in the DEM. When this is the case,
/// the algorithm will multiply each elevation in the DEM by the Z conversion factor. If the
/// DEM is in the geographic coordinate system (latitude and longitude), the following equation
/// is used:
///
/// > zfactor = 1.0 / (111320.0 x cos(mid_lat))
///
/// where `mid_lat` is the latitude of the centre of the raster, in radians.
///
/// The hillshade value (*HS*) of a DEM grid cell is calculate as:
///
/// > *HS* = tan(*s*) / [1 - tan(*s*)<sup>2</sup>]<sup>0.5</sup> x [sin(*Alt*) / tan(*s*) - cos(*Alt*) x sin(*Az* - *a*)]
///
/// where *s* and *a* are the local slope gradient and aspect (orientation) respectively and *Alt* and *Az*
/// are the illumination source altitude and azimuth respectively. Slope and aspect are calculated using
/// Horn's (1981) 3rd-order finate difference method.
///
/// # Reference
/// Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles
/// and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J.
///
/// # See Also
/// `HypsometricallyTintedHillshade`, `MultidirectionalHillshade`, `Aspect`, `Slope`
pub struct Hillshade {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Hillshade {
    pub fn new() -> Hillshade {
        // public constructor
        let name = "Hillshade".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates a hillshade raster from an input DEM.".to_string();

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
            name: "Azimuth (degrees)".to_owned(),
            flags: vec!["--azimuth".to_owned()],
            description: "Illumination source azimuth in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("315.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Altitude (degrees)".to_owned(),
            flags: vec!["--altitude".to_owned()],
            description: "Illumination source altitude in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("30.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Z Conversion Factor".to_owned(),
            flags: vec!["--zfactor".to_owned()],
            description:
                "Optional multiplier for when the vertical and horizontal units are not the same."
                    .to_owned(),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif --azimuth=315.0 --altitude=30.0", short_exe, name).replace("*", &sep);

        Hillshade {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Hillshade {
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
        let mut azimuth = 315.0f64;
        let mut altitude = 30.0f64;
        let mut z_factor = 1f64;

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
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    azimuth = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-altitude" {
                if keyval {
                    altitude = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    altitude = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-zfactor" {
                if keyval {
                    z_factor = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    z_factor = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let resx = input.configs.resolution_x;
        let resy = input.configs.resolution_y;
        let res = (resx + resy) / 2.;

        azimuth = (azimuth - 90f64).to_radians();
        altitude = altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        // let eight_grid_res = input.configs.resolution_x * 8.0;

        let mut configs = input.configs.clone();
        configs.data_type = DataType::I16;
        configs.nodata = -32768f64;
        let mut output = Raster::initialize_using_config(&output_file, &configs);
        
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        
        let (tx, rx) = mpsc::channel();
        if !input.is_in_geographic_coordinates() {
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z12: f64;
                    let mut p: f64;
                    let mut q: f64;
                    let offsets = [
                        [-2, -2], [-1, -2], [0, -2], [1, -2], [2, -2], 
                        [-2, -1], [-1, -1], [0, -1], [1, -1], [2, -1], 
                        [-2, 0], [-1, 0], [0, 0], [1, 0], [2, 0], 
                        [-2, 1], [-1, 1], [0, 1], [1, 1], [2, 1], 
                        [-2, 2], [-1, 2], [0, 2], [1, 2], [2, 2]
                    ];
                    let mut z = [0f64; 25];
                    let mut val: f64;
                    let (mut term1, mut term2, mut term3): (f64, f64, f64);
                    let mut tan_slope: f64;
                    let mut aspect: f64;
                    let half_pi = PI / 2f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z12 = input.get_value(row, col);
                            if z12 != nodata {
                                for n in 0..25 {
                                    z[n] = input.get_value(row + offsets[n][1], col + offsets[n][0]);
                                    if z[n] != nodata {
                                        z[n] *= z_factor;
                                    } else {
                                        z[n] = z12 * z_factor;
                                    }
                                }

                                /* 
                                The following equations have been taken from Florinsky (2016) Principles and Methods
                                of Digital Terrain Modelling, Chapter 4, pg. 117. 
                                */
                                p = 1. / (420. * res) * (44. * (z[3] + z[23] - z[1] - z[21]) + 31. * (z[0] + z[20] - z[4] - z[24]
                                + 2. * (z[8] + z[18] - z[6] - z[16])) + 17. * (z[14] - z[10] + 4. * (z[13] - z[11]))
                                + 5. * (z[9] + z[19] - z[5] - z[15]));

                                q = 1. / (420. * res) * (44. * (z[5] + z[9] - z[15] - z[19]) + 31. * (z[20] + z[24] - z[0] - z[4]
                                    + 2. * (z[6] + z[8] - z[16] - z[18])) + 17. * (z[2] - z[22] + 4. * (z[7] - z[17]))
                                    + 5. * (z[1] + z[3] - z[21] - z[23]));

                                tan_slope = (p * p + q * q).sqrt();
                                if tan_slope < 0.00017 {
                                    tan_slope = 0.00017;
                                }
                                aspect = if p != 0f64 {
                                    PI - ((q / p).atan()) + half_pi * (p / (p).abs())
                                } else {
                                    PI
                                };
                                // let sign_p = if p != 0.0 { p.signum() } else { 0.0 };
                                // let sign_q = if q != 0.0 { q.signum() } else { 0.0 };
                                // aspect = -half_pi*(1.0 - sign_q)*(1.0-sign_p.abs())+PI*(1.0 + sign_p) - sign_p * (-q / (p*p + q*q).sqrt()).acos();
                                term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                                term2 = sin_theta / tan_slope;
                                term3 = cos_theta * (azimuth - aspect).sin();
                                val = term1 * (term2 - term3);
                                val = val * 32767.0;
                                if val < 0.0 {
                                    val = 0.0;
                                }
                                data[col as usize] = val.round();
                            }
                        }

                        tx.send((row, data)).expect("Error sending data to thread.");
                    }
                });
            }
        } else { // geographic coordinates

            let phi1 = input.get_y_from_row(0);
            let lambda1 = input.get_x_from_column(0);

            let phi2 = phi1;
            let lambda2 = input.get_x_from_column(-1);

            let linear_res = vincenty_distance((phi1, lambda1), (phi2, lambda2));
            let lr2 =  haversine_distance((phi1, lambda1), (phi2, lambda2)); 
            let diff = 100. * (linear_res - lr2).abs() / linear_res;
            let use_haversine = diff < 0.5; // if the difference is less than 0.5%, use the faster haversine method to calculate distances.

            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z4: f64;
                    let mut p: f64;
                    let mut q: f64;
                    let mut a: f64;
                    let mut b: f64;
                    let mut c: f64;
                    let mut d: f64;
                    let mut e: f64;
                    let mut phi1: f64;
                    let mut lambda1: f64;
                    let mut phi2: f64;
                    let mut lambda2: f64;
                    let offsets = [
                        [-1, -1], [0, -1], [1, -1], 
                        [-1, 0], [0, 0], [1, 0], 
                        [-1, 1], [0, 1], [1, 1]
                    ];
                    let mut z = [0f64; 25];
                    let mut val: f64;
                    let (mut term1, mut term2, mut term3): (f64, f64, f64);
                    let mut tan_slope: f64;
                    let mut aspect: f64;
                    let half_pi = PI / 2f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z4 = input.get_value(row, col);
                            if z4 != nodata {
                                for n in 0..9 {
                                    z[n] = input.get_value(row + offsets[n][1], col + offsets[n][0]);
                                    if z[n] != nodata {
                                        z[n] *= z_factor;
                                    } else {
                                        z[n] = z4 * z_factor;
                                    }
                                }

                                // Calculate a, b, c, d, and e.
                                phi1 = input.get_y_from_row(row);
                                lambda1 = input.get_x_from_column(col);

                                phi2 = phi1;
                                lambda2 = input.get_x_from_column(col-1);

                                b = if use_haversine {
                                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                                } else {
                                    vincenty_distance((phi1, lambda1), (phi2, lambda2))
                                };

                                phi2 = input.get_y_from_row(row+1);
                                lambda2 = lambda1;

                                d = if use_haversine {
                                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                                } else {
                                    vincenty_distance((phi1, lambda1), (phi2, lambda2))
                                };

                                phi2 = input.get_y_from_row(row-1);
                                lambda2 = lambda1;

                                e = if use_haversine {
                                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                                } else {
                                    vincenty_distance((phi1, lambda1), (phi2, lambda2))
                                };

                                phi1 = input.get_y_from_row(row+1);
                                lambda1 = input.get_x_from_column(col);

                                phi2 = phi1;
                                lambda2 = input.get_x_from_column(col-1);

                                a = if use_haversine {
                                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                                } else {
                                    vincenty_distance((phi1, lambda1), (phi2, lambda2))
                                };

                                phi1 = input.get_y_from_row(row-1);
                                lambda1 = input.get_x_from_column(col);

                                phi2 = phi1;
                                lambda2 = input.get_x_from_column(col-1);

                                c = if use_haversine {
                                    haversine_distance((phi1, lambda1), (phi2, lambda2))
                                } else {
                                    vincenty_distance((phi1, lambda1), (phi2, lambda2))
                                };

                                /* 
                                The following equations have been taken from Florinsky (2016) Principles and Methods
                                of Digital Terrain Modelling, Chapter 4, pg. 117.
                                */

                                p = (a * a * c * d * (d + e) * (z[2] - z[0]) + b * (a * a * d * d + c * c * e * e) * (z[5] - z[3]) + a * c * c * e * (d + e) * (z[8] - z[6]))
                                / (2. * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

                                q = 1. / (3. * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4))) 
                                * ((d * d * (a.powi(4) + b.powi(4) + b * b * c * c) + c * c * e * e * (a * a - b * b)) * (z[0] + z[2])
                                - (d * d * (a.powi(4) + c.powi(4) + b * b * c * c) - e * e * (a.powi(4) + c.powi(4) + a * a * b * b)) * (z[3] + z[5])
                                - (e * e * (b.powi(4) + c.powi(4) + a * a * b * b) - a * a * d * d * (b * b - c * c)) * (z[6] + z[8])
                                + d * d * (b.powi(4) * (z[1] - 3. * z[4]) + c.powi(4) * (3. * z[1] - z[4]) + (a.powi(4) - 2. * b * b * c * c) * (z[1] - z[4]))
                                + e * e * (a.powi(4) * (z[4] - 3. * z[7]) + b.powi(4) * (3. * z[4] - z[7]) + (c.powi(4) - 2. * a * a * b * b) * (z[4] - z[7]))
                                - 2. * (a * a * d * d * (b * b - c * c) * z[7] + c * c * e * e * (a * a - b * b) * z[1]));
                                
                                tan_slope = (p * p + q * q).sqrt();
                                if tan_slope < 0.00017 {
                                    tan_slope = 0.00017;
                                }
                                aspect = if p != 0f64 {
                                    PI - ((q / p).atan()) + half_pi * (p / (p).abs())
                                } else {
                                    PI
                                };
                                term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                                term2 = sin_theta / tan_slope;
                                term3 = cos_theta * (azimuth - aspect).sin();
                                val = term1 * (term2 - term3);
                                val = val * 32767.0;
                                if val < 0.0 {
                                    val = 0.0;
                                }
                                data[col as usize] = val.round();
                            }
                        }

                        tx.send((row, data)).expect("Error sending data to thread.");
                    }
                });
            }
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
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
        output.add_metadata_entry(format!("Altitude: {}", altitude));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
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
