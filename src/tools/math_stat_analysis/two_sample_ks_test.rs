/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/10/2019
Last Modified: 24/10/2019
License: MIT
*/

use crate::raster::*;
use crate::rendering::html::*;
use crate::rendering::LineGraph;
use crate::tools::*;
use rand::prelude::*;
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool will perform a two-sample Kolmogorov-Smirnov (K-S) test to evaluate whether a significant 
/// statistical difference exists between the frequency distributions of two rasters. The user must 
/// specify the name of the two input raster images (`--input1` and `--input2`) and the output report
/// HTML file (`--output`). The test can be performed optionally on the entire image or on a random 
/// sub-sample of pixel values of a user-specified size (`--num_samples`). In evaluating the significance 
/// of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and
/// non-notable differences can be found to be statistically significant. Furthermore
/// statistical significance says nothing about the practical significance of a difference.
/// 
/// # See Also
/// `KSTestForNormality`
pub struct TwoSampleKSTest {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TwoSampleKSTest {
    pub fn new() -> TwoSampleKSTest {
        // public constructor
        let name = "TwoSampleKSTest".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs a 2-sample K-S test for significant differences on two input rasters.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "First Input File".to_owned(),
            flags: vec!["--input1".to_owned()],
            description: "First input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Second Input File".to_owned(),
            flags: vec!["--input2".to_owned()],
            description: "Second input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output HTML file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Num. Samples (blank for while image)".to_owned(),
            flags: vec!["--num_samples".to_owned()],
            description: "Number of samples. Leave blank to use whole image.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1=input1.tif -input2=input2.tif -o=output.html --num_samples=1000", short_exe, name).replace("*", &sep);

