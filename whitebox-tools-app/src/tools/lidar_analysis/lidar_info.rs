/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/06/2017
Last Modified: 30/01/2022
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
// use whitebox_common::structures::Point3D;
use std;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;
use std::u16;
use whitebox_common::structures::{ Point2D, Point3D };
use whitebox_common::algorithms::{ convex_hull, polygon_area };

/// This tool can be used to print basic information about the data contained within a LAS file, used to store LiDAR
/// data. The reported information will include including data on the header, point return frequency, and classification
/// data and information about the variable length records (VLRs) and geokeys.
pub struct LidarInfo {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarInfo {
    pub fn new() -> LidarInfo {
        // public constructor
        let name = "LidarInfo".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Prints information about a LiDAR (LAS) dataset, including header, point return frequency, and classification data and information about the variable length records (VLRs) and geokeys.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Summary Report File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output HTML file for summary report.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Calculate the average point density and nominal point spacing?".to_owned(),
            flags: vec!["--density".to_owned()],
            description:
                "Flag indicating whether or not to calculate the average point density and nominal point spacing."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Print the variable length records (VLRs)?".to_owned(),
            flags: vec!["--vlr".to_owned()],
            description:
                "Flag indicating whether or not to print the variable length records (VLRs)."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Print the geokeys?".to_owned(),
            flags: vec!["--geokeys".to_owned()],
            description: "Flag indicating whether or not to print the geokeys.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las --vlr --geokeys\"
.*{0} -r={1} --wd=\"*path*to*data*\" -i=file.las",
            short_exe, name
        )
        .replace("*", &sep);

