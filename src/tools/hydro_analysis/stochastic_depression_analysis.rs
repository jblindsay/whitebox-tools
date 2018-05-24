/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/05/2018
Last Modified: 11/05/2018
License: MIT

NOTES: This tool differs from the original Whitebox GAT tool in a few significant ways:

1. The Whitebox GAT tool took an error histogram as an input. In practice people found
   it difficult to create this input. Usually they just generated a normal distribution
   in a spreadsheet using information about the DEM RMSE. As such, this tool takes a
   RMSE input and generates the histogram internally. This is far more convienent for
   most applications but loses the flexibility of specifying the error distribution 
   more completely.

2. The Whitebox GAT tool generated the error fields using the turning bands method. 
   This tool generates a random Gaussian error field with no spatial autocorrelation
   and then applies local spatial averaging using a Gaussian filter (the size of 
   which depends of the error autocorrelation length input) to increase the level of
   autocorrelation. We use the Fast Almost Gaussian Filter of Peter Kovesi (2010), 
   which uses five repeat passes of a mean filter, based on an integral image. This
   filter method is highly efficient. This results in a very significant performance
   increase compared with the original tool.

3. The tool operates concurrently, compared with the original tool which calculated
   pdep in serial. Again, this parallel processing can significantly improve the 
   performance, particularly when the tool is applied on hardware with four or
   more processors.
*/

use time;
use num_cpus;
use rand::prelude::*;
use rand::distributions::StandardNormal;
use std::env;
use std::path;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::*;
use structures::Array2D;
use std::f64::consts::PI;

