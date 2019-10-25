/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/10/2019
Last Modified: 24/10/2019
License: MIT
*/

use crate::raster::*;
use crate::rendering::html::*;
use crate::rendering::LineGraph;
use crate::tools::*;
use rand::prelude::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::cmp::Ordering::Equal;

/// This tool will perform a paired-sample t-test to evaluate whether a significant 
/// statistical difference exists between the two rasters. The user must 
/// specify the name of the two input raster images (`--input1` and `--input2`) and the output report
/// HTML file (`--output`). The test can be performed optionally on the entire image or on a random 
/// sub-sample of pixel values of a user-specified size (`--num_samples`). In evaluating the significance 
/// of the test, it is important to keep in mind that given a sufficiently large sample, extremely small and
/// non-notable differences can be found to be statistically significant. Furthermore
/// statistical significance says nothing about the practical significance of a difference.
/// 
/// # See Also
/// `TwoSampleKSTest`
pub struct PairedSampleTTest {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PairedSampleTTest {
    pub fn new() -> PairedSampleTTest {
        // public constructor
        let name = "PairedSampleTTest".to_string();
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

        PairedSampleTTest {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PairedSampleTTest {
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

        let input1 = Arc::new(Raster::new(&input_file1, "r")?);
        let input1_name = input1.get_short_filename();
        let input2 = Arc::new(Raster::new(&input_file2, "r")?);
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
        let mut n = 0;
        let mean: f64;
        let variance: f64;
        let std_dev: f64;

        let mut data1: Vec<f64> = Vec::with_capacity(num_samples);
        let mut data2: Vec<f64> = Vec::with_capacity(num_samples);
        let mut diffs: Vec<f64> = Vec::with_capacity((rows * columns) as usize);

        if num_samples == 0 {
            let num_procs = num_cpus::get() as isize;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input1 = input1.clone();
                let input2 = input2.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z1: f64;
                    let mut z2: f64;
                    let mut diff: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut n = 0;
                        let mut s = 0.0;
                        let mut sq = 0.0;
                        let mut diffs: Vec<f64> = Vec::with_capacity(columns as usize);
                        for col in 0..columns {
                            z1 = input1.get_value(row, col);
                            z2 = input2.get_value(row, col);
                            if z1 != nodata1 && z2 != nodata2 {
                                n += 1;
                                diff = z2 - z1;
                                s += diff;
                                sq += diff * diff;
                                diffs.push(diff);
                            }
                        }
                        tx.send((n, s, sq, diffs)).unwrap();
                    }
                });
            }

