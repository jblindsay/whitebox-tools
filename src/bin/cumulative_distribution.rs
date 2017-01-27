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
use std::f64;
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
                     s.push_str("-features   Optional feature definition (or class) raster.\n");
                     s.push_str("-wd         Optional working directory. If specified, filenames parameters need not include a full path.\n");
                     s.push_str("-version    Prints the tool version number.\n");
                     s.push_str("-h          Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*cumulative_distribution -wd *path*to*data* -i input.tif -features classes.tif -o distros.html -v\n".replace("*", &sep));
                     s.push_str(&">> .*cumulative_distribution -wd *path*to*data* -i input.tif -o distros.html -v\n".replace("*", &sep));
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
    if !feature_file.contains(&sep) && feature_file.len() != 0 {
        feature_file = format!("{}{}", working_directory, feature_file);
    }

    let _ = cumulative_distribution(input_file, feature_file, output_file, verbose);
}

fn cumulative_distribution(input_file: String, feature_file: String, output_file: String, verbose: bool) ->  Result<(), Error> {
    if verbose {
        println!("**************************************");
        println!("* Welcome to cumulative_distribution *");
        println!("**************************************");
    }

    println!("Reading data...");
    let input = match Raster::new(&input_file, "r") {
        Ok(f) => f,
        Err(err) => panic!("Error: {}", err),
    };
    let nodata = input.configs.nodata;
    let columns = input.configs.columns;
    let rows = input.configs.rows;

    // now calculate the percentiles and output them.
    let f = File::create(output_file.clone())?;
    let mut writer = BufWriter::new(f);

    let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
    <head>
        <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
        <title>Cumulative Distributions</title>
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
        <h1>Cumulative Distributions</h1>
    ";
    writer.write_all(s.as_bytes())?;

    if feature_file.len() > 0 {
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
        if range > 255 {
            panic!(Error::new(ErrorKind::InvalidInput, "The feature definition raster has greater than 255 features/classes."));
        }

        //println!("min {} max {} range {}", min_id, max_id, range);

        let mut data: Vec<Vec<f64>> = vec![vec![]; range];
        let mut n = vec![0usize; range];

        let mut progress: usize;
        let mut old_progress: usize = 1;
        for row in 0..rows as isize {
            for col in 0..columns as isize {
                z = input.get_value(row, col);
                id = features.get_value(row, col);
                if z != nodata && id != nodata_features {
                    id_int = id.round() as i32;
                    vec_id = (id_int - min_id) as usize;
                    data[vec_id].push(z);
                    n[vec_id] += 1;
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

        // sort the vectors
        println!("Sorting the data...");
        for i in 0..range {
            if n[i] > 0 {
                data[i].sort_by(|a, b| a.partial_cmp(b).unwrap());
            }
        }

        for i in 0..range {
            if n[i] > 5 {

                s = "<br><table align=\"center\">
                <tr>
                    <th>Percentile</th>
                    <th>Value</th>
                </tr>";
                writer.write_all(s.as_bytes())?;
                let s1 = &format!("\n<br><br><caption>Feature/Class: {} (n={})</caption>\n", (i as i32 + min_id), n[i]);
                writer.write_all(s1.as_bytes())?;

                // let mut s = format!("\nFeature/Class: {} (n={})\n", (i as i32 + min_id), n[i]);
                // writer.write_all(s.as_bytes())?;
                // println!("{}", s);
                // println!("Percentile,Value");
                // writer.write_all("Percentile,Value\n".as_bytes())?;
                let s1 = &format!("<tr>
                    <td class=\"numberCell\">0.0</td>
                    <td class=\"numberCell\">{}</td>
                    </tr>\n", format!("{:.*}", 4, data[i][0]));
                writer.write_all(s1.as_bytes())?;
                let mut val: f64;
                for j in 1..100 as usize {
                    let percentile = j as f64;
                    let a = percentile / 100f64 * n[i] as f64;
                    let b = a.floor();
                    let c = a.ceil();
                    if b != c {
                        val = data[i][b as usize] + (data[i][c as usize] - data[i][b as usize]) * (a - c) / (c - b);
                    } else {
                        val = data[i][b as usize];
                    }
                    let s1 = &format!("<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>\n", format!("{:.*}", 1, percentile), format!("{:.*}", 4, val));
                    writer.write_all(s1.as_bytes())?;
                }

                s = "</table>";
                writer.write_all(s.as_bytes())?;

            } else if n[i] > 0 {
                println!("Feature {} does not have sufficient area to calculate percentiles.", i as i32 + min_id);
            }
        }

    } else {
        let mut data: Vec<f64> = vec![];
        let mut z: f64;
        let mut progress: usize;
        let mut old_progress: usize = 1;
        for row in 0..rows as isize {
            for col in 0..columns as isize {
                z = input.get_value(row, col);
                if z != nodata {
                    data.push(z);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let n = data.len();

        // sort the vectors
        println!("Sorting the data...");
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // now calculate the percentiles
        s = "<br><table align=\"center\">
        <tr>
            <th>Percentile</th>
            <th>Value</th>
        </tr>";
        writer.write_all(s.as_bytes())?;
        let s1 = &format!("<tr>
            <td class=\"numberCell\">0.0</td>
            <td class=\"numberCell\">{}</td>
            </tr>\n", format!("{:.*}", 4, data[0]));
        writer.write_all(s1.as_bytes())?;

        // let mut s = format!("\nn={}", n);
        // writer.write_all(s.as_bytes())?;
        // writer.write_all("Percentile,Value".as_bytes())?;
        // writer.write_all(s.as_bytes())?;
        // s = format!("0,{}", data[0]);
        // writer.write_all(s.as_bytes())?;
        let mut val: f64;
        for j in 1..100 as usize {
            let percentile = j as f64;
            let a = percentile / 100f64 * n as f64;
            let b = a.floor();
            let c = a.ceil();
            if b != c {
                val = data[b as usize] + (data[c as usize] - data[b as usize]) * (a - c) / (c - b);
            } else {
                val = data[b as usize];
            }
            //println!("{},{}", percentile, val);
            //s = format!("{},{}", percentile, val);
            let s1 = &format!("<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>\n", format!("{:.*}", 1, percentile), format!("{:.*}", 4, val));
            writer.write_all(s1.as_bytes())?;
        }
    }

    s = "</body>";
    writer.write_all(s.as_bytes())?;

    let _ = writer.flush();

    println!("Complete! Please see {} for output", output_file);

    let output = Command::new("open")
                     .arg(output_file)
                     .output()
                     .expect("failed to execute process");

    let _ = output.stdout;

    Ok(())
}
