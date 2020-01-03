/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/05/2018
Last Modified: 29/03/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::StandardNormal;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool performs a stochastic analysis of depressions within a DEM, calculating the
/// probability of each cell belonging to a depression. This land-surface prameter
/// (p<sub>dep</sub>) has been widely applied in wetland and bottom-land mapping applications.
///
/// This tool differs from the original Whitebox GAT tool in a few significant ways:
///
/// 1. The Whitebox GAT tool took an error histogram as an input. In practice people found
///    it difficult to create this input. Usually they just generated a normal distribution
///    in a spreadsheet using information about the DEM root-mean-square-error (RMSE). As
///    such, this tool takes a RMSE input and generates the histogram internally. This is
///    more convienent for most applications but loses the flexibility of specifying the
///    error distribution more completely.
///
/// 2. The Whitebox GAT tool generated the error fields using the turning bands method.
///    This tool generates a random Gaussian error field with no spatial autocorrelation
///    and then applies local spatial averaging using a Gaussian filter (the size of
///    which depends of the error autocorrelation length input) to increase the level of
///    autocorrelation. We use the Fast Almost Gaussian Filter of Peter Kovesi (2010),
///    which uses five repeat passes of a mean filter, based on an integral image. This
///    filter method is highly efficient. This results in a significant performance
///    increase compared with the original tool.
///
/// 3. Parts of the tool's workflow utilize parallel processing. However, the depression
///    filling operation, which is the most time-consuming part of the workflow, is
///    not parallelized.
///
/// In addition to the input DEM (`--dem`) and output p<sub>dep</sub> file name (`--output`), the user
/// must specify the nature of the error model, including the root-mean-square error (`--rmse`) and
/// the error field correlation length (`--range`). These parameters determine the statistical frequency
/// distribution and spatial characteristics of the modeled error fields added to the DEM in each
/// iteration of the simulation. The user must also specify the number of iterations (`--iterations`).
/// A larger number of iterations will produce a smoother p<sub>dep</sub> raster.
///
/// This tool creates several temporary rasters in memory and, as a result, is very memory hungry.
/// This will necessarily limit the size of DEMs that can be processed on more memory-constrained
/// systems. As a rough guide for usage, **the computer system will need 6-10 times more memory than
/// the file size of the DEM**. If your computer possesses insufficient memory, you may consider
/// splitting the input DEM apart into smaller tiles.
///
/// # Reference
/// Lindsay, J. B., & Creed, I. F. (2005). Sensitivity of digital landscapes to artifact depressions in
/// remotely-sensed DEMs. Photogrammetric Engineering & Remote Sensing, 71(9), 1029-1036.
///
/// # See Also
/// `ImpoundmentSizeIndex`, `FastAlmostGaussianFilter`
pub struct StochasticDepressionAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl StochasticDepressionAnalysis {
    pub fn new() -> StochasticDepressionAnalysis {
        // public constructor
        let name = "StochasticDepressionAnalysis".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Preforms a stochastic analysis of depressions within a DEM.".to_string();

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
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "DEM root-mean-square-error (z units)".to_owned(), 
            flags: vec!["--rmse".to_owned()], 
            description: "The DEM's root-mean-square-error (RMSE), in z units. This determines error magnitude.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter {
            name: "Range of Autocorrelation (map units)".to_owned(),
            flags: vec!["--range".to_owned()],
            description: "The error field's correlation length, in xy-units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Iterations".to_owned(),
            flags: vec!["--iterations".to_owned()],
            description: "The number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("100".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=out.tif --rmse=10.0 --range=850.0 --iterations=2500", short_exe, name).replace("*", &sep);

        StochasticDepressionAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for StochasticDepressionAnalysis {
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
        let mut rmse = 1f64;
        let mut range = 1f64;
        let mut iterations = 100;

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
            } else if flag_val == "-rmse" {
                rmse = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-range" {
                range = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-iterations" {
                iterations = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut reference_cdf: Vec<Vec<f64>> = vec![];
        let mu = 0f64; // assume the mean error is zero
        let p_step = 6.0 * rmse / 99.0;
        for a in 0..100 {
            let x = -3.0 * rmse + a as f64 * p_step;
            // (1 / sqrt(2σ^2 * π)) * e^(-(x - μ)^2 / 2σ^2)
            let p = (1.0 / (2.0 * PI * rmse.powi(2)).sqrt())
                * (-(x - mu).powi(2) / (2.0 * rmse.powi(2))).exp();
            reference_cdf.push(vec![x, p]);
        }

        // convert the reference histogram to a cdf.
        let num_lines = reference_cdf.len();
        for i in 1..num_lines {
            reference_cdf[i][1] += reference_cdf[i - 1][1];
        }
        let total_frequency = reference_cdf[num_lines - 1][1];
        for i in 0..num_lines {
            reference_cdf[i][1] = reference_cdf[i][1] / total_frequency;
        }

        let mut starting_vals = [0usize; 11];
        let mut p_val: f64;
        for i in 0..num_lines {
            p_val = reference_cdf[i][1];
            if p_val < 0.1 {
                starting_vals[1] = i;
            }
            if p_val < 0.2 {
                starting_vals[2] = i;
            }
            if p_val < 0.3 {
                starting_vals[3] = i;
            }
            if p_val < 0.4 {
                starting_vals[4] = i;
            }
            if p_val < 0.5 {
                starting_vals[5] = i;
            }
            if p_val < 0.6 {
                starting_vals[6] = i;
            }
            if p_val < 0.7 {
                starting_vals[7] = i;
            }
            if p_val < 0.8 {
                starting_vals[8] = i;
            }
            if p_val < 0.9 {
                starting_vals[9] = i;
            }
            if p_val <= 1.0 {
                starting_vals[10] = i;
            }
        }

        if iterations > i16::max_value() as usize {
            if verbose {
                println!(
                    "Warning: Iterations cannot be higher than {}.",
                    i16::max_value()
                );
            }
            iterations = i16::max_value() as usize;
        }

        let input1 = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata = input1.configs.nodata;
        let sigma = range / input1.configs.resolution_x;
        let resolution = (input1.configs.resolution_x + input1.configs.resolution_y) / 2f64;
        let range_in_cells = range / resolution;

        let mut output_config = input1.configs.clone();
        output_config.data_type = DataType::F32;
        let mut freq_dep: Array2D<i16> = Array2D::new(rows, columns, 0i16, -1i16).unwrap();

        let nodata_i32 = i32::min_value();
        let mut input: Array2D<i32> = Array2D::new(rows, columns, nodata_i32, nodata_i32).unwrap();
        let mut z: f64;
        let multiplier = 1000f64;
        let mut num_nodata = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                z = input1.get_value(row, col);
                if z != nodata {
                    input.set_value(row, col, (z * multiplier) as i32);
                } else {
                    num_nodata += 1;
                }
            }
        }
        drop(input1);

        // num_nodata is used by the queue used to initialize the depression filling op.
        // It needs to be able to hold all of the edge cells in the very least.
        if num_nodata < ((rows + 2) * 2 + (columns + 2) * 2) as usize {
            num_nodata = ((rows + 2) * 2 + (columns + 2) * 2) as usize;
        }

        // let mut error_model: Array2D<i32> = Array2D::new(rows, columns, nodata_i32, nodata_i32).unwrap();
        let background_val = i32::min_value() + 1;
        let num_procs = num_cpus::get() as isize;
        let numcells: f64 = (rows * columns) as f64; // used by the histogram matching
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        for iter_num in 0..iterations {
            if verbose {
                println!("Iteration {}...", iter_num + 1);
            }

            /////////////////////////////
            // Generate a random field //
            /////////////////////////////

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut rng = SmallRng::from_entropy();
                    let mut sn_val: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![0i32; columns as usize];
                        for col in 0..columns {
                            sn_val = rng.sample(StandardNormal);
                            data[col as usize] =
                                (sn_val * multiplier * range_in_cells * 2f64) as i32;
                        }

                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut error_model: Array2D<i32> =
                Array2D::new(rows, columns, nodata_i32, nodata_i32).unwrap();
            for _ in 0..rows {
                let (row, data) = rx.recv().unwrap();
                error_model.set_row_data(row, data);
            }

            ////////////////////////////////////////
            // Perform a FastAlmostGaussianFilter //
            ////////////////////////////////////////
            let n = 5;
            let w_ideal = (12f64 * sigma * sigma / n as f64 + 1f64).sqrt();
            let mut wl = w_ideal.floor() as isize;
            if wl % 2 == 0 {
                wl -= 1;
            } // must be an odd integer
            let wu = wl + 2;
            let m = ((12f64 * sigma * sigma
                - (n * wl * wl) as f64
                - (4 * n * wl) as f64
                - (3 * n) as f64)
                / (-4 * wl - 4) as f64)
                .round() as isize;

            let mut val: i32;
            let mut sum: i32;
            let mut i_prev: i32;

            // Find the min and max values.
            let mut min_value = i32::max_value();
            let mut max_value = i32::min_value();
            let mut z: i32;

            for iteration_num in 0..n {
                let midpoint = if iteration_num <= m {
                    (wl as f64 / 2f64).floor() as isize
                } else {
                    (wu as f64 / 2f64).floor() as isize
                };

                // Create the integral image.
                let mut integral: Array2D<i32> =
                    Array2D::new(rows, columns, 0, nodata_i32).unwrap();
                for row in 0..rows {
                    sum = 0;
                    for col in 0..columns {
                        val = error_model.get_value(row, col);
                        sum += val;
                        if row > 0 {
                            i_prev = integral.get_value(row - 1, col);
                            integral.set_value(row, col, sum + i_prev);
                        } else {
                            integral.set_value(row, col, sum);
                        }
                    }
                }

                // Perform Filter
                let integral = Arc::new(integral);
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let tx = tx.clone();
                    let integral = integral.clone();
                    thread::spawn(move || {
                        let mut z: i32;
                        let mut sum: i32;
                        let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                        let mut num_cells: i32;
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            y1 = row - midpoint - 1;
                            if y1 < 0 {
                                y1 = 0;
                            }
                            y2 = row + midpoint;
                            if y2 >= rows {
                                y2 = rows - 1;
                            }
                            let mut data = vec![0i32; columns as usize];
                            let mut min_value = i32::max_value();
                            let mut max_value = i32::min_value();
                            for col in 0..columns {
                                x1 = col - midpoint - 1;
                                if x1 < 0 {
                                    x1 = 0;
                                }
                                x2 = col + midpoint;
                                if x2 >= columns {
                                    x2 = columns - 1;
                                }

                                num_cells = ((y2 - y1) * (x2 - x1)) as i32;
                                if num_cells > 0 {
                                    sum = integral[(y2, x2)] + integral[(y1, x1)]
                                        - integral[(y1, x2)]
                                        - integral[(y2, x1)];

                                    z = sum / num_cells;
                                    data[col as usize] = z;
                                    if z < min_value {
                                        min_value = z;
                                    }
                                    if z > max_value {
                                        max_value = z;
                                    }
                                }
                            }

                            tx.send((row, data, min_value, max_value)).unwrap();
                        }
                    });
                }

                for _ in 0..rows {
                    let (row, data, val1, val2) = rx.recv().unwrap();
                    error_model.set_row_data(row, data);
                    if val1 < min_value {
                        min_value = val1;
                    }
                    if val2 > max_value {
                        max_value = val2;
                    }
                }

                drop(integral);
            }

            ////////////////////////////////////////////
            // Perform a histogram matching operation //
            ////////////////////////////////////////////

            let num_bins = (max_value - min_value + 1) as usize;
            let mut histogram = vec![0f64; num_bins];
            let mut bin_num: usize;
            for row in 0..rows {
                for col in 0..columns {
                    z = error_model.get_value(row, col);
                    bin_num = (z - min_value) as usize;
                    histogram[bin_num] += 1f64;
                }
            }

            let mut cdf = vec![0f64; num_bins];
            cdf[0] = histogram[0];
            for i in 1..num_bins {
                cdf[i] = cdf[i - 1] + histogram[i];
            }
            for i in 0..num_bins {
                cdf[i] = cdf[i] / numcells;
            }

            drop(histogram);

            let mut j: usize;
            let mut x_val = 0f64;
            let mut p_val: f64;
            let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
            for row in 0..rows {
                for col in 0..columns {
                    z = error_model.get_value(row, col);
                    bin_num = (z - min_value) as usize;
                    p_val = cdf[bin_num];
                    j = ((p_val * 10f64).floor()) as usize;
                    for i in starting_vals[j]..num_lines {
                        if reference_cdf[i][1] > p_val {
                            if i > 0 {
                                x1 = reference_cdf[i - 1][0];
                                x2 = reference_cdf[i][0];
                                p1 = reference_cdf[i - 1][1];
                                p2 = reference_cdf[i][1];
                                x_val = if p1 != p2 {
                                    x1 + ((x2 - x1) * ((p_val - p1) / (p2 - p1)))
                                } else {
                                    x1
                                };
                            } else {
                                x_val = reference_cdf[i][0];
                            }
                            break;
                        }
                    }
                    error_model.set_value(row, col, (x_val * multiplier) as i32);
                }
            }

            drop(cdf);

            /////////////////////////////////////
            // Add the DEM to the error model. //
            /////////////////////////////////////
            let mut e: i32;
            for row in 0..rows {
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata_i32 {
                        e = error_model.get_value(row, col);
                        error_model.set_value(row, col, z + e);
                    } else {
                        error_model.set_value(row, col, nodata_i32);
                    }
                }
            }

            /////////////////////////////////////////////////
            // Fill the depressions in the error-added DEM //
            /////////////////////////////////////////////////

            /*
            Find the data edges. This is complicated by the fact that DEMs frequently
            have nodata edges, whereby the DEM does not occupy the full extent of
            the raster. One approach to doing this would be simply to scan the
            raster, looking for cells that neighbour nodata values. However, this
            assumes that there are no interior nodata holes in the dataset. Instead,
            the approach used here is to perform a region-growing operation, looking
            for nodata values along the raster's edges.
            */

            let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity(num_nodata);
            for row in 0..rows {
                /*
                Note that this is only possible because Whitebox rasters
                allow you to address cells beyond the raster extent but
                return the nodata value for these regions.
                */
                queue.push_back((row, -1));
                queue.push_back((row, columns));
            }

            for col in 0..columns {
                queue.push_back((-1, col));
                queue.push_back((rows, col));
            }

            let mut dep_filled: Array2D<i32> =
                Array2D::new(rows, columns, background_val, nodata_i32).unwrap();
            let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize - num_nodata);
            let mut zin_n: i32; // value of neighbour of row, col in input raster
            let mut zout: i32; // value of row, col in output raster
            let mut zout_n: i32; // value of neighbour of row, col in output raster
            let (mut row, mut col): (isize, isize);
            let (mut row_n, mut col_n): (isize, isize);
            while !queue.is_empty() {
                let cell = queue.pop_front().unwrap();
                row = cell.0;
                col = cell.1;
                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = col + dx[n];
                    zin_n = error_model.get_value(row_n, col_n);
                    zout_n = dep_filled.get_value(row_n, col_n);
                    if zout_n == background_val {
                        if zin_n == nodata_i32 {
                            dep_filled.set_value(row_n, col_n, nodata_i32);
                            queue.push_back((row_n, col_n));
                        } else {
                            dep_filled.set_value(row_n, col_n, zin_n);
                            // Push it onto the priority queue for the priority flood operation
                            minheap.push(GridCell {
                                id: row_n * columns + col_n,
                                priority: zin_n,
                            });
                        }
                    }
                }
            }

            drop(queue);

            // Perform the priority flood operation.
            while !minheap.is_empty() {
                let cell = minheap.pop().unwrap();
                row = cell.id / columns;
                col = cell.id % columns;
                zout = dep_filled.get_value(row, col);
                for n in 0..8 {
                    row_n = row + dy[n];
                    col_n = col + dx[n];
                    zout_n = dep_filled.get_value(row_n, col_n);
                    if zout_n == background_val {
                        zin_n = error_model.get_value(row_n, col_n);
                        if zin_n != nodata_i32 {
                            if zin_n < zout {
                                zin_n = zout;
                                // Depression cell; increase its value in output
                                freq_dep.increment(row_n, col_n, 1i16);
                            } // We're in a depression. Raise the elevation.
                            dep_filled.set_value(row_n, col_n, zin_n);
                            minheap.push(GridCell {
                                id: row_n * columns + col_n,
                                priority: zin_n,
                            });
                        } else {
                            // Interior nodata cells are still treated as nodata and are not filled.
                            dep_filled.set_value(row_n, col_n, nodata_i32);
                        }
                    }
                }
            }

            drop(minheap);
            drop(dep_filled);

            if verbose {
                progress = (100.0_f64 * (iter_num + 1) as f64 / iterations as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let iters = iterations as f64;
        let mut output = Raster::initialize_using_config(&output_file, &output_config);
        for row in 0..rows {
            for col in 0..columns {
                if input.get_value(row, col) != nodata_i32 {
                    output.set_value(row, col, freq_dep.get_value(row, col) as f64 / iters);
                } else {
                    output.set_value(row, col, nodata);
                }
            }
        }

        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input base raster file: {}", input_file));
        output.add_metadata_entry(format!("RMSE: {}", rmse));
        output.add_metadata_entry(format!("Range: {}", range));
        output.add_metadata_entry(format!("Iterations: {}", iterations));
        let elapsed_time = get_formatted_elapsed_time(start);
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

struct GridCell {
    id: isize,
    priority: i32,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell {
    fn cmp(&self, other: &GridCell) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialEq for GridCell {
    fn eq(&self, other: &GridCell) -> bool {
        self.priority == other.priority
    }
}
