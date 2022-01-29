/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/06/2017
Last Modified: 12/01/2022
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
use whitebox_common::utils::{
    get_formatted_elapsed_time, 
    haversine_distance,
    vincenty_distance
};

/// This tool calculates the plan (or contour) curvature from a digital elevation model (DEM). Plan curvature 
/// is the curvature of a contour line at a given point on the topographic surface (Florinsky, 2017). This 
/// variable has an unbounded range that can take either positive or negative values. Positive values of the 
/// index are indicative of flow divergence while negative plan curvature values indicate flow convergence. 
/// Thus plan curvature is similar to tangential curvature, although the literature suggests that the latter
/// is more numerically stable (Wilson, 2018). Plan curvature is measured in units of m<sup>-1</sup>.
/// 
/// ![](../../doc_img/PlanCurvature.png)
/// 
/// The user must specify the name of the input DEM (`--dem`) and the output raster (`--output`). The
/// The Z conversion factor (`--zfactor`) is only important when the vertical and horizontal units are not the
/// same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the
/// Z Conversion Factor. Curvature values are often very small and as such the user may opt to log-transform
/// the output raster (`--log`). Transforming the values applies the equation by Shary et al. (2002):
/// 
/// *Θ*' = sign(*Θ*) ln(1 + 10<sup>*n*</sup>|*Θ*|) 
/// 
/// where *Θ* is the parameter value and *n* is dependent on the grid cell size.
/// 
/// For DEMs in projected coordinate systems, the tool uses the 3rd-order bivariate 
/// Taylor polynomial method described by Florinsky (2016). Based on a polynomial fit 
/// of the elevations within the 5x5 neighbourhood surrounding each cell, this method is considered more 
/// robust against outlier elevations (noise) than other methods. For DEMs in geographic coordinate systems
/// (i.e. angular units), the tool uses the 3x3 polynomial fitting method for equal angle grids also 
/// described by Florinsky (2016). 
///
/// # References
/// Florinsky, I. (2016). Digital terrain analysis in soil science and geology. Academic Press.
/// 
/// Florinsky, I. V. (2017). An illustrated introduction to general geomorphometry. Progress in Physical 
/// Geography, 41(6), 723-752.
/// 
/// Shary P. A., Sharaya L. S. and Mitusov A. V. (2002) Fundamental quantitative methods of land surface analysis. 
/// Geoderma 107: 1–32.
/// 
/// Wilson, J. P. (2018). Environmental applications of digital terrain modeling. John Wiley & Sons.
///
/// `TangentialCurvature`, `ProfileCurvature`, `MeanCurvature`, `GaussianCurvature`, `MinimalCurvature`, `MaximalCurvature`
pub struct PlanCurvature {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PlanCurvature {
    pub fn new() -> PlanCurvature {
        // public constructor
        let name = "PlanCurvature".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates a plan (contour) curvature raster from an input DEM.".to_string();

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
            name: "Log-transform the output?".to_owned(),
            flags: vec!["--log".to_owned()],
            description: "Display output values using a log-scale."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        PlanCurvature {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PlanCurvature {
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
        let tool_name = self.get_tool_name();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        // Read in the environment variables and get the necessary values
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;

        // read the arguments
        let mut input_file: String = String::new();
        let mut output_file: String = String::new();
        let mut log_transform = false;
        let mut z_factor = 1f64;
        if args.len() <= 1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with too few parameters.",
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
            } else if flag_val == "-zfactor" {
                z_factor = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-log" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    log_transform = true;
                }
            }
        }

