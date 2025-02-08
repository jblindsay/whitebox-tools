/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: 17/08/2021
Last Modified: 31/05/2022
License: MIT
*/

use whitebox_raster::*;
use whitebox_vector::*;
use whitebox_common::utils::get_formatted_elapsed_time;
use num_cpus;
use std::time::Instant;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::collections::HashMap;
use std::cmp::min;
use rand::prelude::*;

/// This tool creates a new shapefile output (`--output`) sample sites that satisfy a latin hypercube based
/// on a set of input rasters with the same projections (`--inputs`), and is therefore a multidimensional stratified random
/// sampling scheme. A random subset of samples (`--samples`, n << N) is chosen from the population and iteratively resampled
/// (`--max_iter`) to minimize an objective function. An annealing schedule and a random resample probability
/// (`--rs_prob`) are used to control how likely a interaction is to randomly resample, or resample the worst
/// strata, where higher values favour a more random sample, and lower values favour a more stratified sample.
///
/// The annealing process controls the probability that samples will be discarded each iteration.
/// The temperature is decreased over several iterations, decreasing the probability of discarding
/// the new sample. The properties of the annealing process can be manipulated through the
/// parameters `--temp` for initial temperature `--temp_decay` for the temperature decay rate,
/// and  `-cycle` to determine the number of iterations before re-applying the decay rate.
///
/// This implementation is loosely based on Minasny and McBratney (2006). An additional optional
/// parameter `--average` has been added to normalize the continuous objective function and bring
/// it closer to the range of values for the categorical and correlation objective functions. This
/// prevents continuous inputs from dominating the objective function and makes the objective function
/// cutoff threshold (`--threshold`) more predictable. However, as a result, the algorithm will emphasize
/// the categorical and correlation objective relative to the standard weighting.
///
/// data objective function has been used to average the number of strata so that it does not dominate the
/// objective function, and makes a objective function cutoff value more predictable (`--o_thresh`).
/// Another departure from the original is that a lower objective function forces the sample to be retained
/// instead of rejected. This was originally based on a random number and the annealed changed in objective function.
///
/// # Reference
/// Minsasny, B., and McBratney, A. B. (2006). A conditioned Latin hypercube method for sampling in the
/// presence of ancillary information. Computers and Geosciences, 32, 1378-1388.
///
/// # See Also
/// `random_sample`

