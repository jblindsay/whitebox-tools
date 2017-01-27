// #![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;

use std::io::Error;
use std::io::ErrorKind;
use std::process::Command;
use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::env;
use std::path;
use std::path::Path;
use std::f64;
use std::f64::consts::PI;
use whitebox_tools::raster::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut feature_file = String::new();
    let mut working_directory: String = "".to_string();
    let mut verbose: bool = false;
    let mut keyval: bool;
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        keyval = false;
        if vec.len() > 1 { keyval = true; }
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
        } else if vec[0].to_lowercase() == "-wd" || vec[0].to_lowercase() == "--wd" {
            if keyval {
                working_directory = vec[1].to_string();
            } else {
                working_directory = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-v" || vec[0].to_lowercase() == "--verbose" {
            verbose = true;
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
            vec[0].to_lowercase() == "--h"{
            let mut s: String = "Help:\n".to_owned();
                     s.push_str("-i          Input raster file.\n");
                     s.push_str("-o          Output HTML file.\n");
                     s.push_str("-features   Feature definition (or class) raster.\n");
                     s.push_str("-wd         Optional working directory. If specified, filenames parameters need not include a full path.\n");
                     s.push_str("-version    Prints the tool version number.\n");
                     s.push_str("-h          Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*anova -wd *path*to*data* -i input.tif -features classes.tif -o anova.html -v\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("slope v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    if !working_directory.ends_with(&sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file.contains(&sep) {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }
    if !output_file.ends_with(".html") {
        output_file = output_file + ".html";
    }
    if !feature_file.contains(&sep) {
        feature_file = format!("{}{}", working_directory, feature_file);
    }

    match anova(input_file, feature_file, output_file, verbose) {
        Ok(()) => println!("Complete!"),
        Err(err) => panic!("Error: {}", err),
    }
}

fn anova(input_file: String, feature_file: String, output_file: String, verbose: bool) ->  Result<(), Error> {
    if verbose {
        println!("********************");
        println!("* Welcome to anova *");
        println!("********************");
    }

    println!("Reading data...");
    let input =  Raster::new(&input_file, "r")?;
    let nodata = input.configs.nodata;
    let columns = input.configs.columns;
    let rows = input.configs.rows;

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

    let features = match Raster::new(&feature_file, "r") {
        Ok(f) => f,
        Err(err) => panic!("Error: {}", err),
    };
    let nodata_features = features.configs.nodata;
    if features.configs.columns != columns || features.configs.rows != rows {
        panic!(Error::new(ErrorKind::InvalidInput, "The input and feature definition rasters must have the same number of rows and columns."));
    }
    // How many features/classes are there?
    let mut z: f64;
    let mut id: f64;
    let mut id_int: i32;
    let mut vec_id: usize;
    let mut min_id = i32::max_value();
    let mut max_id = i32::min_value();
    let mut overall_sum = 0f64;
    let mut overall_sum_sqr = 0f64;
    //let mut overall_k = -9999f64;
    let mut overall_n = 0usize;
    let mut progress: usize;
    let mut old_progress: usize = 1;
    for row in 0..rows as isize {
        for col in 0..columns as isize {
            z = input.get_value(row, col);
            id = features.get_value(row, col);
            if z != nodata && id != nodata_features {
                id_int = id.round() as i32;
                if id_int > max_id { max_id = id_int; }
                if id_int < min_id { min_id = id_int; }

                // overall statistics
                // if overall_k == -9999f64 {
                //     overall_k = z;
                // }
                overall_n += 1;
                overall_sum += z; // - overall_k;
                overall_sum_sqr += z * z; //(z - overall_k) * (z - overall_k);
            }
        }
        if verbose {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (loop 1 of 2): {}%", progress);
                old_progress = progress;
            }
        }
    }

    let range = (max_id - min_id + 1) as usize;
    let overall_mean = overall_sum / overall_n as f64; //overall_k + overall_sum / overall_n as f64;
    let overall_variance = (overall_sum_sqr - (overall_sum * overall_sum)/ overall_n as f64) / (overall_n as f64 - 1f64);
    let ss_t = overall_sum_sqr - overall_n as f64 * overall_mean * overall_mean;
    // let ss_t = overall_sum_sqr - overall_n as f64 * (overall_mean - overall_k) * (overall_mean - overall_k);

    let mut n = vec![0usize; range];
    let mut sum = vec![0f64; range];
    let mut sum_sqr = vec![0f64; range];
    //let mut k = vec![-9999f64; range];
    let mut mean = vec![0f64; range];
    let mut variance = vec![0f64; range];
    for row in 0..rows as isize {
        for col in 0..columns as isize {
            z = input.get_value(row, col);
            id = features.get_value(row, col);
            if z != nodata && id != nodata_features {
                // class statistics
                id_int = id.round() as i32;
                vec_id = (id_int - min_id) as usize;
                // if k[vec_id] == -9999f64 {
                //     k[vec_id] = z;
                // }
                n[vec_id] += 1;
                sum[vec_id] += z; // - k[vec_id];
                sum_sqr[vec_id] += z * z; //(z - k[vec_id]) * (z - k[vec_id]);
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
            mean[i] = sum[i] / n[i] as f64; // k[i] + sum[i] / n[i] as f64;
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

    let output = Command::new("open")
                     .arg(output_file)
                     .output()
                     .expect("failed to execute process");

    let _ = output.stdout;

    Ok(())
}

// fn calculate_p_value() {
//     let fx = 7.12;
//     let ndf = 4;
//     let ddf = 34;
//     let p1 = f_call(f_spin(fx, ndf, ddf));
//     if p1 < 0.0001 || p1 > 1.0 {
//       p1 = 0.0001;
//     }
// }

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
