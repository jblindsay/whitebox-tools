/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: Dec. 15, 2017
Last Modified: Dec. 15, 2017
License: MIT

Notes: Compared with the original Whitebox GAT tool, this will output a table
with each of the mean, min, max, range, std dev, and total. The output raster can
only represent one statistic, given by the --stat flag.
*/
extern crate time;
extern crate num_cpus;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::env;
use std::path;
use std::f64;
use std::isize;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct ExtractRasterStatistics {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ExtractRasterStatistics {
    /// public constructor
    pub fn new() -> ExtractRasterStatistics { 
        let name = "ExtractRasterStatistics".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Extracts descriptive statistics for a group of patches in a raster.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input Data File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input data raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Input Feature Definition File".to_owned(), 
            flags: vec!["--features".to_owned()], 
            description: "Input feature definition raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output Raster File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Statistic Type".to_owned(), 
            flags: vec!["--stat".to_owned()], 
            description: "Statistic to extract.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "average".to_owned(), 
                "minimum".to_owned(), 
                "maximum".to_owned(), 
                "range".to_owned(), 
                "standard deviation".to_owned(), 
                "total".to_owned()
            ]),
            default_value: Some("mean".to_owned()),
            optional: true
        });

        // parameters.push(ToolParameter{
        //     name: "Would you like an HTML table output?".to_owned(), 
        //     flags: vec!["--out_table".to_owned()], 
        //     description: "Would you like an HTML table output?".to_owned(),
        //     parameter_type: ParameterType::Boolean,
        //     default_value: None,
        //     optional: true
        // });
        parameters.push(ToolParameter{
            name: "Output HTML Table File".to_owned(), 
            flags: vec!["--out_table".to_owned()], 
            description: "Output HTML Table file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.dep' --features='groups.dep' -o='output.dep' --stat='minimum'
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.dep' --features='groups.dep' --out_table='output.html'", short_exe, name).replace("*", &sep);
    
        ExtractRasterStatistics { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for ExtractRasterStatistics {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut features_file = String::new();
        let mut output_file = String::new();
        // let mut out_table = false;
        let mut output_html_file = String::new();
        let mut stat_type = String::from("average");
         
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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if flag_val == "-features" {
                if keyval {
                    features_file = vec[1].to_string();
                } else {
                    features_file = args[i+1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if flag_val == "-out_table" {
                // out_table = true;
                if keyval {
                    output_html_file = vec[1].to_string();
                } else {
                    output_html_file = args[i+1].to_string();
                }
            } else if flag_val == "-stat" {
                if keyval {
                    stat_type = vec[1].to_string().to_lowercase();
                } else {
                    stat_type = args[i+1].to_string().to_lowercase();
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
        if !features_file.contains(&sep) {
            features_file = format!("{}{}", working_directory, features_file);
        }
        if !output_file.is_empty() {
            if !output_file.contains(&sep) {
                output_file = format!("{}{}", working_directory, output_file);
            }
        }
        if !output_html_file.is_empty() {
            if !output_html_file.contains(&sep) {
                output_html_file = format!("{}{}", working_directory, output_html_file);
            }
        }
        if output_file.is_empty() && output_html_file.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput,
                "At least one of --output or --out_table must be specified."));
        }

        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let features = Arc::new(Raster::new(&features_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let features_nodata = features.configs.nodata;

        if features.configs.rows as isize != rows || features.configs.columns as isize != columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                "Input data and features definition raster must have the same dimensions."));
        }

        // How many features are there?
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let features = features.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut features_val: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut min_id = isize::max_value();
                    let mut max_id = isize::min_value();
                    let mut id: isize;
                    for col in 0..columns {
                        features_val = features.get_value(row, col);
                        if features_val != features_nodata {
                            id = features_val.round() as isize;
                            if id < min_id { min_id = id; }
                            if id > max_id { max_id = id; }
                        }
                    }
                    tx.send((min_id, max_id)).unwrap();
                }
            });
        }

        let mut min_id = isize::max_value();
        let mut max_id = isize::min_value();
        for row in 0..rows {
            let (min, max) = rx.recv().unwrap();
            if min < min_id { min_id = min; }
            if max > max_id { max_id = max; }
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let num_features = (max_id - min_id) as usize + 1usize;
        // In reality this is only the number of features if there are 
        // no unused feature IDs between the min and max values

        let mut features_total = vec![0f64; num_features];
        let mut features_n = vec![0f64; num_features];
        let mut features_average = vec![0f64; num_features];
        let mut features_total_deviation = vec![0f64; num_features];
        let mut features_std_deviation = vec![0f64; num_features];
        let mut features_min = vec![f64::INFINITY; num_features];
        let mut features_max = vec![f64::NEG_INFINITY; num_features];
        let mut features_range = vec![0f64; num_features];
        let mut features_present = vec![false; num_features];

        let mut val: f64;
        let mut features_val: f64;
        let mut id: usize;
        for row in 0..rows {
            for col in 0..columns {
                val = input.get_value(row, col);
                features_val = features.get_value(row, col);
                if val != nodata && features_val != features_nodata {
                    id = (features_val.round() as isize - min_id) as usize;
                    features_present[id] = true;
                    features_total[id] += val;
                    features_n[id] += 1f64;
                    if val < features_min[id] {
                        features_min[id] = val;
                    }
                    if val > features_max[id] {
                        features_max[id] = val;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        for id in 0..num_features {
            if features_n[id] > 0f64 {
                features_average[id] = features_total[id] / features_n[id];
                features_range[id] = features_max[id] - features_min[id];
            }
        }
        
        for row in 0..rows {
            for col in 0..columns {
                val = input.get_value(row, col);
                features_val = features.get_value(row, col);
                if val != nodata && features_val != features_nodata {
                    id = (features_val.round() as isize - min_id) as usize;
                    features_total_deviation[id] += (val - features_average[id]) * 
                            (val - features_average[id]);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        for id in 0..num_features {
            if features_n[id] > 1f64 {
                features_std_deviation[id] = (features_total_deviation[id] / (features_n[id] - 1f64)).sqrt();
            }
        }

        // output the raster, if specified.
        if !output_file.is_empty() {

            let mut output = Raster::initialize_using_file(&output_file, &input);
            let out_stat = if stat_type.contains("av") { 
                features_average.clone()
            } else if stat_type.contains("min") { 
                features_min.clone()
            } else if stat_type.contains("max") { 
                features_max.clone()
            } else if stat_type.contains("range") { 
                features_range.clone()
            } else if stat_type.contains("dev") { 
                features_std_deviation.clone()
            } else { // "total"
                features_total.clone()
            };
            for row in 0..rows {
                for col in 0..columns {
                    val = input.get_value(row, col);
                    features_val = features.get_value(row, col);
                    if val != nodata && features_val != features_nodata {
                        id = (features_val.round() as isize - min_id) as usize;
                        output.set_value(row, col, out_stat[id]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Output: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let end = time::now();
            let elapsed_time = end - start;
            output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
            output.add_metadata_entry(format!("Input file: {}", input_file));
            output.add_metadata_entry(format!("Features ID file: {}", features_file));
            output.add_metadata_entry(format!("Statistic: {}", stat_type));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

            if verbose { println!("Saving data...") };
            let _ = match output.write() {
                Ok(_) => if verbose { println!("Output file written") },
                Err(e) => return Err(e),
            };
        }

        if !output_html_file.is_empty() { // out_table {
            // let output_html_file = if output_file.is_empty() {
            //     // output_file not specified and should be based on input file
            //     let p = path::Path::new(&input_file);
            //     let mut extension = String::from(".");
            //     let ext = p.extension().unwrap().to_str().unwrap();
            //     extension.push_str(ext);
            //     input_file.replace(&extension, ".html")
            // } else {
            //     let p = path::Path::new(&output_file);
            //     let mut extension = String::from(".");
            //     let ext = p.extension().unwrap().to_str().unwrap();
            //     extension.push_str(ext);
            //     output_file.replace(&extension, ".html")
            // };
            

            let f = File::create(output_html_file.clone())?;
            let mut writer = BufWriter::new(f);

            writer.write_all("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Extract Raster Statistics</title>
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
                        text-align: center;
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
                <h1>Extract Raster Statistics Summary Report</h1>".as_bytes())?;

            writer.write_all(format!("<p><strong>Input data file</strong>: {}</p>", input_file).as_bytes())?;
            writer.write_all(format!("<p><strong>Input feature definition file</strong>: {}</p>", features_file).as_bytes())?;

            writer.write_all("<br><table align=\"center\">".as_bytes())?;

            // headers ID, average, min, max, range, std dev, and total
            writer.write_all("<tr>
                <th>Feature ID</th>
                <th>Average</th>
                <th>Minimum</th>
                <th>Maximum</th>
                <th>Range</th>
                <th>Std. Dev.</th>
                <th>Total</th>
            </tr>".as_bytes())?;

            // data
            for id in 0..num_features {
                if features_n[id] > 0f64 {
                    writer.write_all(&format!("<tr>
                        <td>{}</td>
                        <td class=\"numberCell\">{}</td>
                        <td class=\"numberCell\">{}</td>
                        <td class=\"numberCell\">{}</td>
                        <td class=\"numberCell\">{}</td>
                        <td class=\"numberCell\">{}</td>
                        <td class=\"numberCell\">{}</td>
                    </tr>", 
                    id, 
                    format!("{:.*}", 4, features_average[id]),
                    format!("{:.*}", 4, features_min[id]),
                    format!("{:.*}", 4, features_max[id]),
                    format!("{:.*}", 4, features_range[id]),
                    format!("{:.*}", 4, features_std_deviation[id]),
                    format!("{:.*}", 4, features_total[id]),
                    ).as_bytes())?;
                }
            }

            writer.write_all("</table>".as_bytes())?;
            writer.write_all("</body>".as_bytes())?;

            let _ = writer.flush();

            if verbose {
                if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                    let output = Command::new("open")
                        .arg(output_html_file.clone())
                        .output()
                        .expect("failed to execute process");

                    let _ = output.stdout;
                } else if cfg!(target_os = "windows") {
                    // let output = Command::new("cmd /c start")
                    let output = Command::new("explorer.exe")
                        .arg(output_html_file.clone())
                        .output()
                        .expect("failed to execute process");

                    let _ = output.stdout;
                } else if cfg!(target_os = "linux") {
                    let output = Command::new("xdg-open")
                        .arg(output_html_file.clone())
                        .output()
                        .expect("failed to execute process");

                    let _ = output.stdout;
                }

                println!("Complete! Please see {} for output.", output_html_file);
            }

        }

        let end = time::now();
        let elapsed_time = end - start;
        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        
        Ok(())
    }
}