#![allow(dead_code, unused_assignments)]

extern crate whitebox_tools;

use std::env;
use std::io::prelude::*;
use std::fs::File;
use std::path;
use std::process::Command;
use whitebox_tools::lidar::las;
use whitebox_tools::lidar::point_data::*;

fn main() {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let mut input_file1: String = "".to_string();
    let mut input_file2: String = "".to_string();
    let mut output_file: String = "".to_string();
    let mut working_directory: String = "".to_string();
    let mut keyval = false;
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 { panic!("Tool run with no paramters. Please see help (-h) for parameter descriptions."); }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        keyval = false;
        if vec.len() > 1 { keyval = true; }
        if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--input1" {
            if keyval {
                input_file1 = vec[1].to_string();
            } else {
                input_file1 = args[i+1].to_string();
            }
        } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--input2" {
            if keyval {
                input_file2 = vec[1].to_string();
            } else {
                input_file2 = args[i+1].to_string();
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
        } else if vec[0].to_lowercase() == "-h" || vec[0].to_lowercase() == "--help" ||
            vec[0].to_lowercase() == "--h"{
            let mut s: String = "Help:\n".to_owned();
                     s.push_str("-i1      Input LAS file (classification).\n");
                     s.push_str("-i2      Input LAS file (reference).\n");
                     s.push_str("-o       Output HTML file.\n");
                     s.push_str("-wd      Optional working directory. If specified, filenames parameters need not include a full path.\n");
                     s.push_str("-version Prints the tool version number.\n");
                     s.push_str("-h       Prints help information.\n\n");
                     s.push_str("Example usage:\n\n");
                     s.push_str(&">> .*lidar_kappa -wd *path*to*data* -i1 class.las -i2 ref.las -o kappa.html\n".replace("*", &sep));
            println!("{}", s);
            return;
        } else if vec[0].to_lowercase() == "-version" || vec[0].to_lowercase() == "--version" {
            const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
            println!("lidar_segmentation v{}", VERSION.unwrap_or("unknown"));
            return;
        }
    }

    println!("**************************");
    println!("* Welcome to lidar_kappa *");
    println!("**************************");

    let sep = std::path::MAIN_SEPARATOR;
    if !working_directory.ends_with(sep) {
        working_directory.push_str(&(sep.to_string()));
    }

    if !input_file1.contains(sep) {
        input_file1 = format!("{}{}", working_directory, input_file1);
    }

    if !input_file2.contains(sep) {
        input_file2 = format!("{}{}", working_directory, input_file2);
    }

    if !output_file.contains(sep) {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if !output_file.ends_with(".html") {
        output_file = output_file + ".html";
    }

    // let input1 = las::LasFile::new(&input_file1, "r");
    // let input2 = las::LasFile::new(&input_file2, "r");

    let input1: las::LasFile = match las::LasFile::new(&input_file1, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };

    let input2: las::LasFile = match las::LasFile::new(&input_file2, "r") {
        Ok(lf) => lf,
        Err(err) => panic!("Error: {}", err),
    };

    let num_points = input1.header.number_of_points;
    if input2.header.number_of_points != num_points {
        panic!("Error: The input files do not contain the same number of points.");
    }
    let mut error_matrix: [[usize; 256]; 256] = [[0; 256]; 256];
    let mut active_class: [bool; 256] = [false; 256];
    let mut p1: PointData;
    let mut p2: PointData;
    let (mut class1, mut class2): (usize, usize);
    for i in 0..input1.header.number_of_points as usize {
        p1 = input1.get_point_info(i);
        p2 = input2.get_point_info(i);
        class1 = p1.classification() as usize;
        class2 = p2.classification() as usize;
        error_matrix[class1][class2] += 1;
        active_class[class1] = true;
        active_class[class2] = true;
    }

    let mut num_classes = 0;
    for a in 0..256usize {
        if active_class[a] { num_classes += 1; }
    }

    let mut agreements = 0usize;
    let mut expected_frequency = 0f64;
    let mut n = 0usize;
    let mut row_total = 0usize;
    let mut col_total = 0usize;
    let mut kappa = 0f64;
    let mut overall_accuracy = 0f64;

    for a in 0..256usize {
        agreements += error_matrix[a][a];
        for b in 0..256usize {
            n += error_matrix[a][b];
        }
    }

    for a in 0..256usize {
        row_total = 0;
        col_total = 0;
        for b in 0..256usize {
            col_total += error_matrix[a][b];
            row_total += error_matrix[b][a];
        }
        expected_frequency += (col_total as f64 * row_total as f64) / (n as f64);
    }

    kappa = (agreements as f64 - expected_frequency as f64) / (n as f64 - expected_frequency as f64);
    overall_accuracy = agreements as f64 / n as f64;

    //for a in 0..256usize {
    //    if active_class[a] {
    //        let mut row_data: String = "".to_string();
    //        for b in 0..256usize {
    //            if active_class[b] {
    //                row_data = row_data + &format!("{}, ", error_matrix[a][b]);
    //            }
    //        }
    //        println!("{}", row_data);
    //    }
    //}

    let mut f = File::create(output_file.as_str()).unwrap();

    // let mut s = "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">";
    // f.write(s.as_bytes()).unwrap();
    // s = "<head>";
    // f.write(s.as_bytes()).unwrap();
    // s = "<meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\"><title>Kappa Index of Agreement Output</title>";
    // f.write(s.as_bytes()).unwrap();
    // s = "</head>";
    // f.write(s.as_bytes()).unwrap();
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
                margin-left: 15px;
                margin-right: 15px;
            }
            table {
                font-size: 12pt;
                font-family: Helvetica, Verdana, Geneva, Arial, sans-serif;
                font-family: arial, sans-serif;
                border-collapse: collapse;
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
    <body>";
    f.write(s.as_bytes()).unwrap();
    s = "<body><h1>Kappa Index of Agreement</h1>";
    f.write(s.as_bytes()).unwrap();
    let s2 = &format!("{}{}{}{}{}", "<b>Input Data:</b> <br><br><b>Classification Data:</b> ", input_file1, "<br><b>Reference Data:</b> ", input_file2, "<br>");
    f.write(s2.as_bytes()).unwrap();
    s = "<br><b>Contingency Table:</b><br>";
    f.write(s.as_bytes()).unwrap();
    s = "<br><table border=\"1\" cellspacing=\"0\" cellpadding=\"3\">";
    f.write(s.as_bytes()).unwrap();
    s = "<tr>";
    f.write(s.as_bytes()).unwrap();
    let s3 = &format!("{}{}{}", "<th colspan=\"2\" rowspan=\"2\"></th><th colspan=\"", num_classes, "\">Reference Data</th><th rowspan=\"2\">Row<br>Totals</th>");
    f.write(s3.as_bytes()).unwrap();
    s = "</tr>";
    f.write(s.as_bytes()).unwrap();
    s = "<tr>";
    f.write(s.as_bytes()).unwrap();
    for a in 0..256 {
        if active_class[a] {
            let s = &format!("{}{}{}", "<th>", convert_class_val_to_class_string(a as u8), "</th>");
            f.write(s.as_bytes()).unwrap();
        }
    }

    s = "</tr>";
    f.write(s.as_bytes()).unwrap();
    let mut first_entry = true;
    for a in 0..256 {
        if active_class[a] {
            if first_entry {
                let s = format!("{}{}{}{}{}", "<tr><th rowspan=\"", num_classes, "\">Class<br>Data</th> <th>", convert_class_val_to_class_string(a as u8), "</th>");
                f.write(s.as_bytes()).unwrap();
            } else {
                let s = format!("{}{}{}", "<tr><th>", convert_class_val_to_class_string(a as u8), "</th>");
                f.write(s.as_bytes()).unwrap();
            }
            row_total = 0;
            for b in 0..256 {
                if active_class[b] {
                    row_total += error_matrix[a][b];
                    let s = format!("{}{}{}", "<td>", error_matrix[a][b], "</td>");
                    f.write(s.as_bytes()).unwrap();
                }
            }
            let s = format!("{}{}{}", "<td>", row_total, "</td>");
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
    for a in 0..256 {
        if active_class[a] {
            col_total = 0;
            for b in 0..256 {
                if active_class[b] {
                    col_total += error_matrix[b][a];
                }
            }
            let s = &format!("{}{}{}", "<td>", col_total, "</td>");
            f.write(s.as_bytes()).unwrap();
        }
    }

    let s4 = &format!("{}{}{}", "<td><b>N</b>=", n, "</td></tr>");
    f.write(s4.as_bytes()).unwrap();
    s = "</table>";
    f.write(s.as_bytes()).unwrap();
    s = "<br><b>Class Statistics:</b><br><br>";
    f.write(s.as_bytes()).unwrap();
    s = "<table border=\"1\" cellspacing=\"0\" cellpadding=\"3\">";
    f.write(s.as_bytes()).unwrap();
    s = "<tr><td><b>Class</b></td><td><b>User's Accuracy</b><sup>1</sup><br>(Reliability)</td><td><b>Producer's Accuracy</b><sup>1</sup><br>(Accuracy)</td></tr>";
    f.write(s.as_bytes()).unwrap();

    //DecimalFormat df = new DecimalFormat("0.00%");
    //DecimalFormat df2 = new DecimalFormat("0.000");
    let mut average_producers = 0.0;
    let mut average_users = 0.0;
    let mut num_active = 0.0;
    for a in 0..256 {
        if active_class[a] {
            num_active += 1.0;
            let mut row_total = 0;
            let mut col_total = 0;
            for b in 0..256 {
                if active_class[b] {
                    col_total += error_matrix[a][b];
                    row_total += error_matrix[b][a];
                }
            }
            average_users += 100.0 * error_matrix[a][a] as f64 / col_total as f64;
            average_producers += 100.0 * error_matrix[a][a] as f64 / row_total as f64;
            let s = &format!("{}{}{}{}{}{}{}", "<tr><td>",  convert_class_val_to_class_string(a as u8), "</td><td>", format!("{:.*}", 2, (100.0 * error_matrix[a][a] as f64 / col_total as f64)),
                    "%</td><td>", format!("{:.*}", 2, (100.0 * error_matrix[a][a] as f64 / row_total as f64)), "%</td></tr>");
            f.write(s.as_bytes()).unwrap();
        }
    }
    f.write(format!("<tr><td>Average</td><td>{}%</td><td>{}%</td></tr>", format!("{:.*}", 2, average_users / num_active),
            format!("{:.*}", 2, average_producers / num_active)).as_bytes()).unwrap();


    s = "</table>";
    f.write(s.as_bytes()).unwrap();
    let s6 = &format!("{}{}", "<br><b>Overall Accuracy</b> = ", format!("{:.*}%", 2, overall_accuracy * 100.0));
    f.write(s6.as_bytes()).unwrap();
    let s7 = &format!("<br><br><b>Kappa</b><sup>2</sup> = {}<br>", format!("{:.*}", 3, kappa));
    f.write(s7.as_bytes()).unwrap();
    let s5 = &format!("{}{}", "<br>Notes:<br>1. User's accuracy refers to the proportion of points correctly assigned to a class (i.e. the number of points correctly classified for a category divided by the row total in the contingency table) and is a measure of the reliability. ",
            "Producer's accuracy is a measure of the proportion of the points in each category correctly classified (i.e. the number of points correctly classified for a category divided by the column total in the contingency table) and is a measure of the accuracy.<br>");
    f.write(s5.as_bytes()).unwrap();
    f.write("<br>2. Cohen's kappa coefficient is a statistic that measures inter-rater agreement for qualitative (categorical)
    items. It is generally thought to be a more robust measure than simple percent agreement calculation, since
    kappa takes into account the agreement occurring by chance. Kappa measures the percentage of data values in the
    main diagonal of the table and then adjusts these values for the amount of agreement that could be expected due
    to chance alone.".as_bytes()).unwrap();
    s = "</body>";
    f.write(s.as_bytes()).unwrap();


    //println!("Overall Accuracy: {}%", format!("{:.*}", 2, overall_accuracy * 100.0));
    //println!("Kappa: {}", format!("{:.*}", 2, kappa));

    println!("Complete, please see output file for results.");

    let output = Command::new("open")
                     .arg(output_file)
                     .output()
                     .expect("failed to execute process");

    let _ = output.stdout;
}
