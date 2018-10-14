/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: Dec. 18, 2017
Last Modified: 12/10/2018
License: MIT
*/

use raster::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;
use tools::*;

pub struct CrossTabulation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CrossTabulation {
    pub fn new() -> CrossTabulation {
        // public constructor
        let name = "CrossTabulation".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs a cross-tabulation on two categorical images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File 1".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input raster file 1.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input File 2".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input raster file 1.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output HTML file (default name will be based on input file if unspecified)."
                    .to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=\"file1.tif\" --i2=\"file2.tif\" -o=outfile.html",
                            short_exe, name).replace("*", &sep);

        CrossTabulation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CrossTabulation {
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
        let mut input_file1: String = String::new();
        let mut input_file2: String = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i1" || flag_val == "-input1" {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i + 1].to_string();
                }
            } else if flag_val == "-i2" || flag_val == "-input2" {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if !input_file1.contains(&sep) && !input_file1.contains("/") {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) && !input_file2.contains("/") {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input1 = Raster::new(&input_file1, "r")?;
        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;

        let input2 = Raster::new(&input_file2, "r")?;
        let nodata2 = input2.configs.nodata;

        let min1 = input1.configs.minimum.round() as isize;
        let min2 = input2.configs.minimum.round() as isize;

        let max1 = input1.configs.maximum.round() as isize;
        let max2 = input2.configs.maximum.round() as isize;

        let image1_range = (max1 - min1) as usize + 1;
        let image2_range = (max2 - min2) as usize + 1;

        let mut contingency_table = vec![vec![0; image2_range]; image1_range];
        let mut class_exists1 = vec![false; image1_range];
        let mut class_exists2 = vec![false; image2_range];

        let mut z1: f64;
        let mut z2: f64;
        let mut index1: usize;
        let mut index2: usize;
        for row in 0..rows {
            for col in 0..columns {
                z1 = input1.get_value(row, col);
                z2 = input2.get_value(row, col);
                if z1 != nodata1 && z2 != nodata2 {
                    index1 = (z1.round() as isize - min1) as usize;
                    index2 = (z2.round() as isize - min2) as usize;
                    class_exists1[index1] = true;
                    class_exists2[index2] = true;
                    contingency_table[index1][index2] += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating contingency table: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Cross Tabulation</title>
            <style  type=\"text/css\">
                h1 {
                    font-size: 14pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: center;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                p {
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
                td, th {
                    border: 1px solid #222222;
                    text-align: centre;
                    padding: 8px;
                }
                tr:nth-child(even) {
                    background-color: #dddddd;
                }
                .numberCell {
                    text-align: right;
                }
                .header {
                    font-weight: bold;
                    text-align: center;
                }
            </style>
        </head>
        <body>
            <h1>Cross Tabulation Report</h1> ".as_bytes())?;

        writer.write_all(
            &format!(
                "<p><strong>Image 1</strong> (columns): {}</p>",
                input_file1.clone()
            ).as_bytes(),
        )?;
        writer.write_all(
            &format!(
                "<p><strong>Image 2</strong> (rows): {}</p>",
                input_file2.clone()
            ).as_bytes(),
        )?;

        // output the table.
        writer.write_all("<div><table align=\"center\">".as_bytes())?;
        writer.write_all("<caption>Cross Tabulation Results</caption>".as_bytes())?;

        let mut s = String::from("<tr><td></td>");
        for a in 0..image1_range {
            if class_exists1[a] {
                s.push_str(&format!("<td class=\"header\">{}</td>", a as isize + min1));
            }
        }
        s.push_str("</tr>");
        writer.write_all(s.as_bytes())?;

        for b in 0..image2_range {
            if class_exists2[b] {
                let mut s = format!("<tr><td class=\"header\">{}</td>", b as isize + min2);
                for a in 0..image1_range {
                    if class_exists1[a] {
                        s.push_str(&format!(
                            "<td class=\"numberCell\">{}</td>",
                            contingency_table[a][b]
                        ));
                    }
                }
                s.push_str("</tr>");
                writer.write_all(s.as_bytes())?;
            }
        }
        writer.write_all("</table></div>".as_bytes())?;
        writer.write_all("</body>".as_bytes())?;

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
            if verbose {
                println!("Complete! Please see {} for output.", output_file);
            }
        }

        Ok(())
    }
}
