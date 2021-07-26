/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 01/03/2021
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool calculates slope aspect (i.e. slope orientation in degrees clockwise from north) for each grid cell
/// in an input digital elevation model (DEM). The user must specify the name of the input
/// DEM (`--dem`) and the output raster image. The *Z conversion factor* is only important
/// when the vertical and horizontal units are not the same in the DEM. When this is the case,
/// the algorithm will multiply each elevation in the DEM by the Z conversion factor. If the
/// DEM is in the geographic coordinate system (latitude and longitude), the following equation
/// is used:
///
/// > zfactor = 1.0 / (111320.0 x cos(mid_lat))
///
/// where `mid_lat` is the latitude of the centre of the raster, in radians.
///
/// The tool uses Horn's (1981) 3rd-order finite difference method to estimate slope. Given
/// the following clock-type grid cell numbering scheme (Gallant and Wilson, 2000),
///
/// |  7  |  8  |  1  | \
/// |  6  |  9  |  2  | \
/// |  5  |  4  |  3  |
///
/// > aspect = 180 - arctan(f<sub>y</sub> / f<sub>x</sub>) + 90(f<sub>x</sub> / |f<sub>x</sub>|)
///
/// where,
///
/// > f<sub>x</sub> = (z<sub>3</sub> - z<sub>5</sub> + 2(z<sub>2</sub> - z<sub>6</sub>) + z<sub>1</sub> - z<sub>7</sub>) / 8 * &Delta;x
///
///  and,
///
/// > f<sub>y</sub> = (z<sub>7</sub> - z<sub>5</sub> + 2(z<sub>8</sub> - z<sub>4</sub>) + z<sub>1</sub> - z<sub>3</sub>) / 8 * &Delta;y
///
/// &Delta;x and &Delta;y are the grid resolutions in the x and y direction respectively
///
/// # Reference
/// Gallant, J. C., and J. P. Wilson, 2000, Primary topographic attributes, in Terrain Analysis: Principles
/// and Applications, edited by J. P. Wilson and J. C. Gallant pp. 51-86, John Wiley, Hoboken, N.J.
///
/// # See Also
/// `Slope`, `PlanCurvature`, `ProfileCurvature`
pub struct Aspect {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Aspect {
    pub fn new() -> Aspect {
        // public constructor
        let name = "Aspect".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates an aspect raster from an input DEM.".to_string();

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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        Aspect {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Aspect {
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
        let mut z_factor = -1f64;

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
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let start = Instant::now();

        let eight_grid_res = input.configs.resolution_x * 8.0;

        let mut output = Raster::initialize_using_file(&output_file, &input);
        if output.configs.data_type != DataType::F32 && output.configs.data_type != DataType::F64 {
            output.configs.data_type = DataType::F32;
        }
        let output_nodata = -999.0;
        output.configs.nodata = output_nodata;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let (mut fx, mut fy): (f64, f64);
                let mut z_factor_array = Vec::with_capacity(rows as usize);
                if input.is_in_geographic_coordinates() && z_factor < 0.0 {
                    // calculate a new z-conversion factor
                    for row in 0..rows {
                        let lat = input.get_y_from_row(row);
                        z_factor_array.push(1.0 / (111320.0 * lat.cos()));
                    }
                } else {
                    if z_factor < 0.0 {
                        z_factor = 1.0;
                    }
                    z_factor_array = vec![z_factor; rows as usize];
                }
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![output_nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            for c in 0..8 {
                                n[c] = input[(row + dy[c], col + dx[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor_array[row as usize];
                                } else {
                                    n[c] = z * z_factor_array[row as usize];
                                }
                            }
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            if fx == 0f64 {
                                fx = 0.00001;
                            }
                            // if fx != 0f64 {
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            data[col as usize] = 180f64 - ((fy / fx).atan()).to_degrees()
                                + 90f64 * (fx / (fx).abs());
                            // } else {
                            //     data[col as usize] = -1f64;
                            // }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
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
        output.configs.palette = "pointer.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
