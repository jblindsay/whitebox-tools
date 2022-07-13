/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 14/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use rand::prelude::*;
// use rand::{Rng, SeedableRng};
use rand::thread_rng;
use rand_distr::StandardNormal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to create a random field using the turning bands algorithm. The user must specify
/// the name of a base raster image (`--base`) from which the output raster will derive its geographical
/// information, dimensions (rows and columns), and other information. In addition, the range (`--range`), in
/// x-y units, must be specified. The range determines the correlation length of the resulting field. For a
/// good description of how the algorithm works, see Carr (2002). The turning bands method creates a number
/// of 1-D simulations (called bands) and fuses these together to create a 2-D error field. There is no
/// natural stopping condition in this process, so the user must specify the number of bands to create
/// (`--iterations`). The default value of 1000 iterations is reasonable. The fewer iterations used, the
/// more prevalent the 1-D simulations will be in the output error image, effectively creating artifacts.
/// Run time increases with the number of iterations.
///
/// Turning bands simulation is a commonly applied technique in Monte Carlo style simulations of uncertainty.
/// As such, it is frequently run many times during a simulation (often 1000s of times). When this is the
/// case, algorithm performance and efficiency are key considerations. One alternative method to efficiently
/// generate spatially autcorrelated random fields is to apply the `FastAlmostGaussianFilter` tool to the
/// output of the `RandomField` tool. This can be used to generate a random field with the desired spatial
/// characteristics and frequency distribution. This is the alternative approach used by the
/// `StochasticDepressionAnalysis` tool.
///
/// # Reference
/// Carr, J. R. (2002). Data visualization in the geosciences. Upper Saddle River, NJ: Prentice Hall. pp. 267.
///
/// # See Also
/// `RandomField`, `FastAlmostGaussianFilter`, `StochasticDepressionAnalysis`
pub struct TurningBandsSimulation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TurningBandsSimulation {
    pub fn new() -> TurningBandsSimulation {
        // public constructor
        let name = "TurningBandsSimulation".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Creates an image containing random values based on a turning-bands simulation."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["-i".to_owned(), "--base".to_owned()],
            description: "Input base raster file.".to_owned(),
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

        parameters.push(ToolParameter {
            name: "Range of Autocorrelation (map units)".to_owned(),
            flags: vec!["--range".to_owned()],
            description:
                "The field's range, in xy-units, related to the extent of spatial autocorrelation."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Iterations".to_owned(),
            flags: vec!["--iterations".to_owned()],
            description: "The number of iterations.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1000".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=in.tif -o=out.tif --range=850.0 --iterations=2500", short_exe, name).replace("*", &sep);

        TurningBandsSimulation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TurningBandsSimulation {
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
        let mut range = 1f64;
        let mut iterations = 1000;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-base" {
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
            } else if flag_val == "-range" {
                if keyval {
                    range = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    range = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-iterations" {
                if keyval {
                    iterations = vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
                } else {
                    iterations = args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                        as usize;
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        // let nodata = input.configs.nodata;

        let diagonal_size =
            (rows as f64 * rows as f64 + columns as f64 * columns as f64).sqrt() as usize;
        let filter_half_size = (range / (2f64 * input.configs.resolution_x as f64)) as usize;
        let filter_size = filter_half_size * 2 + 1;
        let mut cell_offsets = vec![0isize; filter_size];
        for i in 0..filter_size as isize {
            cell_offsets[i as usize] = i - filter_half_size as isize;
        }

        let w = (36f64 / (filter_half_size * (filter_half_size + 1) * filter_size) as f64).sqrt();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(0.0);

        let mut rng = thread_rng();
        let mut rng2 = thread_rng();
        // let normal = Normal::new(0.0, 1.0);
        // let between = Range::new(0, 4);
        // let between_rows = Range::new(0f64, rows as f64);
        // let between_cols = Range::new(0f64, columns as f64);
        let mut z: f64;
        let (mut pnt1x, mut pnt1y, mut pnt2x, mut pnt2y): (f64, f64, f64, f64);

        // loop through the number of iterations
        for i in 0..iterations {
            // create the data line and fill it with random numbers.
            // notice that the initial dataline is 2 * filterHalfSize larger
            // because of the edge effects of the filter.
            let mut t = vec![0f64; diagonal_size + 2 * filter_half_size];
            for j in 0..diagonal_size {
                t[j] = rng.sample(StandardNormal); // normal.ind_sample(&mut rng);
            }

            let mut y = vec![0f64; diagonal_size];

            // filter the line
            let mut m: isize;
            let mut sum = 0.0;
            let mut sq_sum = 0.0;
            for j in 0..diagonal_size {
                z = 0f64;
                for k in 0..filter_size {
                    m = cell_offsets[k];
                    z += m as f64 * t[(j as isize + filter_half_size as isize + m) as usize];
                }
                y[j] = w * z;
                sum += y[j];
                sq_sum += y[j] * y[j];
            }

            // Standardize the line
            let mean = sum / diagonal_size as f64;
            let stdev = (sq_sum / diagonal_size as f64 - mean * mean).sqrt();
            for j in 0..diagonal_size {
                y[j] = (y[j] - mean) / stdev;
            }

            // assign the spatially autocorrelated data line an equation of a transect of the grid
            // first, pick two points on different edges of the grid at random.
            // Edges are as follows 0 = left, 1 = top, 2 = right, and 3 = bottom
            let edge1 = rng.gen_range(0, 4); //between.ind_sample(&mut rng);
            let mut edge2 = edge1;
            while edge2 == edge1 {
                edge2 = rng.gen_range(0, 4); //between.ind_sample(&mut rng);
            }

            match edge1 {
                0 => {
                    pnt1x = 0f64;
                    pnt1y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                }
                1 => {
                    pnt1x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                    pnt1y = 0f64;
                }
                2 => {
                    pnt1x = (columns - 1) as f64;
                    pnt1y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                }
                _ => {
                    // 3
                    pnt1x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                    pnt1y = (rows - 1) as f64;
                }
            }

            match edge2 {
                0 => {
                    pnt2x = 0f64;
                    pnt2y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                }
                1 => {
                    pnt2x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                    pnt2y = 0f64;
                }
                2 => {
                    pnt2x = (columns - 1) as f64;
                    pnt2y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                }
                _ => {
                    // 3
                    pnt2x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                    pnt2y = (rows - 1) as f64;
                }
            }

            if pnt1x == pnt2x || pnt1y == pnt2y {
                while pnt1x == pnt2x || pnt1y == pnt2y {
                    match edge2 {
                        0 => {
                            pnt2x = 0f64;
                            pnt2y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                        }
                        1 => {
                            pnt2x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                            pnt2y = 0f64;
                        }
                        2 => {
                            pnt2x = (columns - 1) as f64;
                            pnt2y = rng2.gen_range(0, rows as isize) as f64; //between_rows.ind_sample(&mut rng);
                        }
                        _ => {
                            // 3
                            pnt2x = rng2.gen_range(0, columns as isize) as f64; //between_cols.ind_sample(&mut rng);
                            pnt2y = (rows - 1) as f64;
                        }
                    }
                }
            }

            let line_slope = (pnt2y - pnt1y) / (pnt2x - pnt1x);
            let line_intercept = pnt1y - line_slope * pnt1x;
            let perpendicular_line_slope = -1f64 / line_slope;
            let slope_diff = line_slope - perpendicular_line_slope;
            let mut perpendicular_line_intercept: f64;
            let (mut row, mut col): (usize, usize);

            // for each of the four corners, figure out what the perpendicular line
            // intersection coordinates would be.

            // point (0,0)
            perpendicular_line_intercept = 0f64;
            let corner1x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner1y = line_slope * corner1x - line_intercept;

            // point (0,cols)
            row = 0;
            col = columns as usize;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner2x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner2y = line_slope * corner2x - line_intercept;

            // point (rows,0)
            row = rows as usize;
            col = 0;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner3x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner3y = line_slope * corner3x - line_intercept;

            // point (rows,cols)
            row = rows as usize;
            col = columns as usize;
            perpendicular_line_intercept = row as f64 - perpendicular_line_slope * col as f64;
            let corner4x = (perpendicular_line_intercept - line_intercept) / slope_diff;
            let corner4y = line_slope * corner4x - line_intercept;

            // find the point with the minimum Y value and set it as the line starting point
            let mut line_start_x = corner1x;
            let mut line_start_y = corner1y;
            if corner2y < line_start_y {
                line_start_x = corner2x;
                line_start_y = corner2y;
            }
            if corner3y < line_start_y {
                line_start_x = corner3x;
                line_start_y = corner3y;
            }
            if corner4y < line_start_y {
                line_start_x = corner4x;
                line_start_y = corner4y;
            }

            // scan through each grid cell and assign it the closest value on the line segment
            let mut num_procs = num_cpus::get() as isize;
            let configs = whitebox_common::configs::get_configs()?;
            let max_procs = configs.max_procs;
            if max_procs > 0 && max_procs < num_procs {
                num_procs = max_procs;
            }
            let (tx, rx) = mpsc::channel();
            let y = Arc::new(y);
            for tid in 0..num_procs {
                let y = y.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut perpendicular_line_intercept: f64;
                        let (mut intersecting_point_x, mut intersecting_point_y): (f64, f64);
                        let mut data = vec![0f64; columns as usize];
                        for col in 0..columns {
                            perpendicular_line_intercept =
                                row as f64 - perpendicular_line_slope * col as f64;
                            intersecting_point_x =
                                (perpendicular_line_intercept - line_intercept) / slope_diff;
                            intersecting_point_y =
                                line_slope * intersecting_point_x - line_intercept;
                            let mut p = (((intersecting_point_x - line_start_x)
                                * (intersecting_point_x - line_start_x)
                                + (intersecting_point_y - line_start_y)
                                    * (intersecting_point_y - line_start_y))
                                .sqrt()) as isize;
                            if p < 0 {
                                p = 0;
                            }
                            if p > (diagonal_size - 1) as isize {
                                p = (diagonal_size - 1) as isize;
                            }
                            data[col as usize] = y[p as usize];
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for _ in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                output.increment_row_data(row, data);
            }

            if verbose {
                progress = (100.0_f64 * i as f64 / (iterations - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let iterations_rooted = (iterations as f64).sqrt(); // * 3.5;
        for row in 0..rows {
            for col in 0..columns {
                // output[(row, col)] = (output[(row, col)] - mean) / stdev; // / iterations as f64;
                output[(row, col)] = output[(row, col)] / iterations_rooted;
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input base raster file: {}", input_file));
        output.add_metadata_entry(format!("Range: {}", range));
        output.add_metadata_entry(format!("Iterations: {}", iterations));
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