        if verbose {
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // Read in the input raster
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let resx = input.configs.resolution_x;
        let resy = input.configs.resolution_y;
        let res = (resx + resy) / 2.;
        
        let mut num_procs = num_cpus::get() as isize;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        if !input.is_in_geographic_coordinates() {
            // Based on Florinsky (2016) pg. 246
            let log_multiplier = match res {
                x if x >= 0. && x < 1. => { 10f64.powi(2) },
                x if x >= 1. && x < 10. => { 10f64.powi(3) },
                x if x >= 10. && x < 100. => { 10f64.powi(4) },
                x if x >= 100. && x < 1000. => { 10f64.powi(5) },
                x if x >= 1000. && x < 5000. => { 10f64.powi(6) },
                x if x >= 5000. && x < 10000. => { 10f64.powi(7) },
                x if x >= 10000. && x < 75000. => { 10f64.powi(8) },
                _ => { 10f64.powi(9) },
            };

            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z12: f64;
                    let mut p: f64;
                    let mut q: f64;
                    let mut r: f64;
                    let mut s: f64;
                    let mut t: f64;
                    let mut plan_curv: f64;
                    let offsets = [
                        [-2, -2], [-1, -2], [0, -2], [1, -2], [2, -2], 
                        [-2, -1], [-1, -1], [0, -1], [1, -1], [2, -1], 
                        [-2, 0], [-1, 0], [0, 0], [1, 0], [2, 0], 
                        [-2, 1], [-1, 1], [0, 1], [1, 1], [2, 1], 
                        [-2, 2], [-1, 2], [0, 2], [1, 2], [2, 2]
                    ];
                    let mut z = [0f64; 25];
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            z12 = input.get_value(row, col);
                            if z12 != nodata {
                                for n in 0..25 {
                                    z[n] = input.get_value(row + offsets[n][0], col + offsets[n][1]);
                                    if z[n] != nodata {
                                        z[n] *= z_factor;
                                    } else {
                                        z[n] = z12 * z_factor;
                                    }
                                }

                                /* 
                                The following equations have been taken from Florinsky (2016) Principles and Methods
                                of Digital Terrain Modelling, Chapter 4, pg. 117. Note that I believe Florinsky reversed
                                the equations for q and p.
                                */
                                r = 1f64 / (35f64 * res * res) * (2. * (z[0] + z[4] + z[5] + z[9] + z[10] + z[14] + z[15] + z[19] + z[20] + z[24])
                                - 2. * (z[2] + z[7] + z[12] + z[17] + z[22]) - z[1] - z[3] - z[6] - z[8]
                                - z[11] - z[13] - z[16] - z[18] - z[21] - z[23]);

                                t = 1f64 / (35f64 * res * res) * (2. * (z[0] + z[1] + z[2] + z[3] + z[4] + z[20] + z[21] + z[22] + z[23] + z[24])
                                - 2. * (z[10] + z[11] + z[12] + z[13] + z[14]) - z[5] - z[6] - z[7] - z[8]
                                - z[9] - z[15] - z[16] - z[17] - z[18] - z[19]);

                                s = 1. / (100. * res * res) * (z[8] + z[16] - z[6] - z[18] + 4. * (z[4] + z[20] - z[0] - z[24])
                                + 2. * (z[3] + z[9] + z[15] + z[21] - z[1] - z[5] - z[19] - z[23]));

                                q = 1. / (420. * res) * (44. * (z[3] + z[23] - z[1] - z[21]) + 31. * (z[0] + z[20] - z[4] - z[24]
                                + 2. * (z[8] + z[18] - z[6] - z[16])) + 17. * (z[14] - z[10] + 4. * (z[13] - z[11]))
                                + 5. * (z[9] + z[19] - z[5] - z[15]));

                                p = 1. / (420. * res) * (44. * (z[5] + z[9] - z[15] - z[19]) + 31. * (z[20] + z[24] - z[0] - z[4]
                                    + 2. * (z[6] + z[8] - z[16] - z[18])) + 17. * (z[2] - z[22] + 4. * (z[7] - z[17]))
                                    + 5. * (z[1] + z[3] - z[21] - z[23]));

                                if (p + q).abs() > 0. {
                                    /* 
                                    The following equation has been taken from Florinsky (2016) Principles and Methods
                                    of Digital Terrain Modelling, Chapter 2, pg. 19.
                                    */
                                    plan_curv = -(q * q * r - 2. * p * q * s + p * p * t) / ((p * p + q * q).powi(3).sqrt());
                                    if log_transform {
                                        // Based on Florinsky (2016) pg. 244 eq. 8.1
                                        plan_curv = plan_curv.signum() * (1. + log_multiplier * plan_curv.abs()).ln();
                                    }
                                } else {
                                    plan_curv = 0.;
                                }
                                data[col as usize] = plan_curv;
                            }
                        }

                        tx.send((row, data)).unwrap();
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
            
            // Based on Florinsky (2016) pg. 246
            let log_multiplier = match linear_res {
                x if x >= 0. && x < 1. => { 10f64.powi(2) },
                x if x >= 1. && x < 10. => { 10f64.powi(3) },
                x if x >= 10. && x < 100. => { 10f64.powi(4) },
                x if x >= 100. && x < 1000. => { 10f64.powi(5) },
                x if x >= 1000. && x < 5000. => { 10f64.powi(6) },
                x if x >= 5000. && x < 10000. => { 10f64.powi(7) },
                x if x >= 10000. && x < 75000. => { 10f64.powi(8) },
                _ => { 10f64.powi(9) },
            };

            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z4: f64;
                    let mut p: f64;
                    let mut q: f64;
                    let mut r: f64;
                    let mut s: f64;
                    let mut t: f64;
                    let mut a: f64;
                    let mut b: f64;
                    let mut c: f64;
                    let mut d: f64;
                    let mut e: f64;
                    let mut phi1: f64;
                    let mut lambda1: f64;
                    let mut phi2: f64;
                    let mut lambda2: f64;
                    let mut plan_curv: f64;
                    let offsets = [
                        [-1, -1], [0, -1], [1, -1], 
                        [-1, 0], [0, 0], [1, 0], 
                        [-1, 1], [0, 1], [1, 1]
                    ];
                    let mut z = [0f64; 25];
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

                                r = (c * c * (z[0] + z[2] - 2. * z[1]) + b * b * (z[3] + z[5] - 2. * z[4]) + a * a * (z[6] + z[8] - 2. * z[7]))
                                / (a.powi(4) + b.powi(4) + c.powi(4));

                                t = 2. / (3. * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4)))
                                * ((d * (a.powi(4) + b.powi(4) + b * b * c * c) - c * c * e * (a * a - b * b)) * (z[0] + z[2])
                                - (d * (a.powi(4) + c.powi(4) + b * b * c * c) + e * (a.powi(4) + c.powi(4) + a * a * b * b)) * (z[3] + z[5])
                                + (e * (b.powi(4) + c.powi(4) + a * a * b * b) + a * a * d * (b * b - c * c)) * (z[6] + z[8])
                                + d * (b.powi(4) * (z[1] - 3. * z[4]) + c.powi(4) * (3. * z[1] - z[4]) + (a.powi(4) - 2. * b * b * c * c) * (z[1] - z[4]))
                                + e * (a.powi(4) * (3. * z[7] - z[4]) + b.powi(4) * (z[7] - 3. * z[4]) + (c.powi(4) - 2. * a * a * b * b) * (z[7] - z[4]))
                                - 2. * (a * a * d * (b * b - c * c) * z[7] - c * c * e * (a * a - b * b) * z[1]));