            let mut sum = 0.0;
            let mut sq_sum = 0.0;
            for row in 0..rows {
                let (a, b, c, d) = rx.recv().unwrap();
                n += a;
                sum += b;
                sq_sum += c;
                for i in 0..d.len() {
                    diffs.push(d[i]);
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            mean = sum / n as f64;
            variance = sq_sum / n as f64 - mean * mean;
            std_dev = variance.sqrt();

        } else {
            // Note that this is sampling with replacement, which is not ideal.
            diffs = Vec::with_capacity(num_samples);
            let mut z1: f64;
            let mut z2: f64;
            let mut diff: f64;
            let mut sum = 0.0;
            let mut sq_sum = 0.0;
            let mut rng = thread_rng();
            let (mut row, mut col): (isize, isize);
            let mut sample_num = 0usize;
            while sample_num < num_samples {
                row = rng.gen_range(0, rows as isize);
                col = rng.gen_range(0, columns as isize);
                z1 = input1.get_value(row, col);
                z2 = input2.get_value(row, col);
                if z1 != nodata1 && z2 != nodata2 {
                    diff = z2 - z1;
                    n += 1;
                    sum += diff;
                    sq_sum += diff * diff;
                    sample_num += 1;
                    data1.push(z1);
                    data2.push(z2);
                    diffs.push(z2 - z1);

                    if verbose {
                        progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }

            mean = sum / n as f64;
            variance = sq_sum / n as f64 - mean * mean;
            std_dev = variance.sqrt();
        }

        let std_err = std_dev / (n as f64).sqrt();
        let t = mean / std_err; 
        let p_value = calc_p_value(t, n-1);

        // let's see the cdf of differences compared with a normal
        diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut series_names = vec![];
        let num_bins = 100usize;
        let min_val = diffs[0];
        let max_val = diffs[n-1];
        let bin_size = (max_val - min_val) / num_bins as f64;
        let mut bin: usize;
        let profile_xdata = (0..num_bins).map(|x| min_val + x as f64 * bin_size).collect::<Vec<f64>>();
        let mut profile_ydata = vec![0f64; num_bins];
        // bin frequency data
        for val in &diffs {
            bin = ((val - min_val) / bin_size).floor() as usize;
            if bin > num_bins -1 { bin = num_bins - 1; }
            profile_ydata[bin] += 1f64;
        }
        for bin in 1..num_bins {
            profile_ydata[bin] += profile_ydata[bin-1];
        }
        for bin in 0..num_bins {
            profile_ydata[bin] /= n as f64;
        }
        xdata.push(profile_xdata.clone());
        ydata.push(profile_ydata.clone());
        series_names.push(String::from("Paired Differences"));

        let mut normal_dist = vec![0f64; num_bins];
        let sd_root2pi = std_dev * (2f64 * f64::consts::PI).sqrt();
        let two_sd_sqr = 2f64 * std_dev * std_dev;
        let mut z: f64;
        for i in 0..num_bins {
            z = min_val + i as f64 * bin_size;
            normal_dist[i] = 1f64 / sd_root2pi * ((-(z - mean) * (z - mean)) / two_sd_sqr).exp();
        }
        for i in 1..num_bins {
            normal_dist[i] = normal_dist[i - 1] + normal_dist[i];
        }

        for i in 1..num_bins {
            normal_dist[i] = normal_dist[i] / normal_dist[num_bins - 1];
        }

        xdata.push(profile_xdata.clone());
        ydata.push(normal_dist.clone());
        series_names.push(String::from("Normal Dist."));

        
        ///////////////////////
        // Output the report //
        ///////////////////////
        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Paired-Samples t-Test</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"
            </head>
            <body>
                <h1>Paired-Samples <em>t</em>-Test Report</h1>
                <p>"#
                .as_bytes(),
        )?;

        writer.write_all(&format!("<strong>Input image 1</strong>: {}<br>", input1_name.clone()).as_bytes())?;
        writer.write_all(&format!("<strong>Input image 2</strong>: {}<br>", input2_name.clone()).as_bytes())?;

        if num_samples < 25 && num_samples > 0 {
            // Point Return Table
            let s = "<p><table>
            <caption>Data Table</caption>
            <tr>
                <th class=\"headerCell\">Image 1</th>
                <th class=\"headerCell\">Image 2</th>
                <th class=\"headerCell\">Diff</th>
            </tr>";
            writer.write_all(s.as_bytes())?;

            for i in 0..num_samples {
                let s1 = &format!(
                    "<tr>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                </tr>\n",
                    format!("{:.4}", data1[i]),
                    format!("{:.4}", data2[i]),
                    format!("{:.4}", diffs[i])
                );
                writer.write_all(s1.as_bytes())?;
            }

            writer.write_all("</table>".as_bytes())?;
        }

        writer.write_all(&format!("<strong>Sample mean of the differences</strong>: {:.4}<br>", mean).as_bytes())?;
        writer.write_all(&format!("<strong>Sample size (n)</strong>: {:.0}<br>", n).as_bytes())?;
        writer.write_all(&format!("<strong>Sample standard deviation of the differences</strong>: {:.4}<br>", std_dev).as_bytes())?;
        writer.write_all(&format!("<strong>Estimated standard error of the mean</strong>: {:.4}<br>", std_err).as_bytes())?;
        writer.write_all(&format!("<strong>Test Statistic (<em>t</em>)</strong>: {:.4}<br>", t).as_bytes())?;
        if p_value > 0.001f64 {
            writer.write_all(
                &format!(
                    "<strong>Two-tailed Significance (<em>p</em>-value)</strong>: {:.4}<br>",
                    p_value
                )
                .as_bytes(),
            )?;
        } else {
            writer.write_all(
                "<strong>Two-tailed Significance (<em>p</em>-value)</strong>: <0.001<br>"
                    .to_string()
                    .as_bytes(),
            )?;
        }
        if p_value < 0.05 {
            writer.write_all("<strong>Result</strong>: The test <strong>rejects</strong> the null hypothesis that the difference between the paired population means is equal to 0.<br>".to_string().as_bytes())?;
        } else {
            writer.write_all("<strong>Result</strong>: The test <strong>fails to reject</strong> the null hypothesis that the difference between the paired population means is equal to 0.<br>".to_string().as_bytes())?;
        }

        writer.write_all("</p>".as_bytes())?;

        writer.write_all("<p><strong>Caveats</strong>: <ol>
        <li>Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, and statistical significance says nothing about the practical significance of a difference.</li> 
        <li>The presence of spatial autocorrelation implies a lack of independence in the data sample, which violates the assumptions of the <em>t</em>-Test, potentially affecting the reliability of the results.</li>
        <li>Care should be taken to ensure that the distribution of paired-sample differences are normally distributed (see below).</li></ol></p>".to_string().as_bytes())?;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 550f64,
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

// The following formulation has been based on a javascript translation from: 
// https://www.easycalculation.com/statistics/p-value-t-test.php
fn calc_p_value(t: f64, df: usize) -> f64 {
    if df == 0 {
        panic!("Error: degrees of freedom (df) must be non-zero.");
    }
    let abst = t.abs();
    let tsq = t*t;
    let p = match df {
        1 => 1f64 - 2f64 * abst.atan() / f64::consts::PI,
        2 => 1f64 - abst / (tsq + 2f64).sqrt(),
        3 => 1f64 - 2f64 * ((abst / 3f64.sqrt()).atan() + abst * 3f64.sqrt() / (tsq + 3f64)) / f64::consts::PI,
        4 => 1f64 - abst * (1f64 + 2f64 / (tsq + 4f64)) / (tsq + 4f64).sqrt(),
        _ => {
            let z = t_to_z(abst, df);
            norm_p(z)
        }
    };
  return p;
}

fn t_to_z(t: f64, df: usize) -> f64 {
    let a9 = df as f64 - 0.5;
    let b9 = 48f64 * a9 * a9;
    let t9 = t * t / df as f64;
    let z8 = if t9 >= 0.04f64 {
        a9 * (1f64 + t9).ln()
    } else {
        a9 * (((1f64 - t9 * 0.75f64) * t9 / 3f64 - 0.5f64) * t9 + 1f64) * t9
    };
    let p7 = ((0.4f64 * z8 + 3.3f64) * z8 + 24f64) * z8 + 85.5f64;
    let b7 = 0.8f64 * z8.powf(2f64) + 100f64 + b9;
    let z = (1f64 + (-p7 / b7 + z8 + 3f64) / b9) * z8.sqrt();
    return z;
}

fn norm_p(z: f64) -> f64 {
    let absz = z.abs();
    const A1: f64 = 0.0000053830;
    const A2: f64 = 0.0000488906;
    const A3: f64 = 0.0000380036;
    const A4: f64 = 0.0032776263;
    const A5: f64 = 0.0211410061;
    const A6: f64 = 0.0498673470;
    let mut p = (((((A1 * absz + A2) * absz + A3) * absz + A4) * absz + A5) * absz + A6) * absz + 1f64;
    p = p.powf(-16f64);
    return p;
}