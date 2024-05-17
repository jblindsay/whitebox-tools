/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/11/2017
Last Modified: 03/09/2020
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use num_cpus;
use std::env;
use std::io::{Error, ErrorKind};
use std::ops::AddAssign;
use std::ops::SubAssign;
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::{f32, f64};

/// This tool implements a highly modified form of the DEM de-noising algorithm described
/// by Sun et al. (2007). It is very effective at removing surface roughness from digital
/// elevation models (DEMs), without significantly altering breaks-in-slope. As such,
/// this tool should be used for smoothing DEMs rather than either smoothing with
/// low-pass filters (e.g. mean, median, Gaussian filters) or grid size coarsening
/// by resampling. The algorithm works by 1) calculating the surface normal 3D vector
/// of each grid cell in the DEM, 2) smoothing the normal vector field using a
/// filtering scheme that applies more weight to neighbours with lower angular difference
/// in surface normal vectors, and 3) uses the smoothed normal vector field to update
/// the elevations in the input DEM.
///
/// Sun et al.'s (2007) original method was intended to work on input point clouds and
/// fitted triangular irregular networks (TINs). The algorithm has been modified to
/// work with input raster DEMs instead. In so doing, this algorithm calculates surface
/// normal vectors from the planes fitted to 3 x 3 neighbourhoods surrounding each
/// grid cell, rather than the triangular facet. The normal vector field smoothing and
/// elevation updating procedures are also based on raster filtering operations. These
/// modifications make this tool more efficient than Sun's original method, but will
/// also result in a slightly different output than what would be achieved with Sun's
/// method.
///
/// The user must specify the values of three key parameters, including the filter size
/// (`--filter`), the normal difference threshold (`--norm_diff`), and the number of
/// iterations (`--num_iter`). Lindsay et al. (2019) found that **the degree of smoothing
/// was less impacted by the filter size than it was either the normal difference threshold
/// and the number of iterations**. A filter size of 11, the default value, tends to work
/// well in many cases. To increase the level of smoothing applied to the DEM, consider
/// increasing the normal difference threshold, i.e. the angular difference in normal vectors
/// between the center cell of a filter window and a neighbouring cell. This parameter determines
/// which neighbouring values are included in a filtering operation and higher values will
/// result in a greater number of neighbouring cells included, and therefore smooother surfaces.
/// Similarly, increasing the number of iterations from the default value of 3 to upwards of
/// 5-10 will result in significantly greater smoothing.
///
/// Before smoothing treatment:
/// ![](../../doc_img/FeaturePreservingSmoothing_fig1.png)
///
/// After smoothing treatment with FPS:
/// ![](../../doc_img/FeaturePreservingSmoothing_fig2.png)
///
/// For a video tutorial on how to use the `FeaturePreservingSmoothing` tool, please see
/// [this YouTube video](https://www.youtube.com/watch?v=FM3It51L7ZA&t=421s).
///
/// # Reference
/// Lindsay JB, Francioni A, Cockburn JMH. 2019. LiDAR DEM smoothing and the preservation of
/// drainage features. *Remote Sensing*, 11(16), 1926; DOI: 10.3390/rs11161926.
///
/// Sun, X., Rosin, P., Martin, R., & Langbein, F. (2007). Fast and effective feature-preserving
/// mesh denoising. *IEEE Transactions on Visualization & Computer Graphics*, (5), 925-938.
pub struct FeaturePreservingSmoothing {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FeaturePreservingSmoothing {
    pub fn new() -> FeaturePreservingSmoothing {
        // public constructor
        let name = "FeaturePreservingSmoothing".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Reduces short-scale variation in an input DEM using a modified Sun et al. (2007) algorithm.".to_string();

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
            name: "Filter Size".to_owned(),
            flags: vec!["--filter".to_owned()],
            description: "Size of the filter kernel.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("11".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Normal Difference Threshold".to_owned(),
            flags: vec!["--norm_diff".to_owned()],
            description: "Maximum difference in normal vectors, in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("15.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Iterations".to_owned(),
            flags: vec!["--num_iter".to_owned()],
            description: "Number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("3".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum Elevation Change".to_owned(),
            flags: vec!["--max_diff".to_owned()],
            description: "Maximum allowable absolute elevation change (optional).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --filter=15 --norm_diff=20.0 --num_iter=4",
            short_exe, name
        ).replace("*", &sep);