                                s = (c * (a * a * (d + e) + b * b * e) * (z[2] - z[0]) - b * (a * a * d - c * c * e) * (z[3] - z[5]) + a * (c * c * (d + e) + b * b * d) * (z[6] - z[8]))
                                / (2. * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

                                p = (a * a * c * d * (d + e) * (z[2] - z[0]) + b * (a * a * d * d + c * c * e * e) * (z[5] - z[3]) + a * c * c * e * (d + e) * (z[8] - z[6]))
                                / (2. * (a * a * c * c * (d + e).powi(2) + b * b * (a * a * d * d + c * c * e * e)));

                                q = 1. / (3. * d * e * (d + e) * (a.powi(4) + b.powi(4) + c.powi(4))) 
                                * ((d * d * (a.powi(4) + b.powi(4) + b * b * c * c) + c * c * e * e * (a * a - b * b)) * (z[0] + z[2])
                                - (d * d * (a.powi(4) + c.powi(4) + b * b * c * c) - e * e * (a.powi(4) + c.powi(4) + a * a * b * b)) * (z[3] + z[5])
                                - (e * e * (b.powi(4) + c.powi(4) + a * a * b * b) - a * a * d * d * (b * b - c * c)) * (z[6] + z[8])
                                + d * d * (b.powi(4) * (z[1] - 3. * z[4]) + c.powi(4) * (3. * z[1] - z[4]) + (a.powi(4) - 2. * b * b * c * c) * (z[1] - z[4]))
                                + e * e * (a.powi(4) * (z[4] - 3. * z[7]) + b.powi(4) * (3. * z[4] - z[7]) + (c.powi(4) - 2. * a * a * b * b) * (z[4] - z[7]))
                                - 2. * (a * a * d * d * (b * b - c * c) * z[7] + c * c * e * e * (a * a - b * b) * z[1]));

                                if (p + q).abs() > 0. {
                                    /* 
                                    The following equation has been taken from Florinsky (2016) Principles and Methods
                                    of Digital Terrain Modelling, Chapter 2, pg. 19.
                                    */
                                    plan_curv = -(q * q * r - 2. * p * q * s + p * p * t) / ((p * p + q * q).powi(3).sqrt());
                                    if log_transform {
                                        // Based on Florinsky (2016) pg. 244 eq. 8.1
                                        plan_curv = plan_curv.signum() * (1. + log_multiplier * plan_curv.abs()).ln();
                                    }
                                } else {
                                    plan_curv = 0.;
                                }
                                data[col as usize] = plan_curv;
                            }
                        }

                        tx.send((row, data)).unwrap();
                    }
                });
            }
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        for row in 0..rows {
            let (r, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(r, data);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        
        //////////////////////
        // Output the image //
        //////////////////////
        if verbose {
            println!("Saving data...")
        };
        
        let elapsed_time = get_formatted_elapsed_time(start);
        
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            tool_name
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
