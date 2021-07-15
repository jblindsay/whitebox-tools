/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Simon Gudim and Dr. John Lindsay
Created: 06/12/2019
Last Modified: 06/12/2019
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use statrs::distribution::{StudentsT, Univariate};
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to perform nieghbourhood-based (i.e. using roving search windows applied to each
/// grid cell) correlation analysis on two input rasters (`--input1` and `--input2`). The tool outputs a
/// correlation value raster (`--output1`) and a significance (p-value) raster (`--output2`). Additionally,
/// the user must specify the size of the search window (`--filter`) and the correlation statistic (`--stat`).
/// Options for the correlation statistic include [`pearson`](https://en.wikipedia.org/wiki/Pearson_correlation_coefficient),
/// [`kendall`](https://en.wikipedia.org/wiki/Kendall_rank_correlation_coefficient), and
/// [`spearman`](https://en.wikipedia.org/wiki/Spearman%27s_rank_correlation_coefficient). Notice that Pearson's *r* is the
/// most computationally efficient of the three correlation metrics but is unsuitable when the input distributions are
/// non-linearly associated, in which case, either Spearman's Rho or Kendall's tau-b correlations are more suited.
/// Both Spearman and Kendall correlations evaluate monotonic associations without assuming linearity in the relation.
/// Kendall's tau-b is by far the most computationally expensive of the three statistics and may not be suitable to
/// larger sized search windows.
///
/// # See Also
/// `ImageCorrelation`, `ImageRegression`
pub struct ImageCorrelationNeighbourhoodAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImageCorrelationNeighbourhoodAnalysis {
    pub fn new() -> ImageCorrelationNeighbourhoodAnalysis {
        // public constructor
        let name = "ImageCorrelationNeighbourhoodAnalysis".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs image correlation on two input images neighbourhood search windows."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Image 1".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Image 2".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Correlation File".to_owned(),
            flags: vec!["--o1".to_owned(), "--output1".to_owned()],
            description: "Output correlation (r-value or rho) raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Significance File".to_owned(),
            flags: vec!["--o2".to_owned(), "--output2".to_owned()],
            description: "Output significance (p-value) raster file.".to_owned(),
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
            name: "Correlation Statistic Type".to_owned(),
            flags: vec!["--stat".to_owned()],
            description: "Correlation type; one of 'pearson' (default) and 'spearman'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "pearson".to_owned(),
                "kendall".to_owned(),
                "spearman".to_owned(),
            ]),
            default_value: Some("pearson".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=file1.tif --i2=file2.tif --o1=corr.tif --o2=sig.tif --stat=\"spearman\"",
                            short_exe,
                            name)
                .replace("*", &sep);

        ImageCorrelationNeighbourhoodAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImageCorrelationNeighbourhoodAnalysis {
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
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file1 = String::new();
        let mut output_file2 = String::new();
        let mut filter_size = 11usize;
        let mut stat_type = String::from("pearson");

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
            if flag_val == "-i1" || flag_val == "-input1" {
                input_file1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-i2" || flag_val == "-input2" {
                input_file2 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o1" || flag_val == "-output1" {
                output_file1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o2" || flag_val == "-output2" {
                output_file2 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-stat" {
                let val = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i + 1].to_lowercase()
                };
                stat_type = if val.contains("son") {
                    "pearson".to_string()
                } else if val.contains("kendall") {
                    "kendall".to_string()
                } else {
                    "spearman".to_string()
                };
            } else if vec[0].to_lowercase() == "-filter" || vec[0].to_lowercase() == "--filter" {
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
        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        //let start = time::now();
        let start = Instant::now(); // had to change to this from old time::now

        if !input_file1.contains(&sep) && !input_file1.contains("/") {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) && !input_file2.contains("/") {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }

        if !output_file1.contains(&sep) && !output_file1.contains("/") {
            output_file1 = format!("{}{}", working_directory, output_file1);
        }

        if !output_file2.contains(&sep) && !output_file2.contains("/") {
            output_file2 = format!("{}{}", working_directory, output_file2);
        }

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        if verbose {
            println!("Reading data...")
        };

        let image1 = Arc::new(Raster::new(&input_file1, "r")?);
        let rows = image1.configs.rows as isize;
        let columns = image1.configs.columns as isize;
        let nodata1 = image1.configs.nodata;

        let image2 = Arc::new(Raster::new(&input_file2, "r")?);
        if image2.configs.rows as isize != rows || image2.configs.columns as isize != columns {
            panic!("Error: The input files do not contain the same raster extent.");
        }
        let nodata2 = image2.configs.nodata;

        // The r-value output
        let mut output_val = Raster::initialize_using_file(&output_file1, &image1);
        // The significance (p-value) output
        let mut output_sig = Raster::initialize_using_file(&output_file2, &image1);

        // Pearson's Correlation //
        if stat_type == "pearson" {
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let image1 = image1.clone();
                let image2 = image2.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut num_cells: usize;
                    let mut sum1: f64;
                    let mut sum2: f64;
                    let mut total_deviation1: f64;
                    let mut total_deviation2: f64;
                    let mut product_deviations: f64;
                    let (mut mean1, mut mean2): (f64, f64);
                    let mut r: f64;
                    let mut df: usize;
                    let mut tvalue: f64;
                    let mut pvalue: f64;
                    let (mut z1, mut z2): (f64, f64);
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let num_pixels_in_filter = filter_size * filter_size;
                    let mut dx = vec![0isize; num_pixels_in_filter];
                    let mut dy = vec![0isize; num_pixels_in_filter];

                    // fill the filter d_x and d_y values
                    let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
                    let mut a = 0;
                    for row in 0..filter_size {
                        for col in 0..filter_size {
                            dx[a] = col as isize - midpoint;
                            dy[a] = row as isize - midpoint;
                            a += 1;
                        }
                    }

                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data1 = vec![nodata1; columns as usize];
                        let mut data2 = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            z1 = image1.get_value(row, col);
                            z2 = image2.get_value(row, col);
                            if z1 != nodata1 && z2 != nodata2 {
                                // First, calculate the mean
                                num_cells = 0;
                                sum1 = 0f64;
                                sum2 = 0f64;
                                for i in 0..num_pixels_in_filter {
                                    z_n1 = image1.get_value(row + dy[i], col + dx[i]);
                                    z_n2 = image2.get_value(row + dy[i], col + dx[i]);
                                    if z_n1 != nodata1 && z_n2 != nodata2 {
                                        sum1 += z_n1;
                                        sum2 += z_n2;
                                        num_cells += 1;
                                    }
                                }
                                mean1 = sum1 / num_cells as f64;
                                mean2 = sum2 / num_cells as f64;

                                // Now calculate the total deviations and total cross-deviation.
                                total_deviation1 = 0f64;
                                total_deviation2 = 0f64;
                                product_deviations = 0f64;
                                if num_cells > 2 {
                                    for i in 0..num_pixels_in_filter {
                                        z_n1 = image1.get_value(row + dy[i], col + dx[i]);
                                        z_n2 = image2.get_value(row + dy[i], col + dx[i]);
                                        if z_n1 != nodata1 && z_n2 != nodata2 {
                                            total_deviation1 += (z_n1 - mean1) * (z_n1 - mean1);
                                            total_deviation2 += (z_n2 - mean2) * (z_n2 - mean2);
                                            product_deviations += (z_n1 - mean1) * (z_n2 - mean2);
                                        }
                                    }
                                }

                                // Finally, calculate r for the neighbourhood.
                                r = if total_deviation1 != 0f64
                                    && total_deviation2 != 0f64
                                    && num_cells > 2
                                {
                                    product_deviations
                                        / (total_deviation1 * total_deviation2).sqrt()
                                } else {
                                    // You can't divide by zero
                                    0f64
                                };

                                data1[col as usize] = r;

                                df = num_cells - 2;
                                if df > 2 {
                                    tvalue = r * (df as f64 / (1f64 - r * r)).sqrt();
                                    let t = StudentsT::new(0.0, 1.0, df as f64).unwrap();
                                    pvalue = 2f64 * (1f64 - t.cdf(tvalue.abs()));
                                    data2[col as usize] = pvalue;
                                } else {
                                    data2[col as usize] = 0f64;
                                }
                            }
                        }
                        tx.send((row, data1, data2)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data1, data2) = rx.recv().expect("Error receiving data from thread.");
                output_val.set_row_data(row, data1);
                output_sig.set_row_data(row, data2);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing Correlation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else if stat_type == "kendall" {
            // Perform Kendall's Tau-b correlation
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let image1 = image1.clone();
                let image2 = image2.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut num_cells: usize;
                    let mut num_cells_f64: f64;
                    let mut tau: f64;
                    let mut df: f64;
                    let mut zvalue: f64;
                    let mut pvalue: f64;
                    let (mut z1, mut z2): (f64, f64);
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let num_pixels_in_filter = filter_size * filter_size;
                    let mut dx = vec![0isize; num_pixels_in_filter];
                    let mut dy = vec![0isize; num_pixels_in_filter];
                    let (mut rank, mut rank2): (f64, f64);
                    let mut upper_range: usize;

                    let mut num_tied_vals: f64;
                    let mut nt1: f64;
                    let mut nt2: f64;
                    let mut n0: f64;
                    let mut numer: f64;

                    let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
                    let mut a = 0;
                    for row in 0..filter_size {
                        for col in 0..filter_size {
                            dx[a] = col as isize - midpoint;
                            dy[a] = row as isize - midpoint;
                            a += 1;
                        }
                    }

                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data1 = vec![nodata1; columns as usize];
                        let mut data2 = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            z1 = image1.get_value(row, col);
                            z2 = image2.get_value(row, col);
                            if z1 != nodata1 && z2 != nodata2 {
                                let mut v1 = Vec::with_capacity(num_pixels_in_filter);
                                let mut v2 = Vec::with_capacity(num_pixels_in_filter);
                                num_cells = 0;
                                for i in 0..num_pixels_in_filter {
                                    z_n1 = image1.get_value(row + dy[i], col + dx[i]);
                                    z_n2 = image2.get_value(row + dy[i], col + dx[i]);
                                    if z_n1 != nodata1 && z_n2 != nodata2 {
                                        num_cells += 1;
                                        // tuple = (value, index, rank)
                                        v1.push((z_n1, num_cells, 0f64));
                                        v2.push((z_n2, num_cells, 0f64));
                                    }
                                }
                                num_cells_f64 = num_cells as f64;

                                // Sort both lists based on value
                                v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
                                v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));

                                // Now provide the rank data
                                rank = 0f64;
                                nt1 = 0f64;
                                for i in 0..num_cells {
                                    if v1[i].2 == 0f64 {
                                        rank += 1f64;
                                        if i < num_cells - 1 {
                                            // are there any ties above this one?
                                            upper_range = i;
                                            for j in i + 1..num_cells {
                                                if v1[i].0 == v1[j].0 {
                                                    upper_range = j;
                                                } else {
                                                    break;
                                                }
                                            }
                                            if upper_range != i {
                                                num_tied_vals = (upper_range - i + 1) as f64;
                                                nt1 +=
                                                    num_tied_vals * (num_tied_vals - 1f64) / 2f64;
                                                rank2 = rank + (upper_range - i) as f64;
                                                rank = (rank + rank2) / 2f64; // average rank
                                                for k in i..=upper_range {
                                                    v1[k].2 = rank;
                                                }
                                                rank = rank2;
                                            } else {
                                                v1[i].2 = rank;
                                            }
                                        } else {
                                            v1[i].2 = rank;
                                        }
                                    }
                                }

                                nt2 = 0f64;
                                rank = 0f64;
                                for i in 0..num_cells {
                                    if v2[i].2 == 0f64 {
                                        rank += 1f64;
                                        if i < num_cells - 1 {
                                            // are there any ties above this one?
                                            upper_range = i;
                                            for j in i + 1..num_cells {
                                                if v2[i].0 == v2[j].0 {
                                                    upper_range = j;
                                                } else {
                                                    break;
                                                }
                                            }
                                            if upper_range != i {
                                                num_tied_vals = (upper_range - i + 1) as f64;
                                                nt2 +=
                                                    num_tied_vals * (num_tied_vals - 1f64) / 2f64;
                                                rank2 = rank + (upper_range - i) as f64;
                                                rank = (rank + rank2) / 2f64; // average rank
                                                for k in i..=upper_range {
                                                    v2[k].2 = rank;
                                                }
                                                rank = rank2;
                                            } else {
                                                v2[i].2 = rank;
                                            }
                                        } else {
                                            v2[i].2 = rank;
                                        }
                                    }
                                }

                                // Sort both lists based on index
                                v1.sort_by(|a, b| a.1.cmp(&b.1));
                                v2.sort_by(|a, b| a.1.cmp(&b.1));

                                ////////////////////////////////////////////////////////////////////////////
                                // This block of code is O(n^2) and is a serious performance killer. There
                                // is a O(nlogn) solution based on swaps in a merge-sort but I have yet to
                                // figure it out. As it stands, this solution is unacceptable for search
                                // windows larger than about 25, depending the number of cores in the
                                // system processor.
                                ////////////////////////////////////////////////////////////////////////////
                                numer = 0f64;
                                for i in 0..num_cells {
                                    for j in i + 1..num_cells {
                                        if v1[i].2 != v1[j].2 && v2[i].2 != v2[j].2 {
                                            numer += (v1[i].2 - v1[j].2).signum()
                                                * (v2[i].2 - v2[j].2).signum();
                                        }
                                    }
                                }

                                n0 = num_cells as f64 * (num_cells as f64 - 1f64) / 2f64;
                                tau = numer / ((n0 - nt1) * (n0 - nt2)).sqrt();
                                data1[col as usize] = tau;
                                df = num_cells_f64 - 2f64;

                                if df > 2f64 {
                                    zvalue = 3f64 * numer
                                        / (num_cells_f64
                                            * (num_cells_f64 - 1f64)
                                            * (2f64 * num_cells_f64 + 5f64)
                                            / 2f64)
                                            .sqrt();
                                    let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
                                    pvalue = 2f64 * (1f64 - t.cdf(zvalue.abs())); // calculate the p-value (significance)
                                    data2[col as usize] = pvalue;
                                } else {
                                    data2[col as usize] = 0f64;
                                }
                            }
                        }
                        tx.send((row, data1, data2)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data1, data2) = rx.recv().expect("Error receiving data from thread.");
                output_val.set_row_data(row, data1);
                output_sig.set_row_data(row, data2);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing Correlation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // Calculate Spearman's Rho correlation
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let image1 = image1.clone();
                let image2 = image2.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut num_cells: usize;
                    let mut num_cells_f64: f64;
                    let mut rho: f64;
                    let mut df: f64;
                    let mut tvalue: f64;
                    let mut pvalue: f64;
                    let (mut z1, mut z2): (f64, f64);
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let num_pixels_in_filter = filter_size * filter_size;
                    let mut dx = vec![0isize; num_pixels_in_filter];
                    let mut dy = vec![0isize; num_pixels_in_filter];
                    let (mut rank, mut rank2): (f64, f64);
                    let mut upper_range: usize;
                    let mut num_ties = 0;
                    let mut num_ties_test: isize;
                    let mut max_num_ties: isize;
                    let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
                    let mut a = 0;
                    for row in 0..filter_size {
                        for col in 0..filter_size {
                            dx[a] = col as isize - midpoint;
                            dy[a] = row as isize - midpoint;
                            a += 1;
                        }
                    }

                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data1 = vec![nodata1; columns as usize];
                        let mut data2 = vec![nodata1; columns as usize];
                        max_num_ties = -1;
                        for col in 0..columns {
                            z1 = image1.get_value(row, col);
                            z2 = image2.get_value(row, col);
                            if z1 != nodata1 && z2 != nodata2 {
                                let mut v1 = Vec::with_capacity(num_pixels_in_filter);
                                let mut v2 = Vec::with_capacity(num_pixels_in_filter);
                                num_cells = 0;
                                for i in 0..num_pixels_in_filter {
                                    z_n1 = image1.get_value(row + dy[i], col + dx[i]);
                                    z_n2 = image2.get_value(row + dy[i], col + dx[i]);
                                    if z_n1 != nodata1 && z_n2 != nodata2 {
                                        num_cells += 1;
                                        // tuple = (value, index, rank)
                                        v1.push((z_n1, num_cells, 0f64));
                                        v2.push((z_n2, num_cells, 0f64));
                                    }
                                }
                                num_cells_f64 = num_cells as f64;

                                // Sort both lists based on value
                                v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
                                v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
                                num_ties_test = 0;
                                rank = 0f64;
                                for i in 0..num_cells {
                                    if v1[i].2 == 0f64 {
                                        rank += 1f64;
                                        if i < num_cells - 1 {
                                            // are there any ties above this one?
                                            upper_range = i;
                                            for j in i + 1..num_cells {
                                                if v1[i].0 == v1[j].0 {
                                                    upper_range = j;
                                                    num_ties += 1;
                                                    num_ties_test += 1;
                                                } else {
                                                    break;
                                                }
                                            }
                                            if upper_range != i {
                                                rank2 = rank + (upper_range - i) as f64;
                                                rank = (rank + rank2) / 2f64; // average rank
                                                for k in i..=upper_range {
                                                    v1[k].2 = rank;
                                                }
                                                rank = rank2;
                                            } else {
                                                v1[i].2 = rank;
                                            }
                                        } else {
                                            v1[i].2 = rank;
                                        }
                                    }
                                }

                                rank = 0f64;
                                for i in 0..num_cells {
                                    if v2[i].2 == 0f64 {
                                        rank += 1f64;
                                        if i < num_cells - 1 {
                                            // are there any ties above this one?
                                            upper_range = i;
                                            for j in i + 1..num_cells {
                                                if v2[i].0 == v2[j].0 {
                                                    upper_range = j;
                                                    num_ties += 1;
                                                    num_ties_test += 1;
                                                } else {
                                                    break;
                                                }
                                            }
                                            if upper_range != i {
                                                rank2 = rank + (upper_range - i) as f64;
                                                rank = (rank + rank2) / 2f64; // average rank
                                                for k in i..=upper_range {
                                                    v2[k].2 = rank;
                                                }
                                                rank = rank2;
                                            } else {
                                                v2[i].2 = rank;
                                            }
                                        } else {
                                            v2[i].2 = rank;
                                        }
                                    }
                                }

                                // Sort both lists based on index
                                v1.sort_by(|a, b| a.1.cmp(&b.1));
                                v2.sort_by(|a, b| a.1.cmp(&b.1));

                                let mut rank_diff_sqrd = 0f64;
                                for i in 0..num_cells {
                                    rank_diff_sqrd += (v1[i].2 - v2[i].2) * (v1[i].2 - v2[i].2);
                                }

                                rho = 1f64
                                    - (6f64 * rank_diff_sqrd
                                        / (num_cells_f64 * num_cells_f64 * num_cells_f64
                                            - num_cells_f64));
                                data1[col as usize] = rho;
                                df = num_cells_f64 - 2f64; // calculate degrees of freedom (Anthony Comment)

                                if df > 2f64 {
                                    tvalue = rho * (df / (1f64 - rho * rho)).sqrt();
                                    let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
                                    pvalue = 2f64 * (1f64 - t.cdf(tvalue.abs())); // calculate the p-value (significance)
                                    data2[col as usize] = pvalue;
                                } else {
                                    data2[col as usize] = 0f64;
                                }

                                if max_num_ties < num_ties_test {
                                    max_num_ties = num_ties_test;
                                }
                            }
                        }
                        tx.send((row, data1, data2, num_ties, max_num_ties))
                            .unwrap();
                    }
                });
            }

            let mut max_ties = -1isize;
            let mut num_ties = 0;
            for r in 0..rows {
                let (row, data1, data2, ties, max_row_ties) =
                    rx.recv().expect("Error receiving data from thread.");
                output_val.set_row_data(row, data1);
                output_sig.set_row_data(row, data2);
                num_ties += ties;
                if max_row_ties > max_ties {
                    max_ties = max_row_ties;
                }

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Performing Correlation: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            if num_ties > 0 {
                println!("Warning: There was a maximum of {} ties in a test and as a result p-values \nmay be misleading. You may want to consider using Kendall's Tau instead.", max_ties);
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        output_val.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_val.add_metadata_entry(format!("Filter size: {}", filter_size));
        output_val.add_metadata_entry(format!("Input file 1: {}", input_file1));
        output_val.add_metadata_entry(format!("Input file 2: {}", input_file2));
        output_val.add_metadata_entry(format!("Statistic: {}", stat_type));
        output_val.add_metadata_entry(
            format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
        );

        if verbose {
            println!("Saving data...")
        };
        let _ = match output_val.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        output_sig.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_sig.add_metadata_entry(format!("Filter size: {}", filter_size));
        output_sig.add_metadata_entry(format!("Input file 1: {}", input_file1));
        output_sig.add_metadata_entry(format!("Input file 2: {}", input_file2));
        output_val.add_metadata_entry(format!("Statistic: {}", stat_type));
        output_sig.add_metadata_entry(
            format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
        );

        let _ = match output_sig.write() {
            Ok(_) => {
                if verbose {
                    println!(" ")
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

        // println!("Number of total cells: {}", valid);
        // println!("Numbed of significant cells: {}", sig);
        // println!("Numbed of significant negative cells: {}", sig_neg);
        // println!("Numbed of significant positive cells: {}", sig_pos);

        Ok(())
    }
}
// #[derive(PartialEq, Debug)]
// struct GridCell {
//     pub z: f64,
//     pub index: usize,
//     pub rank: f64,
// }

// impl Eq for GridCell {}

// impl PartialOrd for GridCell {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         self.z.partial_cmp(&other.z)
//     }
// }

// impl Ord for GridCell {
//     fn cmp(&self, other: &GridCell) -> Ordering {
//         self.partial_cmp(other).unwrap()
//     }
// }
