/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: January 30, 2018
Last Modified: January 30, 2018
License: MIT
*/
extern crate time;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::env;
use std::path;
use std::f64;
use std::process::Command;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;
use rendering::LineGraph;
use rendering::html::*;

pub struct HypsometricAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HypsometricAnalysis {
    pub fn new() -> HypsometricAnalysis {
        // public constructor
        let name = "HypsometricAnalysis".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates a hypsometric curve for one or more DEMs.".to_string();

        let mut parameters = vec![];
        // parameters.push(ToolParameter{
        //     name: "Input DEM File".to_owned(), 
        //     flags: vec!["-i".to_owned(), "--input".to_owned()], 
        //     description: "Input DEM file.".to_owned(),
        //     parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
        //     default_value: None,
        //     optional: false
        // });
        parameters.push(ToolParameter{
            name: "Input DEM Files".to_owned(), 
            flags: vec!["-i".to_owned(), "--inputs".to_owned()], 
            description: "Input DEM files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output HTML File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output HTML file (default name will be based on input file if unspecified).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"DEM.tif\" -o=outfile.html",
                            short_exe, name).replace("*", &sep);

        HypsometricAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HypsometricAnalysis {
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
        // let mut input_file = String::new();
        let mut input_files_str = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters."));
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
            // if flag_val == "-i" || flag_val == "-input" {
            //     if keyval {
            //         input_file = vec[1].to_string();
            //     } else {
            //         input_file = args[i+1].to_string();
            //     }
            if flag_val == "-i" || flag_val == "-inputs" {
                input_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
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

        let start = time::now();
        
        let mut cmd = input_files_str.split(";");
        let mut input_files = cmd.collect::<Vec<&str>>();
        if input_files.len() == 1 {
            cmd = input_files_str.split(",");
            input_files = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_files.len();
        if num_files < 1 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Hypsometric Analysis</title>"#.as_bytes())?;
        
        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;
            
        writer.write_all(&r#"</head>
        <body>
            <h1>Hypsometric Analysis</h1>"#.as_bytes())?;
        
        if num_files > 1 {
            writer.write_all(("<p><strong>Input DEMs</strong>:<br>").as_bytes())?;
        }
        
        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut shortnames = vec![];
        for i in 0..num_files {
            let mut input_file = input_files[i].to_string();
            if !input_file.contains(&sep) {
                input_file = format!("{}{}", working_directory, input_file);
            }
            let input = Raster::new(&input_file, "r")?;

            if num_files == 1 {
                writer.write_all((format!("<p><strong>Input DEM</strong>: {}", input.get_short_filename())).as_bytes())?;
            }
            let rows = input.configs.rows as isize;
            let columns = input.configs.columns as isize;
            let nodata = input.configs.nodata;
            
            let min = input.configs.minimum;
            let max = input.configs.maximum;
            let range = max - min + 0.00001f64;
            let mut num_bins = (max - min) as usize / 5;
            if num_bins < ((rows * columns) as f64).log2().ceil() as usize  + 1 {
                num_bins = ((rows * columns) as f64).log2().ceil() as usize  + 1;
            }
            let bin_width = range / num_bins as f64;
            let mut freq_data = vec![0f64; num_bins];
            let mut bin_elevations = vec![0f64; num_bins];
            let mut val: f64;
            let mut bin: usize;
            let mut total_n = 0f64;
            for row in 0..rows {
                for col in 0..columns {
                    val = input.get_value(row, col);
                    if val != nodata {
                        bin = ((val - min) / bin_width).floor() as usize;
                        freq_data[bin] += 1f64;
                        total_n += 1f64;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Binning the data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            bin_elevations[0] = min;
            for i in 1..num_bins {
                freq_data[i] += freq_data[i - 1];
                bin_elevations[i] = min + i as f64 * bin_width;
            }

            for i in 0..num_bins {
                freq_data[i] = 100f64 * (1f64 - freq_data[i] / total_n);
            }

            freq_data[num_bins-1] = 0.0001; // this is necessary so the axis will start at zero.
            xdata.push(freq_data);
            ydata.push(bin_elevations);
            shortnames.push(input.get_short_filename());

            if num_files > 1 {
                writer.write_all(&format!("{}<br>", shortnames[i]).as_bytes())?;
            }
        }
        writer.write_all(("</p>").as_bytes())?;
        let end = time::now();
        let elapsed_time = end - start;

        let multiples = num_files > 1;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 600f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: shortnames.clone(), 
            x_axis_label: "% Area Above".to_string(),
            y_axis_label: "Elevation".to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(&format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes())?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        if verbose { println!("\n{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")); }

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