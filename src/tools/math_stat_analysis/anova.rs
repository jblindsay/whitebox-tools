/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: September 23, 2017
Last Modified: November 16, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::env;
use std::path;
use std::path::Path;
use std::f64;
use std::f64::consts::PI;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct Anova {
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Anova {
    pub fn new() -> Anova { // public constructor
        let name = "Anova".to_string();
        
        let description = "Performs an analysis of variance (ANOVA) test on a raster dataset.".to_string();
        
        // let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        // parameters.push_str("--features    Feature definition (or class) raster.\n");
        // parameters.push_str("-o, --output  Output HTML file.\n");

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
            name: "Feature Definition (Class) File".to_owned(), 
            flags: vec!["--features".to_owned()], 
            description: "Feature definition (or class) raster.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output HTML File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output HTML file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false
        });
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} --wd=\"*path*to*data*\" -i=data.tif --features=classes.tif -o=anova.html", short_exe, name).replace("*", &sep);
    
        Anova { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for Anova {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut feature_file = String::new();
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
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-features" || vec[0].to_lowercase() == "--features" {
                if keyval {
                    feature_file = vec[1].to_string();
                } else {
                    feature_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
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
        if !feature_file.contains(&sep) {
            feature_file = format!("{}{}", working_directory, feature_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !output_file.ends_with(".html") {
            output_file = output_file + ".html";
        }

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>ANOVA</title>
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
            <h1>One-way ANOVA test</h1>
        ";
        writer.write_all(s.as_bytes())?;


        let path = Path::new(&input_file);
        let s1 = &format!("<p><strong>Measurement variable:</strong> {}</p>", path.file_name().unwrap().to_str().unwrap());
        writer.write_all(s1.as_bytes())?;
        let path = Path::new(&feature_file);
        let s1 = &format!("<p><strong>Nominal variable:</strong> {}</p><br>", path.file_name().unwrap().to_str().unwrap());
        writer.write_all(s1.as_bytes())?;

        let features = Arc::new(Raster::new(&feature_file, "r")?);
        let nodata_features = features.configs.nodata;
        if features.configs.columns as isize != columns || features.configs.rows as isize != rows {
            panic!(Error::new(ErrorKind::InvalidInput, "The input and feature definition rasters must have the same number of rows and columns."));
        }
        // How many features/classes are there?
        // let mut z: f64;
        // let mut id: f64;
        // let mut id_int: i32;
        let mut vec_id: usize;
        
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let features = features.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut id: f64;
                let mut id_int: i32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut overall_sum = 0f64;
                    let mut overall_sum_sqr = 0f64;
                    let mut overall_n = 0usize;
                    let mut min_id = i32::max_value();
                    let mut max_id = i32::min_value();
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        id = features.get_value(row, col);
                        if z != nodata && id != nodata_features {
                            id_int = id.round() as i32;
                            if id_int > max_id { max_id = id_int; }
                            if id_int < min_id { min_id = id_int; }
                            overall_n += 1;
                            overall_sum += z;
                            overall_sum_sqr += z * z;
                        }
                    }
                    tx.send((overall_n, overall_sum, overall_sum_sqr, min_id, max_id)).unwrap();
                }
            });
        }

        let mut overall_sum = 0f64;
        let mut overall_sum_sqr = 0f64;
        let mut overall_n = 0usize;
        let mut min_id = i32::max_value();
        let mut max_id = i32::min_value();
        for row in 0..rows {
            let (a, b, c, d, e) = rx.recv().unwrap();
            overall_n += a;
            overall_sum += b;
            overall_sum_sqr += c;
            if d < min_id { min_id = d; }
            if e > max_id { max_id = e; }
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let range = (max_id - min_id + 1) as usize;
        let overall_mean = overall_sum / overall_n as f64;
        let overall_variance = (overall_sum_sqr - (overall_sum * overall_sum)/ overall_n as f64) / (overall_n as f64 - 1f64);
        let ss_t = overall_sum_sqr - overall_n as f64 * overall_mean * overall_mean;
        
        let mut n = vec![0usize; range];
        let mut sum = vec![0f64; range];
        let mut sum_sqr = vec![0f64; range];
        let mut mean = vec![0f64; range];
        let mut variance = vec![0f64; range];
        let mut z: f64;
        let mut id: f64;
        let mut id_int: i32;
        for row in 0..rows as isize {
            for col in 0..columns as isize {
                z = input.get_value(row, col);
                id = features.get_value(row, col);
                if z != nodata && id != nodata_features {
                    // class statistics
                    id_int = id.round() as i32;
                    vec_id = (id_int - min_id) as usize;
                    n[vec_id] += 1;
                    sum[vec_id] += z;
                    sum_sqr[vec_id] += z * z;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut num_classes = 0;
        for i in 0..range {
            if n[i] > 0 {
                num_classes += 1;
                mean[i] = sum[i] / n[i] as f64;
                variance[i] = (sum_sqr[i] - (sum[i] * sum[i])/ n[i] as f64) / (n[i] as f64 - 1f64);
            }
        }

        // between-groups sum of squares
        let mut ss_b = 0f64;
        for i in 0..range {
            if n[i] > 0 {
                ss_b += n[i] as f64 * (mean[i] - overall_mean) * (mean[i] - overall_mean);
            }
        }

        let mut ss_w = overall_sum_sqr;
        for i in 0..range {
            if n[i] > 0 {
                ss_w -= sum[i] * sum[i] / n[i] as f64;
            }
        }

        let df_b = num_classes - 1;
        let df_w = overall_n - num_classes;
        let df_t = overall_n - 1;
        let ms_b = ss_b / df_b as f64;
        let ms_w = ss_w / df_w as f64;
        let f = ms_b / ms_w;

        println!("Calclating the p-value...");
        // Formula Used:
        // P value = [ 1 / Β(ndf/2,ddf/2) ] × [ (ndf × x) / (ndf × x + ddf) ]^ndf/2 × [1 - (ndf × x) / (ndf × x + ddf) ]^ddf/2 × x-1
        // Where,
        // Β - Beta Function
        // x - Critical Value
        // ndf - Numerator Degrees of Freedom
        // ddf - Denominator Degrees of Freedom
        // from: https://www.easycalculation.com/statistics/f-test-p-value.php
        let p = f_call(f_spin(f, df_b, df_w));

        println!("Saving output...");



        s = "<br><table align=\"center\">
        <caption>Group Summaries</caption>
        <tr>
            <th class=\"headerCell\">Group</th>
            <th class=\"headerCell\">N</th>
            <th class=\"headerCell\">Mean</th>
            <th class=\"headerCell\">St. Dev.</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        for i in 0..range {
            if n[i] > 0 {
                let s1 = &format!("<tr>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                    <td class=\"numberCell\">{}</td>
                </tr>\n",
                i as i32 + min_id,
                n[i],
                format!("{:.*}", 4, mean[i]),
                format!("{:.*}", 4, variance[i].sqrt()));
                writer.write_all(s1.as_bytes())?;
            }
        }

        let s1 = &format!("<tr>
            <td class=\"numberCell\">Overall</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        overall_n,
        format!("{:.*}", 4, overall_mean),
        format!("{:.*}", 4, overall_variance.sqrt()));
        writer.write_all(s1.as_bytes())?;

        s = "</table>";
        writer.write_all(s.as_bytes())?;


        s = "<br><br><table align=\"center\">
        <caption>ANOVA Table</caption>
        <tr>
            <th class=\"headerCell\">Source of<br>Variation</th>
            <th class=\"headerCell\">Sum of<br>Squares</th>
            <th class=\"headerCell\">df</th>
            <th class=\"headerCell\">Mean Square<br>Variance</th>
            <th class=\"headerCell\">F</th>
            <th class=\"headerCell\">p</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        let p_str;
        if p == 0.0 {
            p_str = String::from("< .0001");
        } else if p > 0.01 {
            p_str = format!("{:.*}", 4, p);
        } else {
            p_str = format!("{:.e}", p as f32);
        }

        let s1 = &format!("<tr>
            <td class=\"numberCell\">Between groups</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
        </tr>\n",
        format!("{:.*}", 3, ss_b),
        df_b,
        format!("{:.*}", 3, ms_b),
        format!("{:.*}", 3, f),
        p_str);
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">Within groups</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
        </tr>\n",
        format!("{:.*}", 3, ss_w),
        df_w,
        format!("{:.*}", 3, ms_w));
        writer.write_all(s1.as_bytes())?;

        let s1 = &format!("<tr>
            <td class=\"numberCell\">Total variation</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\">{}</td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
            <td class=\"numberCell\"></td>
        </tr>\n",
        format!("{:.*}", 3, ss_t),
        df_t);
        writer.write_all(s1.as_bytes())?;

        s = "</table>";
        writer.write_all(s.as_bytes())?;

        if p < 0.05 {
            let p_str;
            if p > 0.0001 {
                p_str = format!("={:.*}", 5, p);
            } else {
                p_str = format!("< 0.0001");
            }
            let s1 = &format!("<br><br>
            <h3>Interpretation:</h3>
            <p>The null hypothesis states that the means of the measurement variable are the same for the different categories of data;
            the alternative hypothesis states that they are not all the same.
            The analysis showed that the category means were significantly heterogeneous (one-way anova, F<sub>&alpha;=0.05, df1={}, df2={}</sub>={}, p{}), i.e.
            using an &alpha; of 0.05 the null hypothesis should be <strong>rejected</strong>.</p>
            <p>Caveat: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant
            and statistical significance says nothing about the practical significance of a difference.</p>
            <h3>Assumptions:</h3>
            <p>The ANOVA test has important assumptions that must be satisfied in order for the associated p-value to
            be valid:
            <ol>
            <li>The samples are independent.</li>
            <li>Each sample is from a normally distributed population.</li>
            <li>The population standard deviations of the groups are all equal.
                This property is known as homoscedasticity.</li>
            </ol>",
            df_b,
            df_w,
            format!("{:.*}", 3, f),
            p_str);
            writer.write_all(s1.as_bytes())?;
        } else {
            let s1 = &format!("<br><br>
            <h3>Interpretation:</h3>
            <p>The null hypothesis states that the means of the measurement variable are the same for the different categories of data;
            the alternative hypothesis states that they are not all the same.
            The analysis showed that the category means were not significantly different (one-way anova, F<sub>&alpha;=0.05, df1={}, df2={}</sub>={}, p{}), i.e.
            using an &alpha; of 0.05 the null hypothesis should be <strong>accepted</strong>.</p>
            <p>Caveat: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant
            and statistical significance says nothing about the practical significance of a difference.</p>
            <h3>Assumptions:</h3>
            <p>The ANOVA test has important assumptions that must be satisfied in order for the associated p-value to
            be valid:
            <ol>
            <li>The samples are independent.</li>
            <li>Each sample is from a normally distributed population.</li>
            <li>The population standard deviations of the groups are all equal.
                This property is known as homoscedasticity.</li>
            </ol>",
            df_b,
            df_w,
            format!("{:.*}", 3, f),
            format!("={:.*}", 3, p));
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

            println!("Complete! Please see {} for output.", output_file);
        }

        let end = time::now();
        let elapsed_time = end - start;

        println!("\n{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}

fn f_call(x: f64) -> f64 {
  let zk;
  if x >= 0.0 {
    zk = x + 0.0000005;
  } else {
    zk = x - 0.0000005;
  }
  zk
}

const PJ2: f64 = PI / 2.0;
fn f_spin(f: f64, df1: usize, df2: usize) -> f64 {
    let x = df2 as f64 / (df1 as f64 * f + df2 as f64);
    if (df1 as f64 % 2.0) == 0.0 {
      return lj_spin(1.0 - x, df2 as f64, df1 as f64 + df2 as f64 - 4.0, df2 as f64 - 2.0) * x.powf(df2 as f64 / 2.0);
    }
    if (df2 as f64 % 2.0) == 0.0 {
      return 1.0 - lj_spin(x, df1 as f64, df1 as f64 + df2 as f64 - 4.0, df1 as f64 - 2.0) * (1.0 - x).powf(df1 as f64 / 2.0);
    }
    let tan = ((df1 as f64 * f / df2 as f64).sqrt()).atan();
    let mut a = tan / PJ2;
    let sat = tan.sin();
    let cot = tan.cos();
    if df2 as f64 > 1.0 {
      a += sat * cot * lj_spin(cot * cot, 2.0, df2 as f64 - 3.0, -1.0 ) / PJ2;
    }
    if df1 == 1 {
      return 1.0 - a;
    }
    let mut c = 4.0 * lj_spin(sat * sat, df2 as f64 + 1.0, df1 as f64 + df2 as f64 - 4.0, df2 as f64 - 2.0) * sat * cot.powf(df2 as f64) / PI;
    if df2 == 1 {
      return 1.0 - a + c / 2.0;
    }
    let mut k = 2.0;
    while k <= (df2 as f64 - 1.0) / 2.0 {
      c = c * k / (k - 0.5);
      k = k + 1.0;
    }
    return 1.0 - a + c;
}

fn lj_spin(q: f64, i: f64, j: f64, b: f64) -> f64 {
   let mut zz = 1.0;
   let mut z = zz;
   let mut k = i;
   while k <= j {
     zz = zz * q * k / (k - b);
     z = z + zz;
     k = k + 2.0;
   }
   z
}