        LidarInfo {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarInfo {
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
        let mut input_file: String = "".to_string();
        let mut output_file = String::new();
        let mut show_vlrs = false;
        let mut show_geokeys = false;
        let mut keyval: bool;
        let mut show_density = false;
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
            keyval = false;
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
            } else if flag_val == "-density" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    show_density = true;
                }
            } else if flag_val == "-vlr" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    show_vlrs = true;
                }
            } else if flag_val == "-geokeys" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    show_geokeys = true;
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

        let sep = std::path::MAIN_SEPARATOR;
        // if !working_directory.ends_with(sep) {
        //     working_directory.push_str(&(sep.to_string()));
        // }

        if !input_file.contains(sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if output_file.len() == 0 {
            output_file = input_file.replace(".las", "_summary.html");
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>LAS File Summary</title>
            <style  type=\"text/css\">
                h1 {
                    font-size: 14pt;
                    margin-left: 15px;
                    margin-right: 15px;
                    text-align: center;
                    font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                }
                h2 {
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
                td, th {
                    text-align: left;
                    padding: 8px;
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
        <body>
            <h1>LAS File Summary</h1>
        ";
        writer.write_all(s.as_bytes())?;

        let input = LasFile::new(&input_file, "r")?;

        let s1 = &format!("<h2>File Summary</h2><p>{}", input);
        writer.write_all(s1.replace("\n", "<br>").as_bytes())?;

        let num_points = input.header.number_of_points;
        let mut min_i = u16::MAX;
        let mut max_i = u16::MIN;
        let mut intensity: u16;
        let mut num_first: i64 = 0;
        let mut num_last: i64 = 0;
        let mut num_only: i64 = 0;
        let mut num_intermediate: i64 = 0;
        let mut ret: u8;
        let mut nrets: u8;
        let mut pd: PointData;
        // let mut p: Point3D;
        let mut ret_array: [i32; 5] = [0; 5];
        let mut class_array: [i32; 256] = [0; 256];
        // read the points into a Vec<Point2D>
        let mut points: Vec<Point2D> = Vec::with_capacity(input.header.number_of_points as usize);
        let mut p: Point3D;
        for i in 0..input.header.number_of_points as usize {
            pd = input[i]; 
            p = input.get_transformed_coords(i);
            points.push(Point2D::new(p.x, p.y));
            ret = pd.return_number();
            if ret > 5 {
                // Return is too high
                ret = 5;
            }
            ret_array[(ret - 1) as usize] += 1;
            nrets = pd.number_of_returns();
            class_array[pd.classification() as usize] += 1;
            if nrets == 1 {
                num_only += 1;
            } else if ret == 1 && nrets > 1 {
                num_first += 1;
            } else if ret == nrets {
                num_last += 1;
            } else {
                num_intermediate += 1;
            }
            intensity = pd.intensity;
            if intensity > max_i {
                max_i = intensity;
            }
            if intensity < min_i {
                min_i = intensity;
            }
        }

        // println!("\n\nMin I: {}\nMax I: {}", min_i, max_i);
        let s1 = &format!(
            "<br>Min Intensity: {}<br>Max Intensity: {}</p>",
            min_i, max_i
        );
        writer.write_all(s1.as_bytes())?;

        s = "<h2>Point Returns Analysis</h2>";
        writer.write_all(s.as_bytes())?;

        // Point Return Table
        s = "<p><table>
        <caption>Point Return Table</caption>
        <tr>
            <th class=\"headerCell\">Return Value</th>
            <th class=\"headerCell\">Number</th>
            <th class=\"headerCell\">Percentage</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        for i in 0..5 {
            if ret_array[i] > 0 {
                let s1 = &format!(
                    "<tr>
                    <td>{}</td>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                </tr>\n",
                    i + 1,
                    ret_array[i],
                    format!("{:.1}%", ret_array[i] as f64 / num_points as f64 * 100f64)
                );
                writer.write_all(s1.as_bytes())?;
            }
        }

        s = "</table></p>";
        writer.write_all(s.as_bytes())?;

        // Point Return Table
        s = "<p><table>
        <caption>Point Position Table</caption>
        <tr>
            <th class=\"headerCell\">Return Position</th>
            <th class=\"headerCell\">Number</th>
            <th class=\"headerCell\">Percentage</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        let s1 = &format!(
            "<tr>
            <td>Only</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}%</td>
        </tr>\n",
            num_only,
            format!("{:.1}", num_only as f64 / num_points as f64 * 100f64)
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!(
            "<tr>
            <td>First</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}%</td>
        </tr>\n",
            num_first,
            format!("{:.1}", num_first as f64 / num_points as f64 * 100f64)
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!(
            "<tr>
            <td>Intermediate</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}%</td>
        </tr>\n",
            num_intermediate,
            format!(
                "{:.1}",
                num_intermediate as f64 / num_points as f64 * 100f64
            )
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!(
            "<tr>
            <td>Last</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}%</td>
        </tr>\n",
            num_last,
            format!("{:.1}", num_last as f64 / num_points as f64 * 100f64)
        );
        writer.write_all(s1.as_bytes())?;

        s = "</table></p>";
        writer.write_all(s.as_bytes())?;

        // Point Classification Table
        s = "<p><table>
        <caption>Point Classification Table</caption>
        <tr>
            <th class=\"headerCell\">Classification</th>
            <th class=\"headerCell\">Number</th>
            <th class=\"headerCell\">Percentage</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        for i in 0..256 {
            if class_array[i] > 0 {
                let percent: f64 = class_array[i] as f64 / num_points as f64 * 100.0;
                let percent_str = format!("{:.*}", 1, percent);
                let class_string = convert_class_val_to_class_string(i as u8);
                let s1 = &format!(
                    "<tr>
                    <td>{} - {}</td>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}%</td>
                </tr>\n",
                    i, class_string, class_array[i], percent_str
                );
                writer.write_all(s1.as_bytes())?;
            }
        }

        s = "</table></p>";
        writer.write_all(s.as_bytes())?;

        if show_density {
            let hull_points = convex_hull(&mut points);
            let area = polygon_area(&hull_points);
            let density = (input.header.number_of_points as f64) / area;
            let spacing = 1f64 / density.sqrt();

            let s1 = &format!("<p>Average point density: {:.3} pts / m<sup>2</sup><br>Nominal point spacing: {:.4} m</p>", density, spacing);
            writer.write_all(s1.as_bytes()).expect("Error writing to file.");
        }

        if show_vlrs {
            s = "<h2>Variable Length Records</h2>";
            writer.write_all(s.as_bytes())?;
            if input.header.number_of_vlrs > 0 {
                for i in 0..(input.header.number_of_vlrs as usize) {
                    let s1 = &format!("<p>VLR {}:<br>{}</p>", i, input.vlr_data[i].clone());
                    writer.write_all(s1.as_bytes())?;
                }
            } else {
                s = "<p>VLRs have not been set.</p>";
                writer.write_all(s.as_bytes())?;
            }
        }

        if show_geokeys {
            s = "<h2>Geokeys</h2>";
            writer.write_all(s.as_bytes())?;
            let s1 = &format!(
                "<p>{}</p>",
                input.geokeys.interpret_geokeys().replace("\n", "<br>")
            );
            writer.write_all(s1.as_bytes())?;
        }

        s = "</body>";
        writer.write_all(s.as_bytes())?;

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