fn main() {
    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}
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

    let exe_name = &format!("clhs{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    ConditionedLatinHypercube Help

    The ConditionedLatinHypercube tool is used to identify sample sites base on stratified random sampling of 
    ancillary information.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input     Name of the input raster file.
    -o, --output    Name of the output shapefile.
    --samples       Number of sites to select.
    --iterations    Maximum number of resampling iterations.
    --seed          Seed the random number generator for consisent results.
    --prob          Probability of resampling randomly vs. the worst strata [0,1].
    --threshhold    Objective function threshold below which reamping is stopped.
    --temp          Initial annealing temperature between [0,1].
    --temp_decay    Annealing temperature decay rate between [0,1].
    --cycle         Annealing cycle length in iterations.
    --average       Weight the continuous objective funtion by the 1/N contributing strata.

    Input/output file names can be fully qualified, or can rely on the working directory contained in
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=Raster1.tif;Raster2.tif --output=sites.shp --samples=500
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "cLHS v{} by Dan Newman (c) 2022.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("ConditionedLatinHypercube") // This should be camel case and is a reference to the tool name.
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
    let verbose = configurations.verbose_mode;

    let mut input_files_str = String::new();
    let mut output_file = String::new();
    let mut num_samples = 500isize;
    let mut max_iter = 25000isize;
    let mut temp = 1f64;
    let mut temp_decay = 0.05f64;
    let mut anneal_cycle = 10isize;
    let mut rs_prob = 0.5f64;
    let mut o_thresh = f64::MIN;
    let mut rng_seed = -1isize;
    let mut norm_o1 = false;
    let mut weights = [1f64, 1f64, 1f64]; // add weights arguments later

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
        if flag_val == "-i" || flag_val == "-inputs" {
            input_files_str = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-o" || flag_val == "-output" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i + 1].to_string();
            }
        } else if flag_val == "-samples" {
            if keyval {
                num_samples = vec[1].to_string().parse::<isize>().unwrap();
            } else {
                num_samples = args[i + 1].to_string().parse::<isize>().unwrap();
            }
        } else if flag_val == "-iterations" {
            if keyval {
                max_iter = vec[1].to_string().parse::<isize>().unwrap();
            } else {
                max_iter = args[i + 1].to_string().parse::<isize>().unwrap();
            }
        } else if flag_val == "-threshold" {
            if keyval {
                o_thresh = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                o_thresh = args[i + 1].to_string().parse::<f64>().unwrap();
            }
        } else if flag_val == "-temp" {
            if keyval {
                temp = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                temp = args[i + 1].to_string().parse::<f64>().unwrap();
            }
        } else if flag_val == "-temp_decay" {
            if keyval {
                temp_decay = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                temp_decay = args[i + 1].to_string().parse::<f64>().unwrap();
            }
        } else if flag_val == "-cycle" {
            if keyval {
                anneal_cycle = vec[1].to_string().parse::<isize>().unwrap();
            } else {
                anneal_cycle = args[i + 1].to_string().parse::<isize>().unwrap();
            }
        } else if flag_val == "-prob" {
            if keyval {
                rs_prob = vec[1].to_string().parse::<f64>().unwrap();
            } else {
                rs_prob = args[i + 1].to_string().parse::<f64>().unwrap();
            }
        } else if flag_val == "-seed" {
            if keyval {
                rng_seed = vec[1].to_string().parse::<isize>().unwrap().abs();
            } else {
                rng_seed = args[i + 1].to_string().parse::<isize>().unwrap().abs();
            }
        } else if flag_val == "-average" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                norm_o1 = true;
            }
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

    let num_samples = num_samples.abs() as usize;
    if num_samples == 0 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Number of samples must be > 0.", ));
    }
    let anneal_cycle = anneal_cycle.abs() as usize;
    if anneal_cycle == 0 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Anneal cycle length must be > 0.", ));
    }
    let max_iter = max_iter.abs() as usize;
    if max_iter == 0 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Max iterations must be > 0.", ));
    }
    if temp < 0f64 || temp > 1f64 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Initial temperature must be between [0,1].", ));
    }
    if temp_decay < 0f64 || temp_decay > 1f64 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Temperature decay rate must be between [0,1].", ));
    }
    let temp_decay = 1f64 - temp_decay;
    if rs_prob < 0f64 || rs_prob > 1f64 {
        return Err(Error::new(ErrorKind::InvalidInput,
            "Resample probability must be between [0,1].", ));
    }

    let mut cmd = input_files_str.split(";");
    let mut input_files = cmd.collect::<Vec<&str>>();
    if input_files.len() == 1 {
        cmd = input_files_str.split(",");
        input_files = cmd.collect::<Vec<&str>>();
    }

    let wd = if working_directory.is_empty() {
        // set the working directory to that of the first input file.
        let p = path::Path::new(input_files[0].trim());
        // let wd = p.parent().unwrap().to_str().unwrap().to_owned();
        format!(
            "{}{}",
            p.parent().unwrap().to_str().unwrap().to_owned(),
            sep
        )
    } else {
        working_directory.clone().to_owned()
    };

    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", wd, output_file);
    }

    if !output_file.ends_with(".shp") {
        output_file.push_str(".shp");
    }

    let num_files = input_files.len();

    let seed: u64 = if rng_seed < 0 { // no seed specified, random seed
        thread_rng().gen_range(u64::MIN, u64::MAX)
    } else { rng_seed as u64 };
    let mut rng : StdRng = StdRng::seed_from_u64(seed);

    //let num_samp_input = num_samples;
    if num_samples < (25*num_files) {
        println!("Warning: Too few samples to calculate sample correlation matrices.");
        println!("Increase number of samples to at least 25 per input for better results");
        //println!("Setting the sample size to {:?} (i.e., 25 * the number of inputs).", 25*num_files);
        //println!("{:?} random samples will be drawn from the larger sample pool.", num_samp_input);
        //num_samples = 25*num_files;
    }

    let mut num_procs = num_cpus::get() as isize;
    let configs = whitebox_common::configs::get_configs()?;
    let max_procs = configs.max_procs;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    let start = Instant::now();

    let mut strata = vec![0f64; num_samples];
    for i in 0..num_samples {
        strata[i] = (i+1) as f64 / num_samples as f64;
    }

    let mut totals = vec![0f64; num_files];
    let mut averages = vec![0f64; num_files];
    let mut num_cells = vec![0f64; num_files];
    let mut k_is_cont = vec![true; num_files]; // true for continuous, false for categorical

    let num_bins = 25000isize;
    let mut quantiles = vec![vec![]; num_files];

    if verbose {
        println!("Estimating strata and collecting initial samples...");
    }
    // Precalculate stats for later and collect samples
    let k_pool: Vec<f64> = (0..num_files).map(|_| rng.gen()).collect();
    let k_pool_sum = k_pool.iter().fold(0f64, |a,v| a+v);
    let k_pool: Vec<usize> = k_pool.into_iter()
                .map(|k| ((k / k_pool_sum) * max_iter as f64)
                .ceil() as usize).collect();
    let reservoir_size = (num_files * num_samples) +
                k_pool.iter().fold(0, |a,v| a+v);
    let mut reservoir: Vec<Sample> = Vec::with_capacity(reservoir_size);
    let mut projection: String = String::new();
    // currently, each input is treated independently
    // if all inputs are to have the same structure (i.e., row, col, x, y)
    // then the HashSet can be used to store all unique Nodata indices for all inputs
    //let mut nodata_indices = HashSet::new();
    let (mut rows, mut columns): (isize, isize);

    for k in 0..num_files {
        let input = Raster::new(&input_files[k].trim(), "r")?;
        let nodata = input.configs.nodata;
        let minval = input.configs.minimum;
        let maxval = input.configs.maximum;
        //if k == 0 {
        rows = input.configs.rows as isize;
        columns = input.configs.columns as isize;
        //} else if input.configs.rows as isize != rows
        //        || input.configs.columns != columns {
        //            return Err(Error::new(ErrorKind::InvalidInput,
        //                "All input images must share the same dimensions (rows and columns) and spatial extent."));
        //}
        let valrange = (maxval - minval).ceil();
        let binsize = valrange / num_bins as f64;
        let is_cont = input.configs.data_type.is_float();
        if input.configs.coordinate_ref_system_wkt.len() > projection.len(){
            projection = input.configs.coordinate_ref_system_wkt.clone();
        }
        k_is_cont[k] = is_cont;

        // NOTE: parallelizing this step tends to be slower overall, and requires more memory
        // serial version commented out below for now
        let input = Arc::new(input);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut bin: isize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut histo = vec![0f64; num_bins as usize];
                    let mut data = vec![nodata; columns as usize];
                    let (mut tot, mut count) = (0f64, 0f64);
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            tot += z;
                            count += 1f64;
                            data[col as usize] = z;
                            if is_cont {
                                // is continuous data
                                bin = ((z - minval) / binsize).floor() as isize;
                                if bin >= num_bins {
                                    bin = num_bins - 1;
                                }
                                histo[bin as usize] += 1f64;
                            }
                        }
                    }
                    tx.send((row, tot, count, histo, data)).unwrap();
                }
            });
        }

        let mut histogram = vec![0f64; num_bins as usize];
        let mut class_histo = HashMap::new();
        // use valid_indices to hold index if not nodata, else nodata
        let mut valid_indices = vec![nodata; (rows*columns) as usize];
        let (mut idx, mut val): (usize, f64);
        for _ in 0..rows {
            let (row, tot, count, histo, data) = rx.recv()
                .expect("Error receiving data from thread.");
            totals[k] += tot;
            num_cells[k] += count;
            for c in 0..columns {
                val = data[c as usize];
                if val != nodata {
                    idx = rc2i(row, c, columns);
                    valid_indices[idx] = idx as f64;
                    // because data holds cell values, it can also be used to hash classes
                    if !is_cont {
                        // is categorical data, where val is the class key
                        let counter = class_histo.entry(val as isize)
                                .or_insert(0);
                        *counter += 1; // increment hashmap value
                    }
                }
            }
            if is_cont {
                for b in 0..num_bins as usize {
                    histogram[b] += histo[b]
                }
            }
        }
        averages[k] = totals[k] / num_cells[k];
        // remove all the nodata "indices" so that only valid indices remain
        let mut valid_indices: Vec<usize> =
                    valid_indices.into_iter()
                    .filter(|i| *i != nodata)
                    .map(|i| i as usize).collect();

