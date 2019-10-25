/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/10/2019
Last Modified: 24/10/2019
License: MIT
*/

use self::statrs::distribution::{Normal, Univariate};
use crate::raster::*;
use crate::rendering::html::*;
use crate::tools::*;
use rand::prelude::*;
use statrs;
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
                        for col in 0..columns {
                            z1 = input1.get_value(row, col);
                            z2 = input2.get_value(row, col);
                            if z1 != nodata1 && z2 != nodata2 {
                                n += 1;
                                diff = z2 - z1;
                                s += diff;
                                sq += diff * diff;
                            }
                        }
                        tx.send((n, s, sq)).unwrap();
                    }
                });
            }

            let mut sum = 0.0;
            let mut sq_sum = 0.0;
            for row in 0..rows {
                let (a, b, c) = rx.recv().unwrap();
                n += a;
                sum += b;
                sq_sum += c;
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
        let distribution = Normal::new(0.0, 1.0).unwrap();
        let p_value = 2f64 * (1f64 - distribution.cdf(t.abs()));
        
        ///////////////////////
        // Output the report //
        ///////////////////////
        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Paired-Sample t-Test</title>"#.as_bytes())?;

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
        writer.write_all(&format!("<strong>Sample mean of the differences</strong>: {:.4}<br>", mean).as_bytes())?;
        writer.write_all(&format!("<strong>Sample size (n)</strong>: {:.0}<br>", n).as_bytes())?;
        writer.write_all(&format!("<strong>Sample standard deviation of the differences</strong>: {:.4}<br>", std_dev).as_bytes())?;
        writer.write_all(&format!("<strong>Estimated standard error of the mean</strong>: {:.4}<br>", std_err).as_bytes())?;
        writer.write_all(&format!("<strong>Test Statistic (<em>t</em>)</strong>: {:.4}<br>", t).as_bytes())?;
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
            writer.write_all("<strong>Result</strong>: The test <strong>rejects</strong> the null hypothesis that the difference between the paired population means is equal to 0.<br>".to_string().as_bytes())?;
        } else {
            writer.write_all("<strong>Result</strong>: The test <strong>fails to reject</strong> the null hypothesis that the difference between the paired population means is equal to 0.<br>".to_string().as_bytes())?;
        }

        writer.write_all("</p>".as_bytes())?;

        writer.write_all("<p><strong>Caveat</strong>: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, \nand statistical significance says nothing about the practical significance of a difference.</p>".to_string().as_bytes())?;

        // let graph = LineGraph {
        //     parent_id: "graph".to_string(),
        //     width: 700f64,
        //     height: 500f64,
        //     data_x: xdata.clone(),
        //     data_y: ydata.clone(),
        //     series_labels: series_names.clone(),
        //     x_axis_label: "X".to_string(),
        //     y_axis_label: "Cumulative Probability".to_string(),
        //     draw_points: false,
        //     draw_gridlines: true,
        //     draw_legend: true,
        //     draw_grey_background: false,
        // };

        // writer.write_all(
        //     &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        // )?;


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
