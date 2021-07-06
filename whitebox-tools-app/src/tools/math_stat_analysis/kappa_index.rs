/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/09/2017
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use std::cmp::max;
use std::cmp::min;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::path;
use std::path::Path;
use std::process::Command;

/// This tool calculates the [Kappa index of agreement](https://en.wikipedia.org/wiki/Cohen%27s_kappa) (KIA), or
/// Cohen's Kappa, for two categorical input raster images (`--input1` and `--input2`). The KIA is a measure of inter-rater
/// reliability (i.e. classification accuracy) and is widely applied in many fields, notably remote sensing. For example,
/// The KIA is often used as a means of assessing the accuracy of an image classification analysis. The KIA
/// can be interpreted as the percentage improvement that the underlying classification has over and above a random
/// classifier (i.e. random assignment to categories). The user must specify the output HTML file (`--output`). The input
/// images must be of a categorical data type, i.e. contain classes. As a measure of classification accuracy, the
/// KIA is more robust than the *overall percent agreement* because it takes into account the agreement occurring by
/// chance. A KIA of 0 would indicate that the classifier is no better than random class assignment. In addition to the
/// KIA, this tool will also output the [producer's and user's accuracy](http://gis.humboldt.edu/OLM/Courses/GSP_216_Online/lesson6-2/metrics.html),
/// the overall accuracy, and the error matrix.
///
/// # See Also
/// `CrossTabulation`
pub struct KappaIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl KappaIndex {
    pub fn new() -> KappaIndex {
        // public constructor
        let name = "KappaIndex".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs a kappa index of agreement (KIA) analysis on two categorical raster files."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Classification File".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input classification raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Reference File".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input reference raster file.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=class.tif --i2=reference.tif -o=kia.html", short_exe, name).replace("*", &sep);

        KappaIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for KappaIndex {
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
            if vec[0].to_lowercase() == "-i1"
                || vec[0].to_lowercase() == "--i1"
                || vec[0].to_lowercase() == "--input1"
            {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2"
                || vec[0].to_lowercase() == "--i2"
                || vec[0].to_lowercase() == "--input2"
            {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        let mut progress: i32;
        let mut old_progress: i32 = -1;

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

        if verbose {
            println!("Reading data...")
        };

        let input1 = Raster::new(&input_file1, "r")?;
        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;
        let min1 = input1.configs.minimum.round() as i32;
        let max1 = input1.configs.maximum.round() as i32;

        let input2 = Raster::new(&input_file2, "r")?;
        if input2.configs.rows as isize != rows || input2.configs.columns as isize != columns {
            panic!("Error: The input files do not contain the same raster extent.");
        }
        let nodata2 = input2.configs.nodata;
        let min2 = input2.configs.minimum.round() as i32;
        let max2 = input2.configs.maximum.round() as i32;

        let min_val = min(min1, min2);
        let max_val = max(max1, max2);
        let range = (max_val - min_val) as usize + 1;

        let start = Instant::now();

        let mut error_matrix = vec![vec![0usize; range]; range];
        let mut active_class = vec![false; range];
        let mut z1: f64;
        let mut z2: f64;
        let (mut class1, mut class2): (usize, usize);
        for row in 0..rows {
            for col in 0..columns {
                z1 = input1.get_value(row, col);
                z2 = input2.get_value(row, col);
                if z1 != nodata1 && z2 != nodata2 {
                    class1 = (z1 - min_val as f64).round() as usize;
                    class2 = (z2 - min_val as f64).round() as usize;
                    error_matrix[class1][class2] += 1;
                    active_class[class1] = true;
                    active_class[class2] = true;
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

        let mut num_classes = 0;
        for a in 0..range as usize {
            if active_class[a] {
                num_classes += 1;
            }
        }

        let mut agreements = 0usize;
        let mut expected_frequency = 0f64;
        let mut n = 0usize;
        let mut row_total: usize;
        let mut col_total: usize;
        let kappa: f64;
        let overall_accuracy: f64;

        for a in 0..range as usize {
            agreements += error_matrix[a][a];
            for b in 0..range as usize {
                n += error_matrix[a][b];
            }
        }

        for a in 0..range as usize {
            row_total = 0;
            col_total = 0;
            for b in 0..range as usize {
                col_total += error_matrix[a][b];
                row_total += error_matrix[b][a];
            }
            expected_frequency += (col_total as f64 * row_total as f64) / (n as f64);
        }

        kappa = (agreements as f64 - expected_frequency as f64)
            / (n as f64 - expected_frequency as f64);
        overall_accuracy = agreements as f64 / n as f64;

        let mut f = File::create(output_file.as_str()).unwrap();

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Lidar Kappa Index of Agreement</title>
            <style  type=\"text/css\">
                h1 {
                    font-size: 14pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: center;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                h3 {
                    font-size: 12pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: left;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                p, ol, ul, li {
                    font-size: 12pt;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    margin-left: 15px;
                    margin-right: 15px;
                }
                caption {
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    font-size: 12pt;
                    margin-left: 15px;
                    margin-right: 15px;
                }
                table {
                    font-size: 12pt;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                    font-family: arial, sans-serif;
                    border-collapse: collapse;
                    align: center;
                }
                td {
                    text-align: left;
                    padding: 8px;
                }
                th {
                    text-align: left;
                    padding: 8px;
                    background-color: #ffffff;
                    border-bottom: 1px solid #333333;
                    text-align: center;
                }
                tr:nth-child(1) {
                    border-bottom: 1px solid #333333;
                    border-top: 2px solid #333333;
                }
                tr:last-child {
                    border-bottom: 2px solid #333333;
                }
                tr:nth-child(even) {
                    background-color: #dddddd;
                }
                .numberCell {
                    text-align: right;
                }
                .headerCell {
                    text-align: center;
                }
            </style>
        </head>
        <body>";
        f.write(s.as_bytes()).unwrap();
        s = "<body><h1>Kappa Index of Agreement</h1>";
        f.write(s.as_bytes()).unwrap();
        let path = Path::new(&input_file1);
        let s1 = &format!(
            "<p><strong>Classification Data:</strong> {}</p>",
            path.file_name().unwrap().to_str().unwrap()
        );
        f.write_all(s1.as_bytes())?;
        let path = Path::new(&input_file2);
        let s1 = &format!(
            "<p><strong>Reference Data:</strong> {}</p><br>",
            path.file_name().unwrap().to_str().unwrap()
        );
        f.write_all(s1.as_bytes())?;
        // let s2 = &format!("{}{}{}{}{}", "<p><b>Input Data:</b> <br><br><b>Classification Data:</b> ", input_file1, "<br><br><b>Reference Data:</b> ", input_file2, "<p>");
        // f.write(s2.as_bytes()).unwrap();

        s = "<br><table>";
        f.write(s.as_bytes()).unwrap();
        s = "<caption>Contingency Table</caption>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        let s3 = &format!(
            "{}{}{}",
            "<th colspan=\"2\" rowspan=\"2\"></th><th colspan=\"",
            num_classes,
            "\">Reference Data</th><th rowspan=\"2\">Row<br>Totals</th>"
        );
        f.write(s3.as_bytes()).unwrap();
        s = "</tr>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        for a in 0..range as usize {
            if active_class[a] {
                let s = &format!("{}{}{}", "<th>", (a as usize), "</th>");
                f.write(s.as_bytes()).unwrap();
            }
        }

        s = "</tr>";
        f.write(s.as_bytes()).unwrap();
        let mut first_entry = true;
        for a in 0..range as usize {
            if active_class[a] {
                if first_entry {
                    let s = format!(
                        "{}{}{}{}{}",
                        "<tr><td rowspan=\"",
                        num_classes,
                        "\" valign=\"center\"><b>Class<br>Data</b></td> <td><b>",
                        (a as usize),
                        "</b></td>"
                    );
                    f.write(s.as_bytes()).unwrap();
                } else {
                    let s = format!("{}{}{}", "<tr><td><b>", (a as usize), "</b></td>");
                    f.write(s.as_bytes()).unwrap();
                }
                row_total = 0;
                for b in 0..range as usize {
                    if active_class[b] {
                        row_total += error_matrix[a][b];
                        let s = format!(
                            "{}{}{}",
                            "<td class=\"numberCell\">", error_matrix[a][b], "</td>"
                        );
                        f.write(s.as_bytes()).unwrap();
                    }
                }
                let s = format!("{}{}{}", "<td class=\"numberCell\">", row_total, "</td>");
                f.write(s.as_bytes()).unwrap();

                let s2 = "</tr>";
                f.write(s2.as_bytes()).unwrap();
                first_entry = false;
            }
        }
        s = "<tr>";
        f.write(s.as_bytes()).unwrap();
        s = "<th colspan=\"2\">Column Totals</th>";
        f.write(s.as_bytes()).unwrap();
        for a in 0..range as usize {
            if active_class[a] {
                col_total = 0;
                for b in 0..range as usize {
                    if active_class[b] {
                        col_total += error_matrix[b][a];
                    }
                }
                let s = &format!("{}{}{}", "<td  class=\"numberCell\">", col_total, "</td>");
                f.write(s.as_bytes()).unwrap();
            }
        }

        let s4 = &format!(
            "{}{}{}",
            "<td class=\"numberCell\"><b>N</b>=", n, "</td></tr>"
        );
        f.write(s4.as_bytes()).unwrap();
        s = "</table>";
        f.write(s.as_bytes()).unwrap();
        s = "<br><br><table>";
        f.write(s.as_bytes()).unwrap();
        s = "<caption>Class Statistics</caption>";
        f.write(s.as_bytes()).unwrap();
        s = "<tr><th class=\"headerCell\">Class</th><th class=\"headerCell\">User's Accuracy<sup>1</sup><br>(Reliability)</th><th class=\"headerCell\">Producer's Accuracy<sup>1</sup><br>(Accuracy)</th></tr>";
        f.write(s.as_bytes()).unwrap();

        let mut average_producers = 0.0;
        let mut average_users = 0.0;
        let mut num_active = 0.0;
        for a in 0..range as usize {
            if active_class[a] {
                num_active += 1.0;
                let mut row_total = 0;
                let mut col_total = 0;
                for b in 0..range as usize {
                    if active_class[b] {
                        col_total += error_matrix[a][b];
                        row_total += error_matrix[b][a];
                    }
                }
                average_users += 100.0 * error_matrix[a][a] as f64 / col_total as f64;
                average_producers += 100.0 * error_matrix[a][a] as f64 / row_total as f64;
                let s = &format!(
                    "{}{}{}{}{}{}{}",
                    "<tr><td>",
                    (a as usize),
                    "</td><td class=\"numberCell\">",
                    format!(
                        "{:.*}",
                        2,
                        (100.0 * error_matrix[a][a] as f64 / col_total as f64)
                    ),
                    "%</td><td class=\"numberCell\">",
                    format!(
                        "{:.*}",
                        2,
                        (100.0 * error_matrix[a][a] as f64 / row_total as f64)
                    ),
                    "%</td></tr>"
                );
                f.write(s.as_bytes()).unwrap();
            }
        }
        f.write(format!("<tr><td>Average</td><td class=\"numberCell\">{}%</td><td class=\"numberCell\">{}%</td></tr>", format!("{:.*}", 2, average_users / num_active),
                format!("{:.*}", 2, average_producers / num_active)).as_bytes()).unwrap();

        s = "</table>";
        f.write(s.as_bytes()).unwrap();
        let s6 = &format!(
            "<p>{}{}</p>",
            "<p><b>Overall Accuracy</b> = ",
            format!("{:.*}%", 2, overall_accuracy * 100.0)
        );
        f.write(s6.as_bytes()).unwrap();
        let s7 = &format!(
            "<p><b>Kappa</b><sup>2</sup> = {}</p>",
            format!("{:.*}", 3, kappa)
        );
        f.write(s7.as_bytes()).unwrap();
        let s5 = &format!("{}{}", "<p><br>Notes:<br>1. User's accuracy refers to the proportion of points correctly assigned to a class (i.e. the number of points correctly classified for a category divided by the row total in the contingency table) and is a measure of the reliability. ",
                "Producer's accuracy is a measure of the proportion of the points in each category correctly classified (i.e. the number of points correctly classified for a category divided by the column total in the contingency table) and is a measure of the accuracy.<br>");
        f.write(s5.as_bytes()).unwrap();
        f.write("<br>2. Cohen's kappa coefficient is a statistic that measures inter-rater agreement for qualitative (categorical)
        items. It is generally thought to be a more robust measure than simple percent agreement calculation, since
        kappa takes into account the agreement occurring by chance. Kappa measures the percentage of data values in the
        main diagonal of the contingency table and then adjusts these values for the amount of agreement that could be expected due
        to chance alone.</p>".as_bytes()).unwrap();
        s = "</body>";
        f.write_all(s.as_bytes())?;

        let _ = f.flush();

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
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
