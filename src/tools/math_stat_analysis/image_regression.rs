/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 27, 2017
Last Modified: December 15, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;
extern crate statrs;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::env;
use std::path;
use std::path::Path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::process::Command;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;
use self::statrs::distribution::{FisherSnedecor, StudentsT, Univariate};

pub struct ImageRegression {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImageRegression {
    pub fn new() -> ImageRegression {
        let name = "ImageRegression".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs image regression analysis on two input images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Independent Variable (X).".to_owned(), 
            flags: vec!["--i1".to_owned(), "--input1".to_owned()], 
            description: "Input raster file (independent variable, X).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Dependent Variable (Y).".to_owned(), 
            flags: vec!["--i2".to_owned(), "--input2".to_owned()], 
            description: "Input raster file (dependent variable, Y).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output Summary Report File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output HTML file for regression summary report.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Optional Residuals Output File".to_owned(), 
            flags: vec!["--out_residuals".to_owned()], 
            description: "Output raster regression resdidual file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Standardize the residuals map?".to_owned(), 
            flags: vec!["--standardize".to_owned()], 
            description: "Optional flag indicating whether to standardize the residuals map.".to_owned(),
            parameter_type: ParameterType::Boolean,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1='file1.tif' --i2='file2.tif' -o='outfile.html' --out_residuals='residuals.tif' --standardize",
                            short_exe, name).replace("*", &sep);

