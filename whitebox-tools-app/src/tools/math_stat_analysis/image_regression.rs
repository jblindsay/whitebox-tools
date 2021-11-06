/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/09/2017
Last Modified: 18/10/2019
License: MIT
*/

use self::statrs::distribution::{FisherSnedecor, StudentsT, Univariate};
use whitebox_raster::*;
use whitebox_common::rendering::Scattergram;
use crate::tools::*;
use num_cpus;
use rand::prelude::*;
use statrs;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool performs a bivariate linear regression analysis on two input raster images. The first image
/// (`--i1`) is considered to be the independent variable while the second image (`--i2`) is considered to
/// be the dependent variable in the analysis. Both input images must share the same grid, as the coefficient
/// requires a comparison of a pair of images on a grid-cell-by-grid-cell basis. The tool will output an HTML
/// report (`--output`) summarizing the regression model, an Analysis of Variance (ANOVA), and the
/// significance of the regression coefficients. The regression residuals can optionally be output as a new
/// raster image (`--out_residuals`) and the user can also optionally specify to standardize the residuals
/// (`--standardize`).
///
/// Note that the analysis performs a linear regression; two variables may be strongly related by a non-linear
/// association (e.g. a power function curve) which will lead to an apparently weak fitting regression model.
/// In fact, non-linear relations are very common among spatial variables, e.g. terrain indices such as slope
/// and contributing area. In such cases, it is advisable that the input images are transformed prior to the
/// analysis.
///
/// **NoData** values in either of the two input images are ignored during the calculation of the correlation
/// between images.
///
/// # See Also
/// `ImageCorrelation`, `ImageCorrelationNeighbourhoodAnalysis`
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
        parameters.push(ToolParameter {
            name: "Independent Variable (X).".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input raster file (independent variable, X).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Dependent Variable (Y).".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input raster file (dependent variable, Y).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Summary Report File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output HTML file for regression summary report.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Optional Residuals Output File".to_owned(),
            flags: vec!["--out_residuals".to_owned()],
            description: "Output raster regression residual file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Standardize the residuals map?".to_owned(),
            flags: vec!["--standardize".to_owned()],
            description: "Optional flag indicating whether to standardize the residuals map."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output scattergram?".to_owned(),
            flags: vec!["--scattergram".to_owned()],
            description: "Optional flag indicating whether to output a scattergram.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Num. Samples For Scattergram".to_owned(),
            flags: vec!["--num_samples".to_owned()],
            description: "Number of samples used to create scattergram".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1000".to_string()),
            optional: false,
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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
        let mut residuals_file = String::new();
        let mut standardize_residuals = false;
        let mut output_residuals = false;
        let mut output_scattergram = false;
        let mut num_samples = 1000usize;

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
            } else if flag_val == "-out_residuals" {
                if keyval {
                    residuals_file = vec[1].to_string();
                } else {
                    residuals_file = args[i + 1].to_string();
                }
                output_residuals = true;
            } else if flag_val == "-standardize" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    standardize_residuals = true;
                }
            } else if flag_val == "-scattergram" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    output_scattergram = true;
                }
            } else if flag_val == "-num_samples" {
                num_samples = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
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

        let start = Instant::now();

        if verbose {
            println!("Loop 1 of 2...");
        }

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
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
                        x = input1.get_value(row, col);
                        y = input2.get_value(row, col);
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
            let (a, b, c, d, e, f) = rx.recv().expect("Error receiving data from thread.");
            sum_x += a;
            sum_y += b;
            sum_xy += c;
            sum_xx += d;
            sum_yy += e;
            n += f;
        }

        let slope = (n * sum_xy - (sum_x * sum_y)) / (n * sum_xx - (sum_x * sum_x));
        let intercept = (sum_y - slope * sum_x) / n;
        let r = (n * sum_xy - (sum_x * sum_y))
            / ((n * sum_xx - (sum_x * sum_x)).sqrt() * ((n * sum_yy - (sum_y * sum_y)).sqrt()));
        let r_sqr = r * r;
        let y_mean = sum_y / n;
        let x_mean = sum_x / n;

        if verbose {
            println!("Loop 2 of 2...");
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
            let (a, b) = rx.recv().expect("Error receiving data from thread.");
            ss_error += a;
            ss_total += b;
        }

        let df_reg = 1f64;
        let df_error = n - 2f64;
        let ss_reg = ss_total - ss_error;
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
        let intercept_pvalue = 2f64 * (1.0 - t.cdf(intercept.abs() / intercept_se));

        let slope_se = (msse / sum_xx).sqrt();
        let slope_t = slope / slope_se;
        let slope_pvalue = 2f64 * (1f64 - t.cdf(slope.abs() / slope_se));

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
            output.configs.data_type = DataType::F32;
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(row, data);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Creating residuals image: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!(
                "Elapsed Time (excluding I/O): {}",
                get_formatted_elapsed_time(start)
            ));

            if verbose {
                println!("Saving data...")
            };
            let _ = match output.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
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
        let s1 = &format!(
            "<p><strong>Input Image 1 (independent variable, X):</strong> {}</p>",
            x_filename.clone()
        );
        writer.write_all(s1.as_bytes())?;
        let path = Path::new(&input_file2);
        let y_filename = path.file_name().unwrap().to_str().unwrap();
        let s1 = &format!(
            "<p><strong>Input Image 2 (dependent variable, Y):</strong> {}</p><br>",
            y_filename.clone()
        );
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

        let s1 = &format!(
            "<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
            format!("{:.4}", r),
            format!("{:.4}", r_sqr),
            format!("{:.4}", se_of_estimate)
        );
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

        let s1 = &format!(
            "<tr>
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
            format!("{:.4}", p_value)
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!(
            "<tr>
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
            format!("{:.4}", ms_error)
        );
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!(
            "<tr>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
        </tr>\n",
            "Total",
            format!("{:.4}", ss_total)
        );
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

        let s1 = &format!(
            "<tr>
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

        let s1 = &format!(
            "<tr>
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

        let sign = if intercept < 0f64 { "-" } else { "+" };
        let s2 = &format!(
            "<p><strong>Regression equation:</strong> {} = {} &#215; {} {} {}</p>",
            input2.get_short_filename(),
            slope,
            input1.get_short_filename(),
            sign.clone(),
            intercept.abs()
        );
        writer.write_all(s2.as_bytes())?;

        s = "<p>Caveat: Given a sufficiently large sample, extremely weak and non-notable relations can be found to be statistically significant
            and statistical significance says nothing about the practical significance of a relation.</p>";
        writer.write_all(s.as_bytes())?;

        if output_scattergram {
            let mut xdata = vec![];
            let mut ydata = vec![];
            let mut series_xdata = vec![];
            let mut series_ydata = vec![];
            let mut series_names = vec![];
            let mut rng = thread_rng();
            let mut sample_num = 0usize;
            let (mut x, mut y): (f64, f64);
            while sample_num < num_samples {
                let row = rng.gen_range(0, rows as isize);
                let col = rng.gen_range(0, columns as isize);
                x = input1.get_value(row, col);
                y = input2.get_value(row, col);
                if x != nodata1 && y != nodata2 {
                    sample_num += 1;
                    series_xdata.push(x);
                    series_ydata.push(y);
                }
            }

            xdata.push(series_xdata.clone());
            ydata.push(series_ydata.clone());
            series_names.push(String::from("Series 1"));

            let graph = Scattergram {
                parent_id: "scattergram".to_string(),
                data_x: xdata.clone(),
                data_y: ydata.clone(),
                series_labels: series_names.clone(),
                x_axis_label: input1.get_short_filename(),
                y_axis_label: input2.get_short_filename(),
                width: 700f64,
                height: 500f64,
                draw_trendline: true,
                draw_gridlines: true,
                draw_legend: false,
                draw_grey_background: false,
            };

            writer.write_all(
                &format!(
                    "<div id='scattergram' align=\"center\">{}</div>",
                    graph.get_svg()
                )
                .as_bytes(),
            )?;
        }

        s = "</body>";
        writer.write_all(s.as_bytes())?;

        let _ = writer.flush();

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("\n{}", &format!("Elapsed Time: {}", elapsed_time));
        }

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