/// Preforms a stochastic analysis of depressions within a DEM.
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
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "DEM root-mean-square-error (z units)".to_owned(), 
            flags: vec!["--rmse".to_owned()], 
            description: "The DEM's root-mean-square-error (RMSE), in z units. This determines error magnitude.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Range of Autocorrelation (map units)".to_owned(), 
            flags: vec!["--range".to_owned()], 
            description: "The error field's correlation length, in xy-units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Iterations".to_owned(), 
            flags: vec!["--iterations".to_owned()], 
            description: "The number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1000".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut rmse = 1f64;
        let mut range = 1f64;
        let mut iterations = 1000;
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters."));
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
                    args[i+1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-range" {
                range = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i+1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-iterations" {
                iterations = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i+1].to_string().parse::<f32>().unwrap() as usize
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut reference_cdf: Vec<Vec<f64>> = vec![];
        let mu = 0f64; // assume the mean error is zero
        let p_step = 6f64 * rmse / (100.0-1f64);
        for a in 0..100 {
            let x = -3.0 * rmse + a as f64 * p_step;
            // (1 / sqrt(2σ^2 * π)) * e^(-(x - μ)^2 / 2σ^2)
            let p = (1f64 / (2f64*PI*rmse.powi(2)).sqrt()) * (-(x - mu).powi(2) / (2f64*rmse.powi(2))).exp();
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
            if p_val <= 1f64 {
                starting_vals[10] = i;
            }
        }
            

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let sigma = range / input.configs.resolution_x;

        let mut output = Raster::initialize_using_file(&output_file, &input); // will contain pdep
        output.reinitialize_values(0.0);

        let num_procs = num_cpus::get(); // as isize;
        let (tx, rx) = mpsc::channel();
        let starting_vals = Arc::new(starting_vals);
        let reference_cdf = Arc::new(reference_cdf);
        
        let iteration_list = Arc::new(Mutex::new(0..iterations));

        for _ in 0..num_procs {
            let tx = tx.clone();
            let input = input.clone();
            let starting_vals = starting_vals.clone();
            let reference_cdf = reference_cdf.clone();
            let iteration_list = iteration_list.clone();
            thread::spawn(move || {
                let mut out: Array2D<u16> = Array2D::new(rows, columns, 0u16, 0u16).unwrap();
                
                // let mut rng = thread_rng();
                // let normal = Normal::new(0.0, 1.0);
                let mut rng = SmallRng::from_entropy();

                let mut iter_num = 0;
                    
                while iter_num < iterations {

                    iter_num = match iteration_list.lock().unwrap().next() {
                        Some(val) => val, 
                        None => break, // There are no more tiles to interpolate
                    };

                    if verbose {
                        println!("Loop {} of {}", iter_num+1, iterations);
                        let progress = (100f64 * (iter_num+1) as f64 / iterations as f64) as isize;
                        println!("Progress: {}%", progress);
                    }

                    /////////////////////////////
                    // Generate a random field //
                    /////////////////////////////
                    let mut error_model: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata).unwrap();

                    for row in 0..rows {
                        for col in 0..columns {
                            error_model.set_value(row, col, rng.sample(StandardNormal)); //normal.ind_sample(&mut rng));
                        }
                    }

                    ////////////////////////////////////////
                    // Perform a FastAlmostGaussianFilter //
                    ////////////////////////////////////////
                    let n = 5;
                    let w_ideal = ((12f64 * sigma * sigma / n as f64 + 1f64)).sqrt();
                    let mut wl = w_ideal.floor() as isize;
                    if wl % 2 == 0 { wl -= 1; } // must be an odd integer
                    let wu = wl + 2;
                    let m = ((12f64 * sigma * sigma - (n * wl * wl) as f64 - (4 * n * wl) as f64 - (3 * n) as f64) / (-4 * wl - 4) as f64).round() as isize;

                    let mut integral: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata).unwrap();
                    let mut integral_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1).unwrap();

                    let mut val: f64;
                    let mut sum: f64;
                    let mut sum_n: i32;
                    let mut i_prev: f64;
                    let mut n_prev: i32;
                    let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
                    let mut num_cells: i32;
                    
                    for iteration_num in 0..n {
                        let midpoint = if iteration_num < m {
                            (wl as f64 / 2f64).floor() as isize
                        } else {
                            (wu as f64 / 2f64).floor() as isize
                        };

                        if iteration_num == 0 { // first iteration
                            // Create the integral images.
                            for row in 0..rows {
                                sum = 0f64;
                                sum_n = 0;
                                for col in 0..columns {
                                    val = error_model.get_value(row, col);
                                    if val == nodata {
                                        val = 0f64;
                                    } else {
                                        sum_n += 1;
                                    }
                                    sum += val;
                                    if row > 0 {
                                        i_prev = integral.get_value(row - 1, col);
                                        n_prev = integral_n.get_value(row - 1, col);
                                        integral.set_value(row, col, sum + i_prev);
                                        integral_n.set_value(row, col, sum_n + n_prev);
                                    } else {
                                        integral.set_value(row, col, sum);
                                        integral_n.set_value(row, col, sum_n);
                                    }
                                }
                            }
                        } else {
                            // Create the integral image based on previous iteration output. 
                            // We don't need to recalculate the num_cells integral image.
                            for row in 0..rows {
                                sum = 0f64;
                                for col in 0..columns {
                                    val = error_model.get_value(row, col);
                                    if val == nodata { val = 0f64; }
                                    sum += val;
                                    if row > 0 {
                                        i_prev = integral.get_value(row - 1, col);
                                        integral.set_value(row, col, sum + i_prev);
                                    } else {
                                        integral.set_value(row, col, sum);
                                    }
                                }
                            }
                        }

                        // Perform Filter
                        for row in 0..rows {
                            y1 = row - midpoint - 1;
                            if y1 < 0 { y1 = 0; }
                            y2 = row + midpoint;
                            if y2 >= rows { y2 = rows - 1; }

                            for col in 0..columns {
                                if input.get_value(row, col) != nodata {
                                    x1 = col - midpoint - 1;
                                    if x1 < 0 { x1 = 0; }
                                    x2 = col + midpoint;
                                    if x2 >= columns { x2 = columns - 1; }

                                    num_cells = integral_n[(y2, x2)] + integral_n[(y1, x1)] - integral_n[(y1, x2)] - integral_n[(y2, x1)];
                                    if num_cells > 0 {
                                        sum = integral[(y2, x2)] + integral[(y1, x1)] - integral[(y1, x2)] - integral[(y2, x1)];
                                        error_model.set_value(row, col, sum / num_cells as f64);
                                    } else {
                                        // should never hit here since input(row, col) != nodata above, therefore, num_cells >= 1
                                        error_model.set_value(row, col, 0f64);
                                    }
                                }
                            }
                        }
                    }

                    // Find the min and max values.
                    let mut min_value = f64::INFINITY;
                    let mut max_value = f64::NEG_INFINITY;
                    let mut z: f64; 
                    for row in 0..rows {
                        for col in 0..columns {
                            z = error_model[(row, col)];
                            if z < min_value { min_value = z; }
                            if z > max_value { max_value = z; }
                        }
                    }

                    ////////////////////////////////////////////
                    // Perform a histogram matching operation //
                    ////////////////////////////////////////////

                    let num_bins = ((max_value - min_value).max(1024f64)).ceil() as usize; 
                    let bin_size = (max_value - min_value) / num_bins as f64;
                    let mut histogram = vec![0f64; num_bins];
                    let num_bins_less_one = num_bins - 1;
                    let mut numcells: f64 = 0f64;
                    let mut bin_num;
                    for row in 0..rows {
                        for col in 0..columns {
                            z = error_model[(row, col)];
                            if z != nodata {
                                numcells += 1f64;
                                bin_num = ((z - min_value) / bin_size) as usize;
                                if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                                histogram[bin_num] += 1f64;
                            }
                        }
                    }

                    let mut cdf = vec![0f64; histogram.len()];
                    cdf[0] = histogram[0];
                    for i in 1..cdf.len() {
                        cdf[i] = cdf[i - 1] + histogram[i];
                    }
                    for i in 0..cdf.len() {
                        cdf[i] = cdf[i] / numcells;
                    }

                    let mut bin_num: usize;
                    let mut j: usize;
                    let mut x_val = 0f64;
                    let mut p_val: f64;
                    let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
                    for row in 0..rows {
                        for col in 0..columns {
                            z = error_model[(row, col)];
                            if z != nodata {
                                bin_num = ((z - min_value) / bin_size) as usize;
                                if bin_num > num_bins_less_one { bin_num = num_bins_less_one; }
                                p_val = cdf[bin_num];
                                j = ((p_val * 10f64).floor()) as usize;
                                for i in starting_vals[j]..num_lines {
                                    if reference_cdf[i][1] > p_val {
                                        if i > 0 {
                                            x1 = reference_cdf[i - 1][0];
                                            x2 = reference_cdf[i][0];
                                            p1 = reference_cdf[i - 1][1];
                                            p2 = reference_cdf[i][1];
                                            if p1 != p2 {
                                                x_val = x1 + ((x2 - x1) * ((p_val - p1) / (p2 - p1)));
                                            } else {
                                                x_val = x1;
                                            }
                                        } else {
                                            x_val = reference_cdf[i][0];
                                        }
                                        break;
                                    }
                                }
                                error_model.set_value(row, col, x_val);
                            }
                        }
                    }

                    /////////////////////////////////////
                    // Add the error model to the DEM. //
                    /////////////////////////////////////
                    let mut e: f64;
                    for row in 0..rows {
                        for col in 0..columns {
                            z = input[(row, col)];
                            if z != nodata {
                                e = error_model.get_value(row, col);
                                error_model.set_value(row, col, z + e);
                            } else {
                                error_model.set_value(row, col, nodata);
                            }
                        }
                    }

                    /////////////////////////////////////////////////
                    // Fill the depressions in the error-added DEM //
                    /////////////////////////////////////////////////
                    let background_val = (i32::min_value() + 1) as f64;
                    let mut dep_filled: Array2D<f64> = Array2D::new(rows, columns, background_val, nodata).unwrap();
                    
                    /*
                    Find the data edges. This is complicated by the fact that DEMs frequently
                    have nodata edges, whereby the DEM does not occupy the full extent of 
                    the raster. One approach to doing this would be simply to scan the
                    raster, looking for cells that neighbour nodata values. However, this
                    assumes that there are no interior nodata holes in the dataset. Instead,
                    the approach used here is to perform a region-growing operation, looking
                    for nodata values along the raster's edges.
                    */

                    let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity((rows * columns) as usize);
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

                    /* 
                    minheap is the priority queue. Note that I've tested using integer-based
                    priority values, by multiplying the elevations, but this didn't result
                    in a significant performance gain over the use of f64s.
                    */
                    let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
                    // let mut num_solved_cells = 0;
                    let mut zin_n: f64; // value of neighbour of row, col in input raster
                    let mut zout: f64; // value of row, col in output raster
                    let mut zout_n: f64; // value of neighbour of row, col in output raster
                    let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
                    let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
                    let (mut row, mut col): (isize, isize);
                    let (mut row_n, mut col_n): (isize, isize);
                    while !queue.is_empty() {
                        let cell = queue.pop_front().unwrap();
                        row = cell.0;
                        col = cell.1;
                        for n in 0..8 {
                            row_n = row + dy[n];
                            col_n = col + dx[n];
                            zin_n = error_model[(row_n, col_n)];
                            zout_n = dep_filled[(row_n, col_n)];
                            if zout_n == background_val {
                                if zin_n == nodata {
                                    dep_filled[(row_n, col_n)] = nodata;
                                    queue.push_back((row_n, col_n));
                                } else {
                                    dep_filled[(row_n, col_n)] = zin_n;
                                    // Push it onto the priority queue for the priority flood operation
                                    minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                                }
                            }
                        }
                    }

                    // Perform the priority flood operation.
                    while !minheap.is_empty() {
                        let cell = minheap.pop().unwrap();
                        row = cell.row;
                        col = cell.column;
                        zout = dep_filled[(row, col)];
                        for n in 0..8 {
                            row_n = row + dy[n];
                            col_n = col + dx[n];
                            zout_n = dep_filled[(row_n, col_n)];
                            if zout_n == background_val {
                                zin_n = error_model[(row_n, col_n)];
                                if zin_n != nodata {
                                    if zin_n < zout { zin_n = zout; } // We're in a depression. Raise the elevation.
                                    dep_filled[(row_n, col_n)] = zin_n;
                                    minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                                } else {
                                    // Interior nodata cells are still treated as nodata and are not filled.
                                    dep_filled[(row_n, col_n)] = nodata;
                                }
                            }
                        }
                    }

                    // Find the modified cells and increase their value in output.
                    for row in 0..rows {
                        for col in 0..columns {
                            if dep_filled[(row, col)] > error_model[(row, col)] {
                                out.increment(row, col, 1u16);
                            }
                        }
                    }
                }

                tx.send(out).unwrap();
            });
        }

        for n in 0..num_procs {
            let data = rx.recv().unwrap();
            if n < num_procs - 1 {
                for row in 0..rows {
                    for col in 0..columns {
                        output.increment(row, col, data.get_value(row, col) as f64);
                    }
                }
            } else {
                let mut z: f64;
                for row in 0..rows {
                    for col in 0..columns {
                        output.increment(row, col, data.get_value(row, col) as f64);
                        z = output.get_value(row, col);
                        output.set_value(row, col, z / iterations as f64);
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input base raster file: {}", input_file));
        output.add_metadata_entry(format!("RMSE: {}", rmse));
        output.add_metadata_entry(format!("Range: {}", range));
        output.add_metadata_entry(format!("Iterations: {}", iterations));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time) .replace("PT", ""));

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
            println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
struct GridCell {
    row: isize,
    column: isize,
    priority: f64,
}

impl Eq for GridCell {}

impl PartialOrd for GridCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.priority.partial_cmp(&self.priority)
    }
}

impl Ord for GridCell {
    fn cmp(&self, other: &GridCell) -> Ordering {
        let ord = self.partial_cmp(other).unwrap();
        match ord {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => ord,
        }
    }
}