/*
        let mut z: f64;
        let mut bin: isize;
        let (mut total, mut count) = (0f64, 0f64);
        let mut histogram = vec![0f64; num_bins as usize];
        let mut class_histo = HashMap::new();
        let mut valid_indices: Vec<usize> = Vec::with_capacity((rows * columns) as usize);
        for row in 0..rows { // don't thread, it randomizes the order when pushing into valid_indices
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata {
                    total += z;
                    count += 1f64;
                    valid_indices.push(rc2i(row, col, columns));
                    if is_cont {
                        // is continuous data
                        bin = ((z - minval) / binsize).floor() as isize;
                        if bin >= num_bins {
                            bin = num_bins - 1;
                        }
                        histogram[bin as usize] += 1f64;
                    } else {
                        // is categorical data, where z is the class key
                        let counter = class_histo.entry(z as isize)
                                .or_insert(0);
                        *counter += 1; // increment hashmap value
                    }
                } //else {
                            //nodata_indices.insert(rc2i(row, col, columns));
                        //}
            }
        }
        totals[k] += total;
        num_cells[k] += count;
        averages[k] = total / count;
*/
        // if continuous, use histogram to estimate quantiles
        if is_cont {
            let mut cdf = vec![0f64; num_bins as usize];
            cdf[0] = histogram[0];
            for i in 1..num_bins as usize {
                cdf[i] = cdf[i - 1] + histogram[i];
            }
            for i in 0..num_bins as usize {
                cdf[i] = cdf[i] / num_cells[k] as f64;
            }

            // estimate quantile from cdf. faster than sorting?
            quantiles[k] = vec![0f64; num_samples];
            let mut bin: usize;
            for s in 0..num_samples {
                bin = 0usize;
                for b in 0..num_bins as usize {
                    if cdf[b] <= strata[s] {
                        bin = b; // exceeded stata bound. best estimate is previous bin.
                    } else {
                        break;
                    }
                }
                quantiles[k][s] = minval + (bin as f64 * binsize)
            }
        } else {
            // categorical data
            // use quantiles[k] to hold unique classes for now
            quantiles[k] = class_histo.clone().into_keys()
                            .map(|c| c as f64).collect::<Vec<f64>>();
        }

        // collect samples data from input k
        let (mut zs, mut qs): (f64, usize);
        let (mut rs, mut cs): (isize, isize);
        valid_indices.shuffle(&mut rng);
        let mut repeater = 0usize;
        for _ in 0..(num_samples + k_pool[k]) {
            // dont need to sample all, only enough for initial samples and all iterations
            (rs, cs) = i2rc(valid_indices[repeater], columns);
            zs = input.get_value(rs,cs);
            if is_cont { // get strata index
                qs = quantiles[k].iter().position(|q| zs <= *q).unwrap();
                //assert_eq!(zs <= quantiles[k][qs], true);
            } else { // get class index
                qs = quantiles[k].iter().position(|c| zs == *c).unwrap();
                //assert_eq!(zs, quantiles[k][qs]);
            }

            reservoir.push(
                Sample {
                    k:k,
                    x:input.get_x_from_column(cs),
                    y:input.get_y_from_row(rs),
                    q:qs, v:zs
                }
            );

            repeater += 1;
            if repeater == valid_indices.len() { // next iteration will panic
                // not enough samples, draw from a new shuffled deck
                valid_indices = valid_indices.clone();
                valid_indices.shuffle(&mut rng);
                repeater = 0;
            }
        }

        if !is_cont {
            // convert class values into histogram counts
            // divide each count to get proportion Kappa of class j for input k
            quantiles[k] = quantiles[k].iter()
                            .map(|j| *class_histo.get(&(*j as isize)).unwrap() as f64)
                            .map(|c| c / num_cells[k]).collect::<Vec<f64>>();
            //assert_eq!(quantiles[k].iter().fold(0f64, |a,v| a+v), 1f64);
        }

        if verbose {
            progress = (100.0_f64 * k as f64 / (num_files - 1) as f64) as usize;
            if progress != old_progress {
                println!(
                    "Extracting data on file {} of {}: {}%",
                    (k + 1),
                    num_files,
                    progress
                );
                old_progress = progress;
            }
        }
    }
    // randomize reservoir sample order
    reservoir.shuffle(&mut rng);

    if verbose { println!("Calculating the correlation matrix..."); }
    let averages = Arc::new(averages);
    let mut cormat = vec![vec![1f64; num_files]; num_files];

    // Calculate corrlation matrix
    let (tx, rx) = mpsc::channel();
    for a in 0..num_files {
        let mut data1 = get_flattened_valid_data(Raster::new(&input_files[a], "r")?);
        data1.shuffle(&mut rng);
        let len1 = data1.len();
        let data1 = Arc::new(data1);
        for b in (a + 1)..num_files {
            let mut data2 = get_flattened_valid_data(Raster::new(&input_files[b], "r")?);
            data2.shuffle(&mut rng);
            let len2 = data2.len();
            let data2 = Arc::new(data2);
            for tid in 0..num_procs {
                let data1 = data1.clone();
                let data2 = data2.clone();
                let averages = averages.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z1: f64;
                    let mut z2: f64;
                    let mut d1_totaldev = 0f64;
                    let mut d2_totaldev = 0f64;
                    let mut sum_productdev = 0f64;
                    for i in (0..len1.min(len2) as isize).filter(|i| i % num_procs == tid) {
                        z1 = data1[i as usize];
                        z2 = data2[i as usize];
                        d1_totaldev +=
                            (z1 - averages[a]) * (z1 - averages[a]);
                        d2_totaldev +=
                            (z2 - averages[b]) * (z2 - averages[b]);
                        sum_productdev +=
                            (z1 - averages[a]) * (z2 - averages[b]);
                    }
                    tx.send((d1_totaldev, d2_totaldev, sum_productdev)).unwrap();
                });
            }
            let mut d1_total_deviation = 0f64;
            let mut d2_total_deviation = 0f64;
            let mut t_product_deviations = 0f64;
            for _ in 0..num_procs {
                let (val1, val2, val3) =
                    rx.recv().expect("Error receiving data from thread.");
                d1_total_deviation += val1;
                d2_total_deviation += val2;
                t_product_deviations += val3;
            }
            cormat[a][b] = t_product_deviations
                / (d1_total_deviation * d2_total_deviation).sqrt();
            cormat[b][a] = cormat[a][b];
        }
        if verbose {
            progress = (100.0_f64 * a as f64 / (num_files - 1) as f64) as usize;
            if progress != old_progress {
                println!(
                    "Calculating the correlation matrix ({} of {}): {}%",
                    (a + 1),
                    num_files,
                    progress
                );
                old_progress = progress;
            }
        }
    }

    // perform analysis
    if verbose { println!("Staring resampling operation..."); }

    if norm_o1 {
        // use the weighted average for obj_fn 1 so that it doesn't dominate the others.
        weights[0] = 1f64 / (0..quantiles.len())
                        .filter(|k| k_is_cont[*k])
                        .fold(0, |a, v| a + quantiles[v].len()) as f64;
    }

    let mut samples = reservoir.split_off(reservoir.len() - num_samples);
    let mut old_samples = samples.clone();
    let mut worst_indices: Vec<usize>;
    let (mut rand1, mut rand2, mut ridx): (f64, f64, usize);
    let (mut o1, mut o2, mut o3): (f64, f64, f64);
    let (mut obj, mut obj_old) = (f64::MAX, f64::MAX);

    for it in 0..max_iter {
        // populate counts matrix from samples
        let counts_matrix = get_eta(&samples, &quantiles);
        // corrlation matrix of samples
        let sample_cormat = sample_correlation_matrix(&samples, num_files);
        // objective functions
        o1 = obj_fn_continuous(&counts_matrix, &k_is_cont);
        o2 = obj_fn_catagorical(&counts_matrix, &quantiles, &k_is_cont, samples.len());
        o3 = obj_fn_correlation(&cormat, &sample_cormat);
        // calculate weighted objective function
        obj = (weights[0]*o1) + (weights[1]*o2) + (weights[2]*o3);
        if obj <= o_thresh { // exit with current samples if obj <= o_thresh
            if verbose {
                println!("Objective function has fallen below the threshold.");
            }
            break
        }

        // determine worst strata
        let (mut worst_k, mut worst_q, mut max) = (0usize, 0usize, 0usize);
        for k in 0..num_files {
            for q in 0..counts_matrix[k].len() {
                if counts_matrix[k][q] >= max {
                    worst_k = k;
                    worst_q = q;
                    max = counts_matrix[k][q];
                }
            }
        }

        // generate uniform random numbers
        rand1 = rng.gen();
        rand2 = rng.gen();

        // anneal
        let o_delta = obj - obj_old;
        let metro: f64 = (-o_delta / temp).exp();
        if it % anneal_cycle == anneal_cycle-1 { temp *= temp_decay; }

        if o_delta < 0f64 || rand1 < metro {
            // keep changes
            old_samples = samples.clone();
        } else {
            // discard changes, return samples to previous iteration
            samples = old_samples.clone();
        }

        obj_old = obj;

        // draw new samples
        if rand2 < rs_prob || max <= 1 { // random sample
            // if max <= 1, there is no worst strata, do a random sample instead
            ridx = rng.gen_range(0, samples.len());
            samples[ridx] = reservoir.pop().unwrap();
        } else { //swap random worst strata sample with fresh random sample
            //pick random worst sample by index
            worst_indices = (0..samples.len())
                    .filter(|s| samples[*s].k == worst_k
                            && samples[*s].q == worst_q).collect();
            ridx = rng.gen_range(0, worst_indices.len());
            samples[ridx] = reservoir.pop().unwrap();
        }

        if verbose {
            progress = (100.0_f64 * it as f64 / (max_iter - 1) as f64) as usize;
            if progress != old_progress {
                println!("Resampling progress: {}%.", progress);
                old_progress = progress;
            }
        }
    }

    // collect site information on final samples
    if verbose {
        println!("Final objective function value: {:.5}", obj);
    }

    //if num_samp_input < num_samples {
    //    println!("Returning samples according specified sample number.");
    //    samples.shuffle(&mut rng);
    //    samples = samples.split_off(samples.len() - num_samp);
    //}

    let elapsed_time = get_formatted_elapsed_time(start);

    if verbose {
        println!("Saving data...")
    };

    let filenames: Vec<&str> = input_files.iter().map(|p| path::Path::new(p.trim())
                    .file_name().unwrap().to_str().unwrap()).collect();
    let mut max_len = 0usize;
    for f in 0..filenames.len() {
        let l = filenames[f].len();
        max_len = max_len.max(l);
    }
    if max_len > 255 {
        max_len = 255
    }

    // Output shapefile file
    let mut output = Shapefile::new(&output_file, ShapeType::Point)?;

    // set the projection information
    output.projection = projection;

    // add the attributes
    output
        .attributes
        .add_field(&AttributeField::new("FID", FieldDataType::Int, 12u8, 0u8));
    output.attributes.add_field(&AttributeField::new(
        "VALUE",
        FieldDataType::Real,
        12u8,
        4u8,
    ));
    output.attributes.add_field(&AttributeField::new(
        "SOURCE",
        FieldDataType::Text,
        max_len as u8,
        4u8,
    ));

    for s in 0..samples.len() {
        output.add_point_record(samples[s].x, samples[s].y);
        output
            .attributes
            .add_record(vec![FieldData::Int((s+1) as i32),
                            FieldData::Real(samples[s].v),
                            FieldData::Text(filenames[samples[s].k].to_string())], false);
    }

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

#[derive(Clone, Copy)]
struct Sample {
    pub k: usize,
    pub q: usize,
    pub x: f64,
    pub y:f64,
    pub v: f64,
}

fn sample_correlation_matrix(samples: &Vec<Sample>, size: usize) -> Vec<Vec<f64>> {
    let mut output = vec![vec![1f64; size]; size];
    for a in 0..size {
        let a_samples: Vec<&Sample> = samples.iter().filter(|s| s.k == a).collect();
        if a_samples.len() == 0 {
            for b in (a+1)..size {
                output[a][b] = 0f64;
                output[b][a] = 0f64;
            }
            continue
        }
        let a_mean = a_samples.iter().fold(0f64, |acc, s| acc + s.v)
                    / a_samples.len() as f64;
        let a_tdev = a_samples.iter()
                    .fold(0f64, |acc, s| acc + ((s.v - a_mean) * (s.v - a_mean)));
        for b in (a+1)..size {
            let b_samples: Vec<&Sample> = samples.iter().filter(|s| s.k == b).collect();
            if b_samples.len() == 0 {
                output[a][b] = 0f64;
                output[b][a] = 0f64;
                continue;
            }
            let b_mean = b_samples.iter().fold(0f64, |acc, s| acc + s.v)
                        / b_samples.len() as f64;
            let b_tdev = b_samples.iter()
                        .fold(0f64, |acc, s| acc + ((s.v - b_mean) * (s.v - b_mean)));
            let mut prod_tdev = 0f64;
            for i in 0..min(a_samples.len(), b_samples.len()) {
                prod_tdev += (a_samples[i].v - a_mean) * (b_samples[i].v - b_mean)
            }
            output[a][b] = prod_tdev / (a_tdev * b_tdev).sqrt();
            output[b][a] = output[a][b];
        }
    }
    output
}

