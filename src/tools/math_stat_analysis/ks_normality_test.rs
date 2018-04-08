/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: January 2, 2018
Last Modified: January 2, 2018
License: MIT

Notes: This tool will perform a Kolmogorov-Smirnov (K-S) test for normality to evaluate 
whether the frequency distribution of values within a raster image are drawn from a 
Gaussian (normal) distribution. The user must specify the name of the raster image. The 
test can be performed optionally on the entire image or on a random sub-sample of pixel 
values of a user-specified size. In evaluating the significance of the test, it is 
important to keep in mind that given a sufficiently large sample, extremely small and 
non-notable differences can be found to be statistically significant. Furthermore 
statistical significance says nothing about the practical significance of a difference.
*/

use time;
use rand;
use std::env;
use std::path;
use std::f64;
use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::process::Command;
use raster::*;
use tools::*;
use rendering::html::*;
use rendering::Histogram;
use self::rand::distributions::{IndependentSample, Range};

pub struct KSTestForNormality {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl KSTestForNormality {
    pub fn new() -> KSTestForNormality {
        // public constructor
        let name = "KSTestForNormality".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Evaluates whether the values in a raster are normally distributed.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output HTML file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Num. Samples (blank for while image)".to_owned(), 
            flags: vec!["--num_samples".to_owned()], 
            description: "Number of samples. Leave blank to use whole image.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.html --num_samples=1000
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.html", short_exe, name).replace("*", &sep);

        KSTestForNormality {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for KSTestForNormality {
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
        let mut num_samples = 0usize;
        
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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-base" {
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
            } else if flag_val == "-num_samples" {
                num_samples = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap() as usize
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
        if !output_file.ends_with(".html") {
            output_file = output_file + ".html";
        }

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        
        if num_samples > (rows * columns) as usize {
            num_samples = (rows * columns) as usize;
        }

        // declare some variables
        let mut z: f64;
        let min_value = input.configs.minimum;
        let max_value = input.configs.maximum;
        let num_bins = 10000usize; 
        let bin_size = (max_value - min_value) / num_bins as f64;
        let mut histogram = vec![0usize; num_bins];
        let mut bin_num: usize;
        let num_bins_less_one = num_bins - 1usize;
        let mut total = 0f64;
        let mean: f64;
        let mut n = 0f64;
        let mut total_deviation = 0f64;

        let fd_num_bins = ((rows * columns) as f64).log2().ceil() as usize + 1;
        let fd_bin_width = (max_value - min_value + 0.00001f64) / fd_num_bins as f64;
        let mut freq_data = vec![0usize; fd_num_bins];

        if num_samples == 0 {
            for row in 0..rows {
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata {
                        bin_num = ((z - min_value) / bin_size).floor() as usize;
                        if bin_num > num_bins_less_one {
                            bin_num = num_bins_less_one;
                        }
                        histogram[bin_num] += 1usize;
                        total += z;
                        n += 1f64;

                        bin_num = ((z - min_value) / fd_bin_width).floor() as usize;
                        freq_data[bin_num] += 1;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / rows as f64) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            mean = total / n;

            for row in 0..rows {
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata {
                        total_deviation += (z - mean) * (z - mean);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / rows as f64) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // Calculate the mean and total_deviation from a random sample.
            // Note that this is sampling with replacement.
            let mut rng = rand::thread_rng();
            let row_rng = Range::new(0, rows as isize);
            let col_rng = Range::new(0, columns as isize);
            let (mut row, mut col, mut cell_index): (isize, isize, isize);
            let mut sample_cells = Vec::with_capacity(num_samples);
            let mut sample_num = 0usize;
            while sample_num < num_samples {
                row = row_rng.ind_sample(&mut rng);
                col = col_rng.ind_sample(&mut rng);
                z = input.get_value(row, col);
                if z != nodata {
                    bin_num = ((z - min_value) / bin_size).floor() as usize;
                    if bin_num > num_bins_less_one {
                        bin_num = num_bins_less_one;
                    }
                    histogram[bin_num] += 1usize;
                    total += z;
                    n += 1f64;

                    bin_num = ((z - min_value) / fd_bin_width).floor() as usize;
                    freq_data[bin_num] += 1;

                    sample_num += 1;
                    cell_index = row * columns + col;
                    sample_cells.push(cell_index);

                    if verbose {
                        progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }

            mean = total / n;

            for sample_num in 0..num_samples {
                cell_index = sample_cells[sample_num];
                col = cell_index % columns;
                row = (cell_index as f64 / columns as f64).floor() as isize;
                z = input.get_value(row, col);
                total_deviation += (z - mean) * (z - mean);

                if verbose {
                    progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let std_dev = (total_deviation / (n - 1f64)).sqrt();

        let mut cdf = vec![0f64; num_bins];
        cdf[0] = histogram[0] as f64; 
        for i in 1..num_bins {
            cdf[i] = cdf[i - 1] + histogram[i] as f64;
        }
        
        for i in 0..num_bins {
            cdf[i] = cdf[i] / n;
        }
        
        let mut normal_dist = vec![0f64; num_bins];
        let sd_root2pi = std_dev * (2f64 * f64::consts::PI).sqrt();
        let two_sd_sqr = 2f64 * std_dev * std_dev;
        for i in 0..num_bins {
            z = min_value + i as f64 * bin_size;
            normal_dist[i] = 1f64 / sd_root2pi * ((-(z - mean) * (z - mean)) / two_sd_sqr).exp();
        }
        for i in 1..num_bins {
            normal_dist[i] = normal_dist[i - 1] + normal_dist[i];
        }
        
        for i in 1..num_bins {
            normal_dist[i] = normal_dist[i] / normal_dist[num_bins - 1];
        }
        
        // calculate the critical statistic, Dmax
        let mut dmax = 0f64;
        for i in 0..num_bins {
            z = (cdf[i] - normal_dist[i]).abs();
            if z > dmax {
                dmax = z;
            }
        }
        
        // calculate p-value
        let s = n * dmax * dmax;
        let p_value = 2f64 * (-(2.000071f64 + 0.331f64 / n.sqrt() + 1.409f64 / n) * s).exp();
        
        ///////////////////////
        // Output the report //
        ///////////////////////
        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>K-S Test for Normality</title>"#.as_bytes())?;
        
        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;
            
        writer.write_all(&r#"
            </head>
            <body>
                <h1>Kolmogorov-Smirnov (K-S) Test for Normality Report</h1>
                <p>"#.as_bytes())?;

        writer.write_all(&format!("<strong>Input image</strong>: {}<br>", input_file).as_bytes())?;
        writer.write_all(&format!("<strong>Sample size (N)</strong>: {:.0}<br>", n).as_bytes())?;
        writer.write_all(&format!("<strong>Test Statistic (D<sub>max</sub>)</strong>: {:.4}<br>", dmax).as_bytes())?;
        if p_value > 0.001f64 {
            writer.write_all(&format!("<strong>Significance (p-value)</strong>: {:.4}<br>", p_value).as_bytes())?;
        } else {
            writer.write_all("<strong>Significance (p-value)</strong>: <0.001<br>".to_string().as_bytes())?;
        }
        if p_value < 0.05 {
            writer.write_all("<strong>Result</strong>: The test <strong>rejects</strong> the null hypothesis that the values come from a normal distribution.<br>".to_string().as_bytes())?;
        } else {
            writer.write_all("<strong>Result</strong>: The test <strong>fails to reject</strong> the null hypothesis that the values come from a normal distribution.<br>".to_string().as_bytes())?;
        }

        writer.write_all("</p>".as_bytes())?;
        
        writer.write_all("<p><strong>Caveat</strong>: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, \nand statistical significance says nothing about the practical significance of a difference.</p>".to_string().as_bytes())?;
        
        let histo = Histogram {
            parent_id: "histo".to_owned(),
            width: 700f64,
            height: 500f64,
            freq_data: freq_data.clone(),
            min_bin_val: min_value, 
            bin_width: fd_bin_width,
            x_axis_label: "Value".to_string(),
            cumulative: true,
        };

        writer.write_all(&format!("<div id='histo' align=\"center\">{}</div>", histo.get_svg()).as_bytes())?;

        writer.write_all("</body>".as_bytes())?;
        writer.write_all("</html>".as_bytes())?;

        let _ = writer.flush();

        if verbose {
            if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                let output = Command::new("open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "windows") {
                // let output = Command::new("cmd /c start")
                let output = Command::new("explorer.exe")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "linux") {
                let output = Command::new("xdg-open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            }

            println!("Complete! Please see {} for output.", output_file);
        }
        
        
        let end = time::now();
        let elapsed_time = end - start;
        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }
        
        Ok(())
    }
}
