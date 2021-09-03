/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/07/2017
Last Modified: 13/10/2018
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

/// This tool outputs distribution summary statistics for input raster images (`--input`).
/// The distribution statistics include the raster minimum, maximum, range, total, mean,
/// variance, and standard deviation. These summary statistics are output to the system `stdout`.
///
/// The following is an example of the summary report:
///
/// > \********************************* <br/>
/// > \* Welcome to RasterSummaryStats * <br/>
/// > \********************************* <br/>
/// > Reading data...
/// >
/// > Number of non-nodata grid cells: 32083559 <br/>
/// > Number of nodata grid cells: 3916441 <br/>
/// > Image minimum: 390.266357421875 <br/>
/// > Image maximum: 426.0322570800781 <br/>
/// > Image range: 35.765899658203125 <br/>
/// > Image total: 13030334843.332886 <br/>
/// > Image average: 406.13745012929786 <br/>
/// > Image variance: 31.370027239143383 <br/>
/// > Image standard deviation: 5.600895217654351 <br/>
///
/// # See Also
/// `RasterHistogram`, `ZonalStatistics`
pub struct RasterSummaryStats {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterSummaryStats {
    pub fn new() -> RasterSummaryStats {
        // public constructor
        let name = "RasterSummaryStats".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Measures a rasters min, max, average, standard deviation, num. non-nodata cells, and total."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=DEM.tif",
            short_exe, name
        )
        .replace("*", &sep);

        RasterSummaryStats {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterSummaryStats {
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
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

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        //if verbose { println!("Calculating image mean and standard deviation...") };
        //let (mean, stdev) = input.calculate_mean_and_stdev();

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
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut n = 0;
                    let mut s = 0.0;
                    let mut sq = 0.0;
                    let mut minz = f64::INFINITY;
                    let mut maxz = f64::NEG_INFINITY;
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            n += 1;
                            s += z;
                            sq += z * z;
                            if z < minz {
                                minz = z;
                            }
                            if z > maxz {
                                maxz = z;
                            }
                        }
                    }
                    tx.send((n, s, sq, minz, maxz)).unwrap();
                }
            });
        }

        let mut num_cells = 0;
        let mut sum = 0.0;
        let mut sq_sum = 0.0;
        let mut minz = f64::INFINITY;
        let mut maxz = f64::NEG_INFINITY;
        for row in 0..rows {
            let (a, b, c, d, e) = rx.recv().expect("Error receiving data from thread.");
            num_cells += a;
            sum += b;
            sq_sum += c;
            if d < minz {
                minz = d;
            }
            if e > maxz {
                maxz = e;
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mean = sum / num_cells as f64;
        let variance = sq_sum / num_cells as f64 - mean * mean;
        let std_dev = variance.sqrt();

        let elapsed_time = get_formatted_elapsed_time(start);

        println!("\nNumber of non-nodata grid cells: {}", num_cells);
        println!(
            "Number of nodata grid cells: {}",
            input.num_cells() - num_cells
        );
        println!("Image minimum: {}", minz);
        println!("Image maximum: {}", maxz);
        println!("Image range: {}", maxz - minz);
        println!("Image total: {}", sum);
        println!("Image average: {}", mean);
        println!("Image variance: {}", variance);
        println!("Image standard deviation: {}", std_dev);
        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