fn get_eta(samples: &Vec<Sample>, quantiles: &Vec<Vec<f64>>) -> Vec<Vec<usize>> {
    let mut counts_matrix: Vec<Vec<usize>> = quantiles.iter()
        .map(|q| vec![0usize; q.len()]).collect(); // set a copy of quantiles to 0;
    for s in 0..samples.len() {
        counts_matrix[samples[s].k][samples[s].q] += 1;
    }
    counts_matrix
}

fn obj_fn_continuous(counts: &Vec<Vec<usize>>, is_cont: &Vec<bool>) -> f64 {
    let mut o1 = 0f64;
    for k in (0..counts.len()).filter(|k| is_cont[*k]) {
        for j in 0..counts[k].len() {
            o1 += (counts[k][j] as isize - 1).abs() as f64
        }
    }
    o1
}

fn obj_fn_catagorical(counts: &Vec<Vec<usize>>,
                        kappa: &Vec<Vec<f64>>,
                        is_cont: &Vec<bool>,
                        n: usize) -> f64
{
    let mut o2 = 0f64;
    for k in (0..counts.len()).filter(|k| !is_cont[*k]) {
        for j in 0..counts[k].len() {
            o2 += ( (counts[k][j] as f64 / n as f64) - kappa[k][j] ).abs()
        }
    }
    o2
}

fn obj_fn_correlation(c: &Vec<Vec<f64>>, t: &Vec<Vec<f64>>) -> f64 {
    let mut o3 = 0f64;
    for i in 0..c.len() {
        for j in 0..c.len() {
            o3 += (c[i][j] - t[i][j]).abs()
        }
    }
    o3
}

fn rc2i(r:isize, c:isize, nc:isize) -> usize {
    (r * nc + c) as usize
}

fn i2rc(i:usize, nc:isize) -> (isize, isize) {
    let r = i as isize / nc;
    (r, i as isize - (r * nc))
}

fn get_flattened_valid_data(data:Raster) -> Vec<f64> {
    // consumes Raster
    //self.data.clone().into_iter()
    //    .filter(|x| *x != self.configs.nodata).collect::<Vec<_>>()
    let mut output: Vec<f64> = Vec::with_capacity(
                        data.configs.rows * data.configs.columns);
    let mut z: f64;
    for row in 0..data.configs.rows as isize {
        for col in 0..data.configs.columns as isize {
            z = data.get_value(row, col);
            if z != data.configs.nodata {
                output.push(z);
            }
        }
    }
    output.shrink_to(output.len());
    output
}
