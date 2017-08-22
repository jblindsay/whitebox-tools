/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: August 15, 2017
Last Modified: August 15, 2017
License: MIT

NOTES: This index (C) is taken from Lindsay (2006) Sensitivity of channel mapping techniques to uncertainty in digital
elevation data: 

C = SS_mp ∕ SS_B = [∑(pij − p-bar)^2] ∕ [ ∑pij(1 − p-bar)^2 + p2(RC − ∑pij)]

Please note that there is an error in the original published equation. Specifically, the denominator
read:

∑pij(1 - p_bar)^2 + p_bar^2 (RC - ∑pij)

instead of the original:

∑pij(1 - p_bar^2) - p_bar^2 (RC - ∑pij)
*/
extern crate time;
extern crate num_cpus;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::process::Command;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct CrispnessIndex {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl CrispnessIndex {
    pub fn new() -> CrispnessIndex {
        // public constructor
        let name = "CrispnessIndex".to_string();

        let description = "Calculates the Crispness Index, which is used to quantify how crisp (or conversely how fuzzy) a probability image is.".to_string();

        let mut parameters = "-i, --input       Input raster file.".to_owned();
        parameters.push_str("-o, --output   Optional output html file (default name will be based on input file if unspecified).\n");

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
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=input.dep
>>.*{0} -r={1} --wd=\"*path*to*data*\" -o=crispness.html",
                            short_exe,
                            name)
                .replace("*", &sep);

        CrispnessIndex {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CrispnessIndex {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
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
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if output_file.len() == 0 {
            // output_file not specified and should be based on input file
            let p = path::Path::new(&input_file);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            output_file = input_file.replace(&extension, ".html");
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        //if verbose { println!("Calculating image mean and standard deviation...") };
        //let (mean, stdev) = input.calculate_mean_and_stdev();

        // calculate the number of downslope cells
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut n = 0;
                    let mut s = 0.0;
                    let mut warning = false;
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            if z < 0f64 || z > 1f64 {
                                warning = true;
                            }
                            n += 1;
                            s += z;
                        }
                    }
                    tx.send((n, s, warning)).unwrap();
                }
            });
        }

        let mut num_cells = 0;
        let mut sum = 0.0;
        let mut warning = false;
        for row in 0..rows {
            let (a, b, c) = rx.recv().unwrap();
            num_cells += a;
            sum += b;
            if c {
                warning = true;
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mean = sum / num_cells as f64;

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut total_dev = 0f64;
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            total_dev += (z - mean) * (z - mean);
                        }
                    }
                    tx.send(total_dev).unwrap();
                }
            });
        }

        let mut total_dev = 0f64;
        for row in 0..rows {
            total_dev += rx.recv().unwrap();

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let denominator = sum * (1f64 - mean) * (1f64 - mean) +
                          (num_cells as f64 - sum) * mean * mean;
        let crispness = total_dev / denominator;

        let end = time::now();
        let elapsed_time = end - start;

        if warning {
            println!("WARNING: This tool is intended to be applied to membership probability (MP) rasters, with probability values 
ranging from 0-1. The input image contains values outside this range.");
        }

        // println!("\nNumber of non-nodata grid cells: {}", num_cells);
        // println!("Image average: {}", mean);
        // println!("SSmp: {}", total_dev);
        // println!("SSb: {}", denominator);
        // println!("Crispness index: {}", crispness);

        println!("\n{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));


        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Crispness Index</title>
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
                    text-align: left;
                    padding: 8px;
                }
                tr:nth-child(even) {
                    background-color: #dddddd;
                }
                .numberCell {
                    text-align: right;
                }
            </style>
        </head>
        <body>
            <h1>Crispness Index Report</h1>
        ".as_bytes())?;

        writer
            .write_all(format!("<p><strong>Input file</strong>: {}</p>", input_file).as_bytes())?;

        if warning {
            writer.write_all("<p><strong>WARNING</strong>: This tool is intended to be applied to membership probability (MP) rasters, with probability values 
ranging from 0-1. The input image contains values outside this range. <em>Therefore, it is unlikely that the results are meaningful</em>.</p>".as_bytes())?;
        }


        writer
            .write_all("<br><table align=\"center\">".as_bytes())?;

        writer
            .write_all(&format!("<tr>
            <td><em>SS<sub>mp</sub></em></td>
            <td class=\"numberCell\">{}</td>
        </tr>",
                                format!("{:.*}", 4, total_dev))
                                .as_bytes())?;

        writer
            .write_all(&format!("<tr>
            <td><em>SS<sub>B</sub></em></td>
            <td class=\"numberCell\">{}</td>
        </tr>",
                                format!("{:.*}", 4, denominator))
                                .as_bytes())?;

        writer
            .write_all(&format!("<tr>
            <td><em>C</em><sup>1</sup></td>
            <td class=\"numberCell\">{}</td>
        </tr>",
                                format!("{:.*}", 4, crispness))
                                .as_bytes())?;

        writer.write_all("</table>".as_bytes())?;

        writer.write_all("<p><sup>1</sup><em>C</em> = <em>SS</em><sub>mp</sub> &#8725; <em>SS</em><sub>B</sub> = 
        [&sum;<sup><em>R</em></sup><sub><em>i</em>=1</sub>&sum;<sup><em>C</em></sup><sub><em>j</em>=1</sub>(<em>p<sub>ij
        </sub></em> &minus; <span style=\"text-decoration: overline\"><em>p</em></span>)<sup>2</sup>] &#8725; [ 
            &sum;<em>p<sub>ij</sub></em>(1 &minus; <span style=\"text-decoration: overline\"><em>p</em></span>)<sup>2
            </sup> + <span style=\"text-decoration: overline\"><em>p</em></span><sup>2</sup>(<em>RC</em> &minus; 
            &sum;<em>p<sub>ij</sub></em>)]</p>".as_bytes())?;

        writer.write_all("<p>Where <em>C</em> is the crispness index, <em>SS</em><sub>mp</sub> is the sum of the squares 
        for the membership probability image, <em>SS</em><sub>B</sub> is the sum of the squares for the Boolean case where 
        the total probability (summed for the image) is arranged crisply, <em>R</em> and <em>C</em> are the number of rows 
        and columns in the image respectively, <span style=\"text-decoration: overline\"><em>p</em></span> is the image 
        average probability value, &sum;<em>p<sub>ij</sub></em> is the image total, and <em>i</em> and <em>j</em> refer to a cell within the image.</p>".as_bytes())?;

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

            println!("Complete! Please see {} for output.", output_file);
        }

        Ok(())
    }
}