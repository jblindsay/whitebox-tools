/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/07/2020
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

/// This tool performs a hillshade operation (also called shaded relief) on an input digital elevation model (DEM)
/// with multiple sources of illumination. The user must specify the  name of the input DEM (`--dem`) and the output
/// hillshade image name (`--output`). Other parameters that must be specified include the altitude of the illumination
/// sources (`--altitude`; i.e. the elevation of the sun above the horizon, measured as an angle
/// from 0 to 90 degrees) and the Z conversion factor (`--zfactor`). The *Z conversion factor* is only important
/// when the vertical and horizontal units are not the same in the DEM. When this is the case,
/// the algorithm will multiply each elevation in the DEM by the Z conversion factor. If the
/// DEM is in the geographic coordinate system (latitude and longitude), the following equation
/// is used:
///
/// > zfactor = 1.0 / (111320.0 x cos(mid_lat))
///
/// where `mid_lat` is the latitude of the centre of the raster, in radians. The Z conversion factor can also be used
/// used to apply a vertical exageration to further emphasize landforms within the hillshade output.
///
/// The hillshade value (*HS*) of a DEM grid cell is calculate as:
///
/// > *HS* = tan(*s*) / [1 - tan(*s*)<sup>2</sup>]<sup>0.5</sup> x [sin(*Alt*) / tan(*s*) - cos(*Alt*) x sin(*Az* - *a*)]
///
/// where *s* and *a* are the local slope gradient and aspect (orientation) respectively and *Alt* and *Az*
/// are the illumination source altitude and azimuth respectively. Slope and aspect are calculated using
/// Horn's (1981) 3rd-order finate difference method.
///
/// Lastly, the user must specify whether or not to use full 360-degrees of illumination sources (`--full_mode`). When this
/// flag is not specified, the tool will perform a weighted summation of the hillshade images from four illumination azimuth
/// positions at 225, 270, 315, and 360 (0) degrees, given weights of 0.1, 0.4, 0.4, and 0.1 respectively. When run in the
/// full 360-degree mode, eight illumination source azimuths are used to calculate the output at 0, 45, 90, 135, 180, 225,
/// 270, and 315 degrees, with weights of 0.15, 0.125, 0.1, 0.05, 0.1, 0.125, 0.15, and 0.2 respectively.
///
/// Classic hillshade (Azimuth=315, Altitude=45.0)
/// ![](../../doc_img/MultidirectionalHillshade_fig1.png)
///
/// Multi-directional hillshade (Altitude=45.0, Four-direction mode)
/// ![](../../doc_img/MultidirectionalHillshade_fig2.png)
///
/// Multi-directional hillshade (Altitude=45.0, 360-degree mode)
/// ![](../../doc_img/MultidirectionalHillshade_fig3.png)
///
/// # See Also
/// `Hillshade`, `HypsometricallyTintedHillshade`, `Aspect`, `Slope`
pub struct MultidirectionalHillshade {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MultidirectionalHillshade {
    pub fn new() -> MultidirectionalHillshade {
        // public constructor
        let name = "MultidirectionalHillshade".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates a multi-direction hillshade raster from an input DEM.".to_string();

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
            name: "Altitude (degrees)".to_owned(),
            flags: vec!["--altitude".to_owned()],
            description: "Illumination source altitude in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("45.0".to_owned()),
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

        parameters.push(ToolParameter {
            name: "Full 360-degree mode?".to_owned(),
            flags: vec!["--full_mode".to_owned()],
            description:
                "Optional flag indicating whether to use full 360-degrees of illumination sources."
                    .to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif --altitude=30.0",
            short_exe, name
        )
        .replace("*", &sep);

        MultidirectionalHillshade {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MultidirectionalHillshade {
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
        let mut altitude = 30.0f64;
        let mut z_factor = -1f64;
        let mut multidirection360mode = false;

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
            } else if flag_val == "-full_mode" || flag_val == "-fullmode" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    multidirection360mode = true;
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

        altitude = altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        let eight_grid_res = input.configs.resolution_x * 8.0;

        if input.is_in_geographic_coordinates() && z_factor < 0.0 {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (111320.0 * mid_lat.cos());
            }
        } else if z_factor < 0.0 {
            z_factor = 1.0;
        }

        let mut configs = input.configs.clone();
        configs.data_type = DataType::I16;
        configs.nodata = -32768f64;
        let mut output = Raster::initialize_using_config(&output_file, &configs);
        let out_nodata = output.configs.nodata;
        let rows = input.configs.rows as isize;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let azimuths = if multidirection360mode {
                    vec![
                        (0f64 - 90f64).to_radians(),
                        (45f64 - 90f64).to_radians(),
                        (90f64 - 90f64).to_radians(),
                        (135f64 - 90f64).to_radians(),
                        (180f64 - 90f64).to_radians(),
                        (225f64 - 90f64).to_radians(),
                        (270f64 - 90f64).to_radians(),
                        (315f64 - 90f64).to_radians(),
                    ]
                } else {
                    vec![
                        (225f64 - 90f64).to_radians(),
                        (270f64 - 90f64).to_radians(),
                        (315f64 - 90f64).to_radians(),
                        (360f64 - 90f64).to_radians(),
                    ]
                };

                let weights = if multidirection360mode {
                    vec![
                        0.15f64, 0.125f64, 0.1f64, 0.05f64, 0.1f64, 0.125f64, 0.15f64, 0.20f64,
                    ]
                } else {
                    vec![0.1f64, 0.4f64, 0.4f64, 0.1f64]
                };
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let mut azimuth: f64;
                let (mut term1, mut term2, mut term3): (f64, f64, f64);
                let (mut fx, mut fy): (f64, f64);
                let mut tan_slope: f64;
                let mut aspect: f64;
                let half_pi = PI / 2f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![out_nodata; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            z = z * z_factor;
                            for c in 0..8 {
                                n[c] = input.get_value(row + d_y[c], col + d_x[c]);
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z;
                                }
                            }
                            // calculate slope and aspect
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            tan_slope = (fx * fx + fy * fy).sqrt();
                            if tan_slope < 0.00017 {
                                tan_slope = 0.00017;
                            }
                            aspect = if fx != 0f64 {
                                PI - ((fy / fx).atan()) + half_pi * (fx / (fx).abs())
                            } else {
                                PI
                            };
                            term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                            term2 = sin_theta / tan_slope;

                            z = 0f64;
                            for a in 0..azimuths.len() {
                                azimuth = azimuths[a];
                                term3 = cos_theta * (azimuth - aspect).sin();
                                z += term1 * (term2 - term3) * weights[a];
                            }
                            z = z * 32767.0;
                            if z < 0.0 {
                                z = 0.0;
                            }
                            data[col as usize] = z.round();
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut histo: [f64; 32768] = [0.0; 32768];
        let mut num_cells = 0.0;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            let mut bin: usize;
            for col in 0..data.1.len() {
                if data.1[col] != out_nodata {
                    bin = data.1[col] as usize;
                    histo[bin] += 1.0;
                    num_cells += 1.0;
                }
            }
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut new_min = 0;
        let mut new_max = 0;
        let clip_percent = 0.01;
        let target_cell_num = num_cells * clip_percent;
        let mut sum = 0.0;
        for c in 0..32768 {
            sum += histo[c];
            if sum >= target_cell_num {
                new_min = c;
                break;
            }
        }

        sum = 0.0;
        for c in (0..32768).rev() {
            sum += histo[c];
            if sum >= target_cell_num {
                new_max = c;
                break;
            }
        }

        if new_max > new_min {
            output.configs.display_min = new_min as f64;
            output.configs.display_max = new_max as f64;
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
