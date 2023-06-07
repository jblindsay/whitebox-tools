/* 
Authors: Whitebox Geospatial Inc. (c)
Developer: Dr. John Lindsay
Created: 03/06/2023
Last Modified: 03/06/2023
License: Whitebox Geospatial Inc. License Agreement
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;
use num_cpus;

/// This tool uses [Ostu's method](https://en.wikipedia.org/wiki/Otsu%27s_method) for optimal automatic binary thresholding,
/// transforming an input image (`--input`) into background and foreground pixels (`--output`). Otsu’s method uses the grayscale 
/// image histogram to detect an optimal threshold value that separates two regions with maximum inter-class variance.
/// The process begins by calculating the image histogram of the input.
///
/// # References
/// Otsu, N., 1979. A threshold selection method from gray-level histograms. IEEE transactions on 
/// systems, man, and cybernetics, 9(1), pp.62-66.
///
/// # See Also
/// `ImageSegmentation`
fn main() {

    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}, 
            Err(e) => panic!("{:?}", e),
        }
    }
    
    if args.len() <= 1 || args[1].trim() == "help" {
        // print help
        help();
    }

    if args[1].trim() == "version" {
        // print version information
        version();
    }
    
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("otsu_thresholding{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    otsu_thresholding Help

    This tool performs a Canny edge-detection filter on an input image. 

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem     Name of the input DEM raster file.
    -o, --output  Name of the output raster file.
    --azimuth     Wind azimuth, in degrees.
    --max_dist    Optional maximum search distance. Minimum value is 5 x cell size.
    --z_factor    Optional multiplier for when the vertical and horizontal units are not the same.
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=input.tif -o=new.tif --sigma=0.25 --low=0.1 --high=0.2

    Note: Use of this tool requires a valid license. To obtain a license,
    contact Whitebox Geospatial Inc. (support@whiteboxgeo.com).
    "#
            .replace("*", &sep)
            .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "otsu_thresholding v{} by Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("OtsuThresholding")
}

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    let tool_name = get_tool_name();

    let sep: String = path::MAIN_SEPARATOR.to_string();

    // Read in the environment variables and get the necessary values
    let configurations = whitebox_common::configs::get_configs()?;
    let mut working_directory = configurations.working_directory.clone();
    if !working_directory.is_empty() && !working_directory.ends_with(&sep) {
        working_directory += &sep;
    }
    let max_procs = configurations.max_procs as isize;

    // read the arguments
    let mut input_file: String = String::new();
    let mut output_file: String = String::new();
    // let mut num_bins = 256usize;

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
        if flag_val == "-i" || flag_val == "-input" {
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
        // } else if flag_val == "-num_bins" {
        //     num_bins = if keyval {
        //         vec[1]
        //             .to_string()
        //             .parse::<usize>()
        //             .expect(&format!("Error parsing {}", flag_val))
        //     } else {
        //         args[i + 1]
        //             .to_string()
        //             .parse::<usize>()
        //             .expect(&format!("Error parsing {}", flag_val))
        //     };
        }
    }

    if configurations.verbose_mode {
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
    let input = Raster::new(&input_file, "r")?;
    let configs = input.configs.clone();
    let rows = configs.rows as isize;
    let columns = configs.columns as isize;
    let nodata = configs.nodata;

    let input = Arc::new(input);

    /////////////////////////////
    // Calculate the histogram //
    /////////////////////////////
    
    let is_rgb_image = if input.configs.data_type == DataType::RGB24
        || input.configs.data_type == DataType::RGBA32
        || input.configs.photometric_interp == PhotometricInterpretation::RGB
    {
        true
    } else {
        false
    };
    

    if configurations.verbose_mode {
        println!("Calculating histogram...");
    }



    // create the histogram
    let mut num_bins = 1024usize;
    let min_value: f64;
    let range: f64;
    let bin_size: f64;

    if !is_rgb_image {
        min_value = input.configs.minimum;
        range = input.configs.maximum - min_value;
        if range.round() as usize > num_bins {
            num_bins = range.round() as usize;
        }
        bin_size = range / (num_bins - 1) as f64;
    } else {
        min_value = 0f64;
        range = 1f64;
        bin_size = range / (num_bins - 1) as f64;
    }

    let mut num_procs = num_cpus::get() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }
    
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut histo = vec![0; num_bins];
            let mut z: f64;
            let mut bin: usize;

            let input_fn: Box<dyn Fn(isize, isize) -> usize> = if !is_rgb_image {
                Box::new(|row: isize, col: isize| -> usize {
                    let x = input.get_value(row, col);
                    ((x - min_value) / bin_size).floor() as usize
                })
            } else {
                Box::new(|row: isize, col: isize| -> usize {
                    let value = input.get_value(row, col);
                    let x = value2i(value);
                    ((x - min_value) / bin_size).floor() as usize
                })
            };

            for row in (0..rows).filter(|r| r % num_procs == tid) {
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata {
                        bin = input_fn(row, col);
                        histo[bin] += 1;
                    }
                }
            }

            tx.send(histo).unwrap();
        });
    }

    let mut histo = vec![0; num_bins];
    let mut total_cells = 0;
    let mut procs_completed = 0;
    for _ in 0..num_procs {
        let proc_histo = rx.recv().expect("Error receiving data from thread.");
        for bin in 0..num_bins {
            histo[bin] += proc_histo[bin];
            total_cells += proc_histo[bin];
        }

        if configurations.verbose_mode {
            procs_completed += 1;
            progress = (100.0_f64 * procs_completed as f64 / num_procs as f64) as usize;
            if progress != old_progress {
                println!("Calculating histogram: {}%", progress);
                old_progress = progress;
            }
        }
    }

    


    let mut cumulative_histo = Vec::with_capacity(num_bins);
    for bin in 0..num_bins {
        if bin > 0 {
            cumulative_histo.push(histo[bin] + cumulative_histo[bin - 1]);
        } else {
            cumulative_histo.push(histo[bin]);
        }
    }
    let cdf: Vec<f64> = cumulative_histo.into_iter().map(|x| x as f64 / total_cells as f64).collect();
    
    let (mut w0, mut w1): (f64, f64);
    let (mut m0, mut m1): (f64, f64);
    let mut var: f64;
    let mut max_var = 0f64;
    let mut max_i = 0;
    for bin in 0..num_bins-1 {
        w0 = cdf[bin];
        w1 = 1.0 - w0;
        m0 = (0..=bin).into_iter().map(|i| i * histo[i]).sum::<usize>() as f64 / (w0 * total_cells as f64);
        m1 = (bin+1..num_bins).into_iter().map(|i| i * histo[i]).sum::<usize>() as f64 / (w1 * total_cells as f64);
        var = w0 * w1 * (m0 - m1).powi(2);
        if var > max_var { 
            max_var = var; 
            max_i = bin;
        }
    }
    
    if configurations.verbose_mode {
        println!("Max variance: {:.2?}\nGreytone bin of max variance: {max_i} of {num_bins} bins", max_var);
    }
    
    let out_nodata = -32768.0;
    
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut z: f64;
            let mut bin: usize;

            let input_fn: Box<dyn Fn(isize, isize) -> usize> = if !is_rgb_image {
                Box::new(|row: isize, col: isize| -> usize {
                    let x = input.get_value(row, col);
                    ((x - min_value) / bin_size).floor() as usize
                })
            } else {
                Box::new(|row: isize, col: isize| -> usize {
                    let value = input.get_value(row, col);
                    let x = value2i(value);
                    ((x - min_value) / bin_size).floor() as usize
                })
            };

            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data = vec![out_nodata; columns as usize];
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata {
                        bin = input_fn(row, col);
                        if bin <= max_i {
                            data[col as usize] = 0.0;
                        } else {
                            data[col as usize] = 1.0;
                        }
                    }
                }
                tx.send((row, data)).unwrap();
            }
        });
    }

    let mut out_configs = input.configs.clone();
    out_configs.data_type = DataType::I16;
    out_configs.nodata = out_nodata;
    out_configs.photometric_interp = PhotometricInterpretation::Categorical;
    let mut output = Raster::initialize_using_config(&output_file, &out_configs);
    
    for r in 0..rows {
        let (row, data) = rx.recv().expect("Error receiving data from thread.");
        output.set_row_data(row, data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Calculating index: {}%", progress);
                old_progress = progress;
            }
        }
    }

    drop(input);

    
    //////////////////////
    // Output the image //
    //////////////////////

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "{}",
            &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
        );
    }

    if configurations.verbose_mode {
        println!("Saving data...")
    };
    let _ = match output.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output file written")
            }
        }
        Err(e) => return Err(e),
    };

    Ok(())
}

fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255f64;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

    (r + g + b) / 3f64
}