        FeaturePreservingSmoothing {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FeaturePreservingSmoothing {
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
        let mut filter_size = 11usize;
        let mut max_norm_diff = 8f32;
        let mut num_iter = 3;
        let mut z_factor = -1f32;
        let mut max_z_diff = f32::INFINITY;

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
            } else if flag_val == "-filter" {
                filter_size = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-norm_diff" {
                max_norm_diff = if keyval {
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
            } else if flag_val == "-num_iter" {
                num_iter = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-zfactor" {
                z_factor = if keyval {
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
            } else if flag_val == "-max_diff" {
                max_z_diff = if keyval {
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

        if filter_size < 3 {
            filter_size = 3;
        }
        if num_iter < 1 {
            num_iter = 1;
        }
        // if max_norm_diff > 90f32 {
        //     max_norm_diff = 90f32;
        // }
        if max_norm_diff > 180f32 {
            max_norm_diff = 180f32;
        }
        let threshold = max_norm_diff.to_radians().cos();

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
        }

        let input_dem = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        if input_dem.is_in_geographic_coordinates() && z_factor < 0.0 {
            // calculate a new z-conversion factor
            let mut mid_lat = (input_dem.configs.north - input_dem.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = (1.0 / (111320.0 * mid_lat.cos())) as f32;
                println!("It appears that the DEM is in geographic coordinates. The z-factor has been updated to {}.", z_factor);
            }
        } else if z_factor < 0.0 {
            z_factor = 1.0;
        }

        let input = Arc::new(input_dem.get_data_as_f32_array2d());
        let mut configs = input_dem.configs.clone();
        drop(input_dem);

        let rows = input.rows as isize;
        let columns = input.columns as isize;
        let nodata = input.nodata;
        let res_x = configs.resolution_x as f32;
        let res_y = configs.resolution_y as f32;
        // let eight_grid_res = ((res_x + res_y) / 2f32) * 8f32;
        let eight_res_x = res_x * 8f32;
        let eight_res_y = res_y * 8f32;

        /*
            Note: the normal should have a,b,c components to it since it is 3D. However, every pixel will
            have a c-value of 1.0 and as such, there is no point in including it in the
            storage of the normals and in the average analysis. It's effectively constant. This is one way
            to both significantly reduce the memory footprint of the tool and reduce the number of calculations
            required for the averaging.
        */

        ///////////////////////////////
        // Create the normal vectors //
        ///////////////////////////////
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
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut z: f32;
                let mut zn: f32;
                let (mut a, mut b): (f32, f32);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![Normal { a: 0f32, b: 0f32 }; columns as usize];
                    let mut values = [0f32; 9];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            for i in 0..8 {
                                zn = input.get_value(row + dy[i], col + dx[i]);
                                if zn != nodata {
                                    values[i] = (zn * z_factor) as f32;
                                } else {
                                    values[i] = (z * z_factor) as f32;
                                }
                            }
                            // from Horn 1981:
                            // Pw = t(z++ + 2z+o + z+.) - (z-+ + 2z_o + z--)]/8Ax
                            // Qw = t(z+++2zo++z-+)- (z+-+2zo-+z--)]/8A>
                            a = -(values[2] - values[4]
                                + 2f32 * (values[1] - values[5])
                                + values[0]
                                - values[6])
                                / eight_res_x;
                            b = -(values[6] - values[4]
                                + 2f32 * (values[7] - values[3])
                                + values[0]
                                - values[2])
                                / eight_res_y;
                            // Notice that these aren't unit vectors. By normalizing by c instead, we remove the need to store the c-value.
                            data[col as usize] = Normal { a: a, b: b };
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let zero_vector = Normal { a: 0f32, b: 0f32 };
        let mut nv: Array2D<Normal> = Array2D::new(rows, columns, zero_vector, zero_vector)?;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            nv.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating normal vectors: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let t1 = Instant::now();
        if verbose {
            println!(
                "{}",
                format!(
                    "Calculating normal vectors: {}",
                    get_formatted_elapsed_time(start)
                )
            );
        }

        //////////////////////////////////////////////////////////
        // Smooth the normal vector field of the fitted planes. //
        //////////////////////////////////////////////////////////
        let nv = Arc::new(nv);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let nv = nv.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let num_pixels_in_filter = filter_size * filter_size;
                let mut dx = vec![0isize; num_pixels_in_filter];
                let mut dy = vec![0isize; num_pixels_in_filter];

                // fill the filter d_x and d_y values and the distance-weights
                let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize;
                let mut a = 0;
                for row in 0..filter_size {
                    for col in 0..filter_size {
                        dx[a] = col as isize - midpoint;
                        dy[a] = row as isize - midpoint;
                        a += 1;
                    }
                }
                let mut z: f32;
                let (mut xn, mut yn): (isize, isize);
                let (mut a, mut b): (f32, f32);
                let mut diff: f32;
                let mut w: f32;
                let mut sum_w: f32;
                let threshold32 = threshold as f32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![Normal { a: 0f32, b: 0f32 }; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            sum_w = 0f32;
                            a = 0f32;
                            b = 0f32;
                            for n in 0..num_pixels_in_filter {
                                xn = col + dx[n];
                                yn = row + dy[n];
                                if input.get_value(yn, xn) != nodata {
                                    diff =
                                        nv.get_value(row, col).angle_between(nv.get_value(yn, xn));
                                    if diff > threshold32 {
                                        w = (diff - threshold32) * (diff - threshold32);
                                        sum_w += w;
                                        a += nv.get_value(yn, xn).a * w;
                                        b += nv.get_value(yn, xn).b * w;
                                    }
                                }
                            }

                            a /= sum_w;
                            b /= sum_w;

                            data[col as usize] = Normal { a: a, b: b };
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut nv_smooth: Array2D<Normal> = Array2D::new(rows, columns, zero_vector, zero_vector)?;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            nv_smooth.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Smoothing normal vectors: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            println!(
                "{}",
                format!(
                    "Smoothing normal vectors: {}",
                    get_formatted_elapsed_time(t1)
                )
            );
        }

        drop(nv);

        ///////////////////////////////////////////////////////////////////////////
        // Update the elevations of the DEM based on the smoothed normal vectors //
        ///////////////////////////////////////////////////////////////////////////
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let x = [-res_x, -res_x, -res_x, 0f32, res_x, res_x, res_x, 0f32];
        let y = [-res_y, 0f32, res_y, res_y, res_y, 0f32, -res_y, -res_y];
        let mut w: f32;
        let mut sum_w: f32;
        let mut diff: f32;
        let mut z: f32;
        let (mut xn, mut yn): (isize, isize);
        let mut zn: f32;

        // configs.nodata = nodata as f64;
        let mut output: Array2D<f32> = Array2D::new(rows, columns, nodata, nodata)?; //Raster::initialize_using_config(&output_file, &configs);
        for row in 0..rows {
            for col in 0..columns {
                output.set_value(row, col, input.get_value(row, col));
            }
        }
        // output.configs.data_type = DataType::F32; // if the input file is integer elevations, the output must be floating-point
        // let mut output = Arc::try_unwrap(input).unwrap_err().clone();

        if verbose {
            println!("Updating elevations...");
        }
        for loop_num in 0..num_iter {
            if verbose {
                println!("Iteration {} of {}...", loop_num + 1, num_iter);
            }

            for row in 0..rows {
                for col in 0..columns {
                    if input.get_value(row, col) != nodata {
                        sum_w = 0f32;
                        z = 0f32;
                        for n in 0..8 {
                            xn = col + dx[n];
                            yn = row + dy[n];
                            zn = output.get_value(yn, xn);
                            if zn != nodata {
                                diff = nv_smooth
                                    .get_value(row, col)
                                    .angle_between(nv_smooth.get_value(yn, xn));
                                if diff > threshold {
                                    w = (diff - threshold) * (diff - threshold);
                                    sum_w += w;
                                    z += -(nv_smooth.get_value(yn, xn).a * x[n]
                                        + nv_smooth.get_value(yn, xn).b * y[n]
                                        - 1f32 * zn)
                                        * w;
                                }
                            }
                        }
                        if sum_w > 0f32 {
                            // this is a division-by-zero safeguard and must be in place.
                            zn = z / sum_w;
                            if (zn - input.get_value(row, col)).abs() <= max_z_diff {
                                output.set_value(row, col, zn);
                            } else {
                                output.set_value(row, col, input.get_value(row, col));
                            }
                        } else {
                            output.set_value(row, col, input.get_value(row, col));
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Updating DEM elevations (Loop {} of {}): {}%",
                            loop_num + 1,
                            num_iter,
                            progress
                        );
                        old_progress = progress;
                    }
                }
            }
        }

        drop(nv_smooth);
        drop(input);

        configs.nodata = nodata as f64;
        let mut output_raster = Raster::initialize_from_array2d(&output_file, &configs, &output);
        output_raster.configs.data_type = DataType::F32;

        let elapsed_time = get_formatted_elapsed_time(start);
        output_raster.configs.display_min = configs.display_min;
        output_raster.configs.display_max = configs.display_max;
        output_raster.configs.palette = configs.palette.clone();
        output_raster.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_raster.add_metadata_entry(format!("Input file: {}", input_file));
        output_raster.add_metadata_entry(format!("Filter size: {}", filter_size));
        output_raster.add_metadata_entry(format!("Normal difference threshold: {}", max_norm_diff));
        output_raster.add_metadata_entry(format!("Iterations: {}", num_iter));
        output_raster.add_metadata_entry(format!("Max. z difference: {}", max_z_diff));
        output_raster.add_metadata_entry(format!("Z-factor: {}", z_factor));
        output_raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        }
        let _ = match output_raster.write() {
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

#[derive(Clone, Copy, Debug)]
struct Normal {
    a: f32,
    b: f32,
}

impl Normal {
    fn angle_between(self, other: Normal) -> f32 {
        /*
         Note that this is actually not the angle between the vectors but
         rather the cosine of the angle between the vectors. This improves
         the performance considerably. Also note that we do not need to worry
         about checking for division by zero here because 'c' will always be
         non-zero and therefore the vector magnitude cannot be zero.
        */
        // let denom = ((self.a * self.a + self.b * self.b + c * c)
        //     * (other.a * other.a + other.b * other.b + c * c))
        //     .sqrt();
        // (self.a * other.a + self.b * other.b + c * c) / denom

        let denom = ((self.a * self.a + self.b * self.b + 1f32)
            * (other.a * other.a + other.b * other.b + 1f32))
            .sqrt();
        (self.a * other.a + self.b * other.b + 1f32) / denom
    }
}

impl AddAssign for Normal {
    fn add_assign(&mut self, other: Normal) {
        *self = Normal {
            a: self.a + other.a,
            b: self.b + other.b,
        };
    }
}

impl SubAssign for Normal {
    fn sub_assign(&mut self, other: Normal) {
        *self = Normal {
            a: self.a - other.a,
            b: self.b - other.b,
        };
    }
}
