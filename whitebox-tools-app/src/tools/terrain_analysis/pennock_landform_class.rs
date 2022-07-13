/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/07/2017
Last Modified: 03/09/2020
License: MIT

Notes: Some degree of DEM smoothing is likely required to get reasonable results.
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

/// Tool can be used to perform a simple landform classification based on measures of slope gradient
/// and curvature derived from a user-specified digital elevation model (DEM). The classification
/// scheme is based on the method proposed by Pennock, Zebarth, and DeJong (1987). The scheme divides
/// a landscape into seven element types, including: convergent footslopes (CFS), divergent footslopes
/// (DFS), convergent shoulders (CSH), divergent shoulders (DSH), convergent backslopes (CBS), divergent
/// backslopes (DBS), and level terrain (L). The output raster image will record each of these base element
/// types as:
///
///  Element Type  |  Code
///  ------------- | -------
///  CFS           |  1
///  DFS           |  2
///  CSH           |  3
///  DSH           |  4
///  CBS           |  5
///  DBS           |  6
///  L             |  7
///
/// The definition of each of the elements, based on the original Pennock et al. (1987) paper, is
/// as follows:
///
/// |    PROFILE             |   GRADIENT    |   PLAN         |  Element |
/// |:-----------------------|:--------------|:---------------|:-------- |
/// | Concave ( -0.10)       |  High >3.0    | Concave 0.0    |  CFS     |
/// | Concave ( -0.10)       |  High >3.0    | Convex >0.0    |  DFS     |
/// | Convex (>0.10)         |  High >3.0    | Concave 0.0    |  CSH     |
/// | Convex (>0.10)         |  High >3.0    | Convex >0.0    |  DSH     |
/// | Linear (-0.10...0.10)  |  High >3.0    | Concave 0.0    |  CBS     |
/// | Linear (-0.10...0.10)  |  High >3.0    | Convex >0.0    |  DBS     |
/// | --                     |  Low 3.0      | --             |  L       |
///
///
/// Where PROFILE is profile curvature, GRADIENT is the slope gradient, and PLAN is the plan curvature.
/// Note that these values are likely landscape and data specific and can be adjusted by the user.
/// Landscape classification schemes that are based on terrain attributes are highly sensitive to
/// short-range topographic variability (i.e. roughness) and can benefit from pre-processing the DEM
/// with a smoothing filter to reduce the effect of surface roughness and emphasize the longer-range
/// topographic signal. The `FeaturePreservingSmoothing` tool
/// offers excellent performance in smoothing DEMs without removing the sharpness of breaks-in-slope.
///
/// # Reference
/// Pennock, D.J., Zebarth, B.J., and DeJong, E. (1987) Landform classification and soil distribution
/// in hummocky terrain, Saskatchewan, Canada. Geoderma, 40: 297-315.
///
/// # See Also
/// `FeaturePreservingSmoothing`
pub struct PennockLandformClass {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PennockLandformClass {
    pub fn new() -> PennockLandformClass {
        // public constructor
        let name = "PennockLandformClass".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Classifies hillslope zones based on slope, profile curvature, and plan curvature."
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
            name: "Slope Threshold (degrees)".to_owned(),
            flags: vec!["--slope".to_owned()],
            description: "Slope threshold value, in degrees (default is 3.0)".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("3.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Profile Curvature Threshold".to_owned(),
            flags: vec!["--prof".to_owned()],
            description: "Profile curvature threshold value (default is 0.1)".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.1".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Plan Curvature Threshold".to_owned(),
            flags: vec!["--plan".to_owned()],
            description: "Plan curvature threshold value (default is 0.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --slope=3.0 --prof=0.1 --plan=0.0", short_exe, name).replace("*", &sep);

        PennockLandformClass {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PennockLandformClass {
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
        let mut z_factor = -1f64;
        let mut slope_threshold = 3f64;
        let mut prof_threshold = 0.1_f64;
        let mut plan_threshold = 0f64;

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
            } else if flag_val == "-slope" {
                if keyval {
                    slope_threshold = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    slope_threshold = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-prof" {
                if keyval {
                    prof_threshold = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    prof_threshold = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-plan" {
                if keyval {
                    plan_threshold = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    plan_threshold = args[i + 1]
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

        let cell_size = input.configs.resolution_x;
        let cell_size_times2 = cell_size * 2.0f64;
        let cell_size_sqrd = cell_size * cell_size;
        let four_times_cell_size_sqrd = cell_size_sqrd * 4.0f64;
        let eight_grid_res = cell_size * 8.0;

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

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        output.configs.nodata = -128f64;
        output.configs.data_type = DataType::I8;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous; //Categorical;
                                                                                   // output.configs.palette = "qual.plt".to_string();

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
                let (mut zx, mut zy, mut zxx, mut zyy, mut zxy, mut zx2, mut zy2): (
                    f64,
                    f64,
                    f64,
                    f64,
                    f64,
                    f64,
                    f64,
                );
                let mut p: f64;
                let mut q: f64;
                let (mut fx, mut fy): (f64, f64);
                let mut slope: f64;
                let mut plan: f64;
                let mut prof: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![-128f64; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            z = z * z_factor;
                            for c in 0..8 {
                                n[c] = input[(row + dy[c], col + dx[c])];
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z;
                                }
                            }
                            // calculate curvature
                            zx = (n[1] - n[5]) / cell_size_times2;
                            zy = (n[7] - n[3]) / cell_size_times2;
                            zxx = (n[1] - 2.0f64 * z + n[5]) / cell_size_sqrd;
                            zyy = (n[7] - 2.0f64 * z + n[3]) / cell_size_sqrd;
                            zxy = (-n[6] + n[0] + n[4] - n[2]) / four_times_cell_size_sqrd;
                            zx2 = zx * zx;
                            zy2 = zy * zy;
                            p = zx2 + zy2;
                            q = p + 1f64;
                            if p > 0.0f64 {
                                fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2])
                                    / eight_grid_res;
                                fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6])
                                    / eight_grid_res;
                                slope = (fx * fx + fy * fy).sqrt().atan().to_degrees();
                                plan = -1f64
                                    * ((zxx * zy2 - 2f64 * zxy * zx * zy + zyy * zx2)
                                        / (p * q.powf(1.5f64)))
                                    .to_degrees();
                                prof = -1f64
                                    * ((zxx * zx2 - 2f64 * zxy * zx * zy + zyy * zy2)
                                        / (p * q.powf(1.5f64)))
                                    .to_degrees();

                                if prof < -prof_threshold
                                    && plan <= -plan_threshold
                                    && slope > slope_threshold
                                {
                                    //Convergent Footslope
                                    data[col as usize] = 1f64;
                                } else if prof < -prof_threshold
                                    && plan > plan_threshold
                                    && slope > slope_threshold
                                {
                                    //Divergent Footslope
                                    data[col as usize] = 2f64;
                                } else if prof > prof_threshold
                                    && plan <= plan_threshold
                                    && slope > slope_threshold
                                {
                                    //Convergent Shoulder
                                    data[col as usize] = 3f64;
                                } else if prof > prof_threshold
                                    && plan > plan_threshold
                                    && slope > slope_threshold
                                {
                                    //Divergent Shoulder
                                    data[col as usize] = 4f64;
                                } else if prof >= -prof_threshold
                                    && prof < prof_threshold
                                    && slope > slope_threshold
                                    && plan <= -plan_threshold
                                {
                                    //Convergent Backslope
                                    data[col as usize] = 5f64;
                                } else if prof >= -prof_threshold
                                    && prof < prof_threshold
                                    && slope > slope_threshold
                                    && plan > plan_threshold
                                {
                                    //Divergent Backslope
                                    data[col as usize] = 6f64;
                                } else if slope <= slope_threshold {
                                    //Level
                                    data[col as usize] = 7f64;
                                }
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

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
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
        output.add_metadata_entry(format!("Slope threshold: {}", slope_threshold));
        output.add_metadata_entry(format!("Profile curvature threshold: {}", prof_threshold));
        output.add_metadata_entry(format!("Plan curvature threshold: {}", plan_threshold));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        output.add_metadata_entry(format!("CLASSIFICATION KEY"));
        output.add_metadata_entry(format!("Value  Class"));
        output.add_metadata_entry(format!("1      Convergent Footslope"));
        output.add_metadata_entry(format!("2      Divergent Footslope"));
        output.add_metadata_entry(format!("3      Convergent Shoulder"));
        output.add_metadata_entry(format!("4      Divergent Shoulder"));
        output.add_metadata_entry(format!("5      Convergent Backslope"));
        output.add_metadata_entry(format!("6      Divergent Backslope"));
        output.add_metadata_entry(format!("7      Level"));
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
            println!("CLASSIFICATION KEY");
            println!("Value  Class");
            println!("1      Convergent Footslope");
            println!("2      Divergent Footslope");
            println!("3      Convergent Shoulder");
            println!("4      Divergent Shoulder");
            println!("5      Convergent Backslope");
            println!("6      Divergent Backslope");
            println!("7      Level");

            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