        ImageRegression {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImageRegression {
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
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
        let mut residuals_file = String::new();
        let mut standardize_residuals = false;
        let mut output_residuals = false;

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
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-out_residuals" || vec[0].to_lowercase() == "--out_residuals" {
                if keyval {
                    residuals_file = vec[1].to_string();
                } else {
                    residuals_file = args[i + 1].to_string();
                }
                output_residuals = true;
            } else if vec[0].to_lowercase() == "-standardize" || vec[0].to_lowercase() == "--standardize" {
                standardize_residuals = true;
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

        if !input_file1.contains(&sep) && !input_file1.contains("/") {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }

        if !input_file2.contains(&sep) && !input_file2.contains("/") {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }

        if output_file.len() == 0 {
            // output_file not specified and should be based on input file
            let p = path::Path::new(&input_file1);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            output_file = input_file1.replace(&extension, ".html");
        }
        if !output_file.ends_with(".html") {
            output_file = output_file + ".html";
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input1 = Arc::new(Raster::new(&input_file1, "r")?);
        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;
        
        let input2 = Arc::new(Raster::new(&input_file2, "r")?);
        if input2.configs.rows as isize != rows || input2.configs.columns as isize != columns {
            panic!("Error: The input files do not contain the same raster extent.");
        }
        let nodata2 = input2.configs.nodata;

        let start = time::now();
        
        if verbose { println!("Loop 1 of 2..."); }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input1 = input1.clone();
            let input2 = input2.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut sum_x = 0f64;
                let mut sum_y = 0f64;
                let mut sum_xy = 0f64;
                let mut sum_xx = 0f64;
                let mut sum_yy = 0f64;
                let mut n = 0f64;
                let mut x: f64;
                let mut y: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        x = input1[(row, col)];
                        y = input2[(row, col)];
                        if x != nodata1 && y != nodata2 {
                            sum_x += x;
                            sum_y += y;
                            sum_xy += x * y;
                            sum_xx += x * x;
                            sum_yy += y * y;
                            n += 1f64;
                        }
                    }
                }
                tx.send((sum_x, sum_y, sum_xy, sum_xx, sum_yy, n)).unwrap();
            });
        }

        let mut sum_x = 0f64;
        let mut sum_y = 0f64;
        let mut sum_xy = 0f64;
        let mut sum_xx = 0f64;
        let mut sum_yy = 0f64;
        let mut n = 0f64;
        for _ in 0..num_procs {
            let (a, b, c, d, e, f) = rx.recv().unwrap();
            sum_x += a;
            sum_y += b;
            sum_xy += c;
            sum_xx += d;
            sum_yy += e;
            n += f;
        }

        let slope = (n * sum_xy - (sum_x * sum_y)) / (n * sum_xx - (sum_x * sum_x));
        let intercept = (sum_y - slope * sum_x) / n;
        let r = (n * sum_xy - (sum_x * sum_y)) / (((n * sum_xx - (sum_x * sum_x)).sqrt() * ((n * sum_yy - (sum_y * sum_y)).sqrt())));
        let r_sqr = r * r;
        let y_mean = sum_y / n;
        let x_mean = sum_x / n;
        
        if verbose { println!("Loop 2 of 2..."); }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input1 = input1.clone();
            let input2 = input2.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut x: f64;
                let mut y: f64;
                let mut y_estimate: f64;
                let mut ss_total = 0f64;
                let mut ss_error = 0f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        x = input1[(row, col)];
                        y = input2[(row, col)];
                        if x != nodata1 && y != nodata2 {
                            y_estimate = slope * x + intercept;
                            ss_error += (y - y_estimate) * (y - y_estimate);
                            ss_total += (y - y_mean) * (y - y_mean);
                        }
                    }
                }
                tx.send((ss_error, ss_total)).unwrap();
            });
        }

        let mut ss_error = 0f64;
        let mut ss_total = 0f64;
        for _ in 0..num_procs {
            let (a, b) = rx.recv().unwrap();
            ss_error += a;
            ss_total += b;
        }

        let df_reg = 1f64;
        let df_error = n - 2f64;
        let ss_reg =  ss_total - ss_error;
        let ms_reg = ss_reg / df_reg;
        let ms_error = ss_error / df_error;
        let f_stat = ms_reg / ms_error;
        let se_of_estimate = ms_error.sqrt();
            
        //FDistribution f = new FDistribution(1, dfError);
        let f = FisherSnedecor::new(1f64, df_error).unwrap();
        let p_value = 1.0 - f.cdf(f_stat);
        let msse = (0f64.max(sum_yy - sum_xy * sum_xy / sum_xx)) / (n - 2f64);
        let intercept_se = (msse * ((1f64 / n) + (x_mean * x_mean) / sum_xx)).sqrt();
        let intercept_t = intercept / intercept_se;

        // TDistribution distribution = new TDistribution(N - 2);
        let t = StudentsT::new(0.0, 1.0, n - 2f64).unwrap();
        let intercept_pvalue =  2f64 * (1.0 - t.cdf(intercept.abs() / intercept_se));
        
        let slope_se = (msse / sum_xx).sqrt();
        let slope_t = slope / slope_se;
        let slope_pvalue =  2f64 * (1f64 - t.cdf(slope.abs() / slope_se));
        
        if output_residuals {
            if !residuals_file.contains(&sep) {
                residuals_file = format!("{}{}", working_directory, residuals_file);
            }
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input1 = input1.clone();
                let input2 = input2.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut x: f64;
                    let mut y: f64;
                    let mut y_estimate: f64;
                    let mut residual: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data: Vec<f64> = vec![nodata1; columns as usize];
                        for col in 0..columns {
                            x = input1[(row, col)];
                            y = input2[(row, col)];
                            if x != nodata1 && y != nodata2 {
                                y_estimate = slope * x + intercept;
                                residual = if standardize_residuals { 
                                    (y - y_estimate) / se_of_estimate
                                } else {
                                    y - y_estimate
                                };
                                data[col as usize] = residual;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&residuals_file, &input1);
            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                output.set_row_data(row, data);
                
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Creating residuals image: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            
            if verbose { println!("Saving data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };
        }
        
        let end = time::now();
        let elapsed_time = end - start;

        if verbose { println!("\n{}",  &format!("Elapsed Time: {}", elapsed_time).replace("PT", "")); }


        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Image Regression</title>
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
            <h1>Image Regression</h1>
        ";
        writer.write_all(s.as_bytes())?;


        let path = Path::new(&input_file1);
        let x_filename = path.file_name().unwrap().to_str().unwrap();
        let s1 = &format!("<p><strong>Input Image 1 (independent variable, X):</strong> {}</p>", x_filename.clone());
        writer.write_all(s1.as_bytes())?;
        let path = Path::new(&input_file2);
        let y_filename = path.file_name().unwrap().to_str().unwrap();
        let s1 = &format!("<p><strong>Input Image 2 (dependent variable, Y):</strong> {}</p><br>", y_filename.clone());
        writer.write_all(s1.as_bytes())?;

        // Model summary
        s = "<br><table>
        <caption>Model Summary</caption>
        <tr>
            <th class=\"headerCell\">R</th>
            <th class=\"headerCell\">R Square</th>
            <th class=\"headerCell\">Std. Error of the Estimate</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        format!("{:.4}", r),
        format!("{:.4}", r_sqr),
        format!("{:.4}", se_of_estimate));
        writer.write_all(s1.as_bytes())?;

        s = "</table>";
        writer.write_all(s.as_bytes())?;


        // ANOVA table
        s = "<br><br><table>
        <caption>Analysis of Variance (ANOVA)</caption>
        <tr>
            <th class=\"headerCell\">Source</th>
            <th class=\"headerCell\">SS</th>
            <th class=\"headerCell\">df</th>
            <th class=\"headerCell\">MS</th>
            <th class=\"headerCell\">F</th>
            <th class=\"headerCell\">p</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        "Regression",
        format!("{:.4}", ss_reg),
        format!("{:.4}", df_reg),
        format!("{:.4}", ms_reg),
        format!("{:.4}", f_stat),
        format!("{:.4}", p_value));
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
        </tr>\n",
        "Residual",
        format!("{:.4}", ss_error),
        format!("{:.4}", df_error),
        format!("{:.4}", ms_error));
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
        </tr>\n",
        "Total",
        format!("{:.4}", ss_total));
        writer.write_all(s1.as_bytes())?;

        s = "</table>";
        writer.write_all(s.as_bytes())?;


        // Regression coefficients
        s = "<br><br><table>
        <caption>Coefficients</caption>
        <tr>
            <th class=\"headerCell\">Variable</th>
            <th class=\"headerCell\">B</th>
            <th class=\"headerCell\">Std. Error</th>
            <th class=\"headerCell\">t</th>
            <th class=\"headerCell\">Sig.</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        "Constant",
        format!("{:.4}", intercept),
        format!("{:.4}", intercept_se),
        format!("{:.4}", intercept_t),
        format!("{:.4}", intercept_pvalue)
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        "Slope",
        format!("{:.4}", slope),
        format!("{:.4}", slope_se),
        format!("{:.4}", slope_t),
        format!("{:.4}", slope_pvalue)
        );
        writer.write_all(s1.as_bytes())?;

        s = "</table>";
        writer.write_all(s.as_bytes())?;

        let sign = if intercept < 0f64 {
            "-"
        } else {
            "+"
        };
        let s2 = &format!("<p><strong>Regression equation:</strong> {} = {} &#215; {} {} {}</p>", y_filename.clone(), slope, x_filename.clone(), sign.clone(), intercept.abs());
        writer.write_all(s2.as_bytes())?;

        s = "<p>Caveat: Given a sufficiently large sample, extremely weak and non-notable relations can be found to be statistically significant
            and statistical significance says nothing about the practical significance of a difference.</p>";
        writer.write_all(s.as_bytes())?;

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

            println!("Complete! Please see {} for output.", output_file);
        }

        Ok(())
    }
}