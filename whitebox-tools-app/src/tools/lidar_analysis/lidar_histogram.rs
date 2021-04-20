/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/12/2017
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_lidar::*;
use whitebox_common::rendering::html::*;
use whitebox_common::rendering::Histogram;
use crate::tools::*;
use whitebox_common::structures::Point3D;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to plot a histogram of data derived from a LiDAR file. The user must specify the
/// name of the input LAS file (`--input`), the name of the output HTML file (`--output`), the parameter
/// (`--parameter`) to be plotted, and the amount (in percent) to clip the upper and lower tails of the f
/// requency distribution (`--clip`). The LiDAR parameters that can be plotted using `LidarHistogram`
/// include the point elevations, intensity values, scan angles, and class values.
///
/// Use the `LidarPointStats` tool instead to examine the spatial distribution of LiDAR points.
///
/// ![](../../doc_img/LidarHistogram_fig1.png)
///
/// # See Also
/// `LidarPointStats`
pub struct LidarHistogram {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarHistogram {
    pub fn new() -> LidarHistogram {
        // public constructor
        let name = "LidarHistogram".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates a histogram of LiDAR data.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output HTML file (default name will be based on input file if unspecified)."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Parameter".to_owned(),
            flags: vec!["--parameter".to_owned()],
            description:
                "Parameter; options are 'elevation' (default), 'intensity', 'scan angle', 'class'."
                    .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "elevation".to_owned(),
                "intensity".to_owned(),
                "scan angle".to_owned(),
                "class".to_owned(),
            ]),
            default_value: Some("elevation".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Tail Clip Percent".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Amount to clip distribution tails (in percent).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.tif, file2.tif, file3.tif\" -o=outfile.htm --contiguity=Bishopsl",
                            short_exe, name).replace("*", &sep);

        LidarHistogram {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarHistogram {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut parameter = "elevation".to_string();
        let mut clip_percent = 1.0f64;

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
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-parameter" {
                if keyval {
                    parameter = vec[1].to_string().to_lowercase();
                } else {
                    parameter = args[i + 1].to_string().to_lowercase();
                }
            } else if flag_val == "-clip" {
                if keyval {
                    clip_percent = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    clip_percent = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: i32;
        let mut old_progress: i32 = -1;

        let start = Instant::now();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        // convert the parameter to a numeric mode value
        let parameter_mode = match parameter.as_ref() {
            "elevation" => 0,
            "intensity" => 1,
            "scan angle" => 2,
            "class" => 3,
            _ => {
                println!("Warning: unrecognized parameter; elevation will be used");
                0 // elevation
            }
        };

        let mut z: f64;
        let mut pd: PointData;
        let mut val: Point3D;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for i in 0..n_points {
            pd = input.get_point_info(i);
            val = input.get_transformed_coords(i);
            z = match parameter_mode {
                0 => val.z,
                1 => pd.intensity as f64,
                2 => pd.scan_angle as f64,
                _ => pd.classification() as f64,
            };
            if z < min {
                min = z;
            }
            if z > max {
                max = z;
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Calculating min and max: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut range = max - min + 0.00001f64;
        let mut num_bins = 1000usize;
        let mut bin_width = range / num_bins as f64;
        let mut freq_data = vec![0usize; num_bins];

        if parameter_mode != 3 {
            let mut bin: isize;
            for i in 0..n_points {
                pd = input.get_point_info(i);
                val = input.get_transformed_coords(i);
                z = match parameter_mode {
                    0 => val.z,
                    1 => pd.intensity as f64,
                    _ => pd.scan_angle as f64,
                };
                bin = ((z - min) / bin_width).floor() as isize;
                freq_data[bin as usize] += 1;
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Clipping the tails: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            // are there outliers?
            clip_percent /= 100f64;
            let tail_threshold = (num_points * clip_percent) as usize;
            let mut n = 0usize;
            let mut lower_tail = 0;
            for bin in 0..num_bins {
                n += freq_data[bin];
                if n > tail_threshold {
                    lower_tail = bin;
                    break;
                }
            }

            n = 0usize;
            let mut upper_tail = 0;
            for bin in (0..num_bins).rev() {
                n += freq_data[bin];
                if n > tail_threshold {
                    upper_tail = bin;
                    break;
                }
            }

            let old_min = min;
            let old_max = max;
            if old_min < old_min + lower_tail as f64 * bin_width {
                min = min + lower_tail as f64 * bin_width;
            }
            if old_max > old_min + upper_tail as f64 * bin_width + bin_width {
                max = old_min + upper_tail as f64 * bin_width + bin_width;
            }

            if min > max {
                let tmp1 = max;
                max = min;
                min = tmp1;
            }

            range = max - min + 0.00001f64;
            num_bins = num_points.log2().ceil() as usize + 1;
            bin_width = range / num_bins as f64;
            freq_data = vec![0usize; num_bins];
            let mut bin: isize;
            for i in 0..n_points {
                pd = input.get_point_info(i);
                val = input.get_transformed_coords(i);
                z = match parameter_mode {
                    0 => val.z,
                    1 => pd.intensity as f64,
                    _ => pd.scan_angle as f64,
                };
                bin = ((z - min) / bin_width).floor() as isize;
                if bin >= 0 && bin < num_bins as isize {
                    freq_data[bin as usize] += 1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Binning the data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            range = max - min + 0.00001f64;
            num_bins = range as usize + 1;
            bin_width = 1f64;
            freq_data = vec![0usize; num_bins];
            let mut bin: isize;
            for i in 0..n_points {
                // val = input.get_point_info(i);
                z = input.get_point_info(i).classification() as f64;
                bin = ((z - min) / bin_width).floor() as isize;
                if bin >= 0 && bin < num_bins as isize {
                    freq_data[bin as usize] += 1;
                }
                if verbose {
                    progress = (100.0_f64 * i as f64 / num_points) as i32;
                    if progress != old_progress {
                        println!("Binning the data: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Histogram Analysis</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Histogram Analysis</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(
            &format!("<p><strong>Input</strong>: {}<br>", input_file.clone()).as_bytes(),
        )?;
        writer.write_all(&format!("<strong>Parameter</strong>: {}", parameter).as_bytes())?;
        if parameter_mode != 3 {
            writer.write_all(
                &format!(
                    "<br><strong>Clip amount</strong>: {}%</p>",
                    clip_percent * 100f64
                )
                .as_bytes(),
            )?;
        } else {
            writer.write_all("</p>".as_bytes())?;
        }

        let x_axis_label = match parameter_mode {
            0 => "Elevation Value".to_owned(),
            1 => "Intensity Value".to_owned(),
            2 => "Scan Angle Value".to_owned(),
            _ => "Class Value".to_owned(),
        };

        let histo = Histogram {
            parent_id: "histo".to_owned(),
            width: 700f64,
            height: 500f64,
            freq_data: freq_data.clone(),
            min_bin_val: min,
            bin_width: bin_width,
            x_axis_label: x_axis_label,
            cumulative: false,
        };

        writer.write_all(
            &format!("<div id='histo' align=\"center\">{}</div>", histo.get_svg()).as_bytes(),
        )?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
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
