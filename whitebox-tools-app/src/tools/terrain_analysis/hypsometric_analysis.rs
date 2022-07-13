/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/01/2018
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::Raster;
use whitebox_common::rendering::html::*;
use whitebox_common::rendering::LineGraph;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to derive the hypsometric curve, or area-altitude curve, of one or more
/// input digital elevation models (DEMs) (`--inputs`). A hypsometric curve is a histogram or cumulative
/// distribution function of elevations in a geographical area.
///
/// ![](../../doc_img/HypsometricAnalysis_fig1.png)
///
/// # See Also
/// `SlopeVsElevationPlot`
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
        parameters.push(ToolParameter {
            name: "Input DEM Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input DEM files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Watershed Files (optional)".to_owned(),
            flags: vec!["--watershed".to_owned()],
            description: "Input watershed files (optional).".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"DEM1.tif;DEM2.tif\" --watershed=\"ws1.tif;ws2.tif\" -o=outfile.html",
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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_files_str = String::new();
        let mut watershed_files_str = "".to_string();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-inputs" {
                input_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-watershed" {
                watershed_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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

        let start = Instant::now();

        let mut cmd = input_files_str.split(";");
        let mut input_files = cmd.collect::<Vec<&str>>();
        if input_files.len() == 1 {
            cmd = input_files_str.split(",");
            input_files = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_files.len();
        if num_files < 1 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input DEM files. At least one input DEM is required to operate this tool."));
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Hypsometric Analysis</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Hypsometric Analysis</h1>"#
                .as_bytes(),
        )?;

        if num_files > 1 {
            writer.write_all(("<p><strong>Input DEMs</strong>:<br>").as_bytes())?;
        }

        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut shortnames = vec![];
        let mut hi_data = vec![];

        if watershed_files_str.is_empty() {
            for i in 0..num_files {
                let mut input_file = input_files[i].to_string();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;

                
                let rows = input.configs.rows as isize;
                let columns = input.configs.columns as isize;
                let nodata = input.configs.nodata;

                let min = input.configs.minimum;
                let max = input.configs.maximum;
                let range = max - min; // + 0.00001f64;
                let num_bins = 101; // (max - min) as usize / 5;
                // if num_bins < ((rows * columns) as f64).log2().ceil() as usize + 1 {
                //     num_bins = ((rows * columns) as f64).log2().ceil() as usize + 1;
                // }
                // let bin_width = range / num_bins as f64;
                let mut freq_data = vec![0f64; num_bins];
                let mut bin_elevations = vec![0f64; num_bins];
                let mut val: f64;
                let mut bin: usize;
                let mut total = 0f64;
                let mut total_n = 0f64;
                for row in 0..rows {
                    for col in 0..columns {
                        val = input.get_value(row, col);
                        if val != nodata {
                            // bin = ((val - min) / bin_width).floor() as usize;
                            bin = (100f64 * (val - min) / range).floor() as usize;
                            // if bin >= num_bins { bin = num_bins - 1; }
                            freq_data[bin] += 1f64;
                            total_n += 1f64;
                            total += val;
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

                let mean = total / total_n;

                bin_elevations[0] = 0.0001;
                for i in 1..num_bins {
                    freq_data[i] += freq_data[i - 1];
                    // bin_elevations[i] =  min + i as f64 * bin_width;
                    bin_elevations[i] =  i as f64 / num_bins as f64;
                }

                for i in 0..num_bins {
                    // freq_data[i] = 100f64 * (1f64 - freq_data[i] / total_n);
                    freq_data[i] = 1f64 - freq_data[i] / total_n;
                }

                freq_data[num_bins - 1] = 0.0001; // this is necessary so the axis will start at zero.
                xdata.push(freq_data);
                ydata.push(bin_elevations);
                shortnames.push(input.get_short_filename());
                hi_data.push((mean - min) / range);

                if num_files == 1 {
                    writer.write_all(
                        (format!(
                            "<p><strong>Input DEM</strong>: {}, Hypsometric Integral: {:.3}",
                            input.get_short_filename(),
                            hi_data[i]
                        ))
                        .as_bytes(),
                    )?;
                } else {
                // if num_files > 1 {
                    writer.write_all(&format!("{}, Hypsometric Integral: {:.3}<br>", shortnames[i], hi_data[i]).as_bytes())?;
                }
            }
        } else {
            // there are watersheds specified for each input DEM.
            let mut cmd = watershed_files_str.split(";");
            let mut watershed_files = cmd.collect::<Vec<&str>>();
            if watershed_files.len() == 1 {
                cmd = watershed_files_str.split(",");
                watershed_files = cmd.collect::<Vec<&str>>();
            }
            if watershed_files.len() != num_files {
                return Err(Error::new(ErrorKind::InvalidInput,
                        "There should be the same number of input DEM and watershed rasters, if watersheds are used."));
            }

            for i in 0..num_files {
                let mut input_file = input_files[i].to_string();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;
                if num_files == 1 {
                    writer.write_all(
                        (format!(
                            "<p><strong>Input DEM</strong>: {}",
                            input.get_short_filename()
                        ))
                        .as_bytes(),
                    )?;
                }
                let rows = input.configs.rows as isize;
                let columns = input.configs.columns as isize;
                let nodata = input.configs.nodata;

                let mut watershed_file = watershed_files[i].to_string();
                if !watershed_file.contains(&sep) {
                    watershed_file = format!("{}{}", working_directory, watershed_file);
                }
                let watershed = Raster::new(&watershed_file, "r")?;
                let ws_nodata = watershed.configs.nodata;
                if watershed.configs.rows as isize != rows
                    || watershed.configs.columns as isize != columns
                {
                    return Err(Error::new(ErrorKind::InvalidInput,
                            "The input DEM and watershed rasters should have the same extents (rows and columns)."));
                }

                let watershed_min = watershed.configs.minimum;
                let watershed_max = watershed.configs.maximum;
                let num_watersheds = (watershed_max - watershed_min) as usize + 1;

                // get the number of watersheds, and the min and max elev for each watershed
                let mut min_elevs = vec![f64::INFINITY; num_watersheds];
                let mut max_elevs = vec![f64::NEG_INFINITY; num_watersheds];
                let mut z: f64;
                let mut val: f64;
                let mut watershed_id: usize;
                for row in 0..rows {
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            val = watershed.get_value(row, col);
                            if val != 0f64 && val != ws_nodata {
                                watershed_id = (val - watershed_min) as usize;
                                if z < min_elevs[watershed_id] {
                                    min_elevs[watershed_id] = z;
                                }
                                if z > max_elevs[watershed_id] {
                                    max_elevs[watershed_id] = z;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Processing watershed data: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }

                let mut freq_data = vec![];
                let mut bin_elevations = vec![];
                let mut num_bins_data = vec![];
                let mut bin_widths = vec![];
                for w in 0..num_watersheds {
                    if min_elevs[w] < f64::INFINITY {
                        let min = min_elevs[w];
                        let max = max_elevs[w];
                        let range = max - min + 0.00001f64;
                        let mut num_bins = (max - min) as usize / 5;
                        if num_bins < ((rows * columns) as f64).log2().ceil() as usize + 1 {
                            num_bins = ((rows * columns) as f64).log2().ceil() as usize + 1;
                        }
                        let bin_width = range / num_bins as f64;
                        bin_widths.push(bin_width);
                        freq_data.push(vec![0f64; num_bins]);
                        num_bins_data.push(num_bins);
                        bin_elevations.push(vec![0f64; num_bins]);
                    } else {
                        freq_data.push(vec![0f64; 1]);
                        bin_elevations.push(vec![0f64; 1]);
                        num_bins_data.push(0);
                        bin_widths.push(0f64);
                    }
                }

                let mut bin: usize;
                let mut total_n = vec![0f64; num_watersheds];
                for row in 0..rows {
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            val = watershed.get_value(row, col);
                            if val != 0f64 && val != ws_nodata {
                                watershed_id = (val - watershed_min) as usize;
                                bin = ((z - min_elevs[watershed_id]) / bin_widths[watershed_id])
                                    .floor() as usize;
                                freq_data[watershed_id][bin] += 1f64;
                                total_n[watershed_id] += 1f64;
                            }
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

                for w in 0..num_watersheds {
                    if min_elevs[w] < f64::INFINITY {
                        bin_elevations[w][0] = min_elevs[w];
                        for i in 1..num_bins_data[w] {
                            freq_data[w][i] += freq_data[w][i - 1];
                            bin_elevations[w][i] = min_elevs[w] + i as f64 * bin_widths[w];
                        }

                        for i in 0..num_bins_data[w] {
                            freq_data[w][i] = 100f64 * (1f64 - freq_data[w][i] / total_n[w]);
                        }

                        freq_data[w][num_bins_data[w] - 1] = 0.0001; // this is necessary so the axis will start at zero.
                        xdata.push(freq_data[w].clone());
                        ydata.push(bin_elevations[w].clone());
                        shortnames.push(format!(
                            "{} {}",
                            input.get_short_filename(),
                            w as f64 + watershed_min
                        ));
                    }
                }

                if num_files > 1 {
                    writer.write_all(&format!("{}<br>", shortnames[i]).as_bytes())?;
                }
            }
        }
        writer.write_all(("</p>").as_bytes())?;
        let elapsed_time = get_formatted_elapsed_time(start);

        let multiples = num_files > 1;
        let width = if multiples { 650f64 } else { 500f64 };
        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: width,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: shortnames.clone(),
            x_axis_label: "Relative Area (a / A)".to_string(),
            y_axis_label: "Relative Altitude (h / H)".to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

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

            println!("Complete! Please see {} for output.", output_file);
        }

        Ok(())
    }
}