        TwoSampleKSTest {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TwoSampleKSTest {
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
        let mut output_file = String::new();
        let mut num_samples = 0usize;

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            if flag_val == "-input1" {
                input_file1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-input2" {
                input_file2 = if keyval {
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

        if !input_file1.contains(&sep) && !input_file1.contains("/") {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) && !input_file2.contains("/") {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !output_file.ends_with(".html") {
            output_file = output_file + ".html";
        }

        let input1 = Raster::new(&input_file1, "r")?;
        let input1_name = input1.get_short_filename();
        let input2 = Raster::new(&input_file2, "r")?;
        let input2_name = input2.get_short_filename();

        if input1.configs.rows != input2.configs.rows || input1.configs.columns != input2.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent."));
        }

        let start = Instant::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;

        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;
        let nodata2 = input2.configs.nodata;

        if num_samples > (rows * columns) as usize {
            num_samples = 0; // This will cause it to use every grid cell
        }

        // declare some variables
        let mut z1: f64;
        let mut z2: f64;
        let mut data1: Vec<f64> = Vec::with_capacity((rows * columns) as usize);
        let mut data2: Vec<f64> = Vec::with_capacity((rows * columns) as usize);

        if num_samples == 0 {
            for row in 0..rows {
                for col in 0..columns {
                    z1 = input1.get_value(row, col);
                    if z1 != nodata1 {
                        data1.push(z1);
                    }
                    z2 = input2.get_value(row, col);
                    if z2 != nodata2 {
                        data2.push(z2);
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
            data1 = Vec::with_capacity(num_samples);
            data2 = Vec::with_capacity(num_samples);

            // Note that this is sampling with replacement, which is not ideal.
            let mut rng = thread_rng();
            let (mut row, mut col): (isize, isize);
            let mut sample_num = 0usize;
            while sample_num < num_samples {
                row = rng.gen_range(0, rows as isize);
                col = rng.gen_range(0, columns as isize);
                z1 = input1.get_value(row, col);
                if z1 != nodata1 {
                    data1.push(z1);

                    sample_num += 1;

                    if verbose {
                        progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }

            sample_num = 0usize;
            while sample_num < num_samples {
                row = rng.gen_range(0, rows as isize);
                col = rng.gen_range(0, columns as isize);
                z2 = input2.get_value(row, col);
                if z2 != nodata2 {
                    data2.push(z2);

                    sample_num += 1;

                    if verbose {
                        progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let mut j1 = 0;
        let mut j2 = 0;
        let n1 = data1.len();
        let n2 = data2.len();
        let (mut d1, mut d2, mut dt): (f64, f64, f64);
        let mut fn1 = 0.0f64;
        let mut fn2 = 0.0f64;
        let mut dmax = -1.0;

        // sort data1 and data2
        data1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        data2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));

        let en1 = n1 as f64;
        let en2 = n2 as f64;

        while j1 < n1 && j2 < n2 {
            d1 = data1[j1];
            d2 = data2[j2];
            if d1 <= d2 {
                j1 += 1;
                fn1 = j1 as f64 / en1;
            }

            if d2 <= d1 {
                j2 += 1;
                fn2 = j2 as f64 / en2;
            }

            dt = (fn2 - fn1).abs();
            if dt > dmax {
                dmax = dt;
            }
        }

        let en = (en1 * en2 / (en1 + en2)).sqrt();

        let mut p_value = calculate_p_value(en * dmax);
        if p_value < 0f64 { p_value = 0f64; }
        if p_value > 1f64 { p_value = 1f64; }

        // create the cdf's
        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut series_names = vec![];
        let num_bins = 100usize;
        let mut min_val = data1[0];
        let mut max_val = data1[n1-1];
        let mut bin_size = (max_val - min_val) / num_bins as f64;
        let mut bin: usize;
        let profile_xdata = (0..num_bins).map(|x| min_val + x as f64 * bin_size).collect::<Vec<f64>>();
        let mut profile_ydata = vec![0f64; num_bins];
        // bin frequency data
        for val in &data1 {
            bin = ((val - min_val) / bin_size).floor() as usize;
            if bin > num_bins -1 { bin = num_bins - 1; }
            profile_ydata[bin] += 1f64;
        }
        for bin in 1..num_bins {
            profile_ydata[bin] += profile_ydata[bin-1];
        }
        for bin in 0..num_bins {
            profile_ydata[bin] /= en1;
        }
        xdata.push(profile_xdata.clone());
        ydata.push(profile_ydata.clone());
        series_names.push(input1_name.clone());

        min_val = data2[0];
        max_val = data2[n2-1];
        bin_size = (max_val - min_val) / num_bins as f64;
        let profile_xdata = (0..num_bins).map(|x| min_val + x as f64 * bin_size).collect::<Vec<f64>>();
        profile_ydata = vec![0f64; num_bins];
        // bin frequency data
        for val in &data2 {
            bin = ((val - min_val) / bin_size).floor() as usize;
            if bin > num_bins -1 { bin = num_bins - 1; }
            profile_ydata[bin] += 1f64;
        }
        for bin in 1..num_bins {
            profile_ydata[bin] += profile_ydata[bin-1];
        }
        for bin in 0..num_bins {
            profile_ydata[bin] /= en2;
        }
        xdata.push(profile_xdata.clone());
        ydata.push(profile_ydata.clone());
        series_names.push(input2_name.clone());

        
        ///////////////////////
        // Output the report //
        ///////////////////////
        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Two-Sample K-S Test</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"
            </head>
            <body>
                <h1>Two-Sample Kolmogorov-Smirnov (K-S) Test Report</h1>
                <p>"#
                .as_bytes(),
        )?;

        writer.write_all(&format!("<strong>Input image 1</strong>: {}<br>", input1_name.clone()).as_bytes())?;
        writer.write_all(&format!("<strong>Input image 2</strong>: {}<br>", input2_name.clone()).as_bytes())?;
        writer.write_all(&format!("<strong>Sample size 1 (n1)</strong>: {:.0}<br>", n1).as_bytes())?;
        writer.write_all(&format!("<strong>Sample size 2 (n2)</strong>: {:.0}<br>", n2).as_bytes())?;
        writer.write_all(
            &format!(
                "<strong>Test Statistic (D<sub>max</sub>)</strong>: {:.4}<br>",
                dmax
            )
            .as_bytes(),
        )?;
        if p_value > 0.001f64 {
            writer.write_all(
                &format!(
                    "<strong>Two-tailed Significance (p-value)</strong>: {:.4}<br>",
                    p_value
                )
                .as_bytes(),
            )?;
        } else {
            writer.write_all(
                "<strong>Two-tailed Significance (p-value)</strong>: <0.001<br>"
                    .to_string()
                    .as_bytes(),
            )?;
        }
        if p_value < 0.05 {
            writer.write_all("<strong>Result</strong>: The test <strong>rejects</strong> the null hypothesis that both samples come from a population with the same distribution.<br>".to_string().as_bytes())?;
        } else {
            writer.write_all("<strong>Result</strong>: The test <strong>fails to reject</strong> the null hypothesis that both samples come from a population with the same distribution.<br>".to_string().as_bytes())?;
        }

        writer.write_all("</p>".as_bytes())?;

        writer.write_all("<p><strong>Caveat</strong>: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, \nand statistical significance says nothing about the practical significance of a difference.</p>".to_string().as_bytes())?;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "X".to_string(),
            y_axis_label: "Cumulative Probability".to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: true,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;


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

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

fn calculate_p_value(alam: f64) -> f64 {
    let mut fac = 2.0f64;
    let mut sum = 0.0f64;
    let mut term: f64;
    let mut termbf = 0.0f64;
    let eps1 = 0.001f64;
    let eps2 = 1.0e-8f64;
    let a2 = -2.0 * alam * alam;
    for j in 1..= 100 {
        term = fac * (a2 * (j * j) as f64).exp();
        sum += term;
        if term.abs() <= eps1 * termbf || term.abs() <= eps2 * sum {
            return sum;
        }
        fac = -fac;
        termbf = term.abs();
    }
    return 1.0
}
