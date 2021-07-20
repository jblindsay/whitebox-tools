/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/12/2017
Last Modified: 12/10/2018
License: MIT
*/

use self::statrs::distribution::{Normal, Univariate};
use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use statrs;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// Spatial autocorrelation describes the extent to which a variable is either dispersed or clustered through space.
/// In the case of a raster image, spatial autocorrelation refers to the similarity in the values of nearby grid
/// cells. This tool measures the spatial autocorrelation of a raster image using the global Moran's *I* statistic.
/// Moran's *I* varies from -1 to 1, where *I* = -1 indicates a dispersed, checkerboard type pattern and *I* = 1 indicates
/// a clustered (smooth) surface. *I* = 0 occurs for a random distribution of values. `ImageAutocorrelation` computes
/// Moran's *I* for the first lag only, meaning that it only takes into account the variability among the immediate
/// neighbors of each grid cell.
///
/// The user must specify the names of one or more input raster images. In addition, the user must specify the
/// contiguity type (`--contiguity`; Rook's, King's, or Bishop's), which describes which neighboring grid cells are examined for
/// the analysis. The following figure describes the available cases:
///
/// Rook's contiguity
///
/// | . |  .  | . |
/// |:-:|:---:|:-:|
/// | 0 |  1  | 0 |
/// | 1 |  X  | 1 |
/// | 0 |  1  | 0 |
///
/// Kings's contiguity
///
/// | . |  .  | . |
/// |:-:|:---:|:-:|
/// | 1 |  1  | 1 |
/// | 1 |  X  | 1 |
/// | 1 |  1  | 1 |
///
/// Bishops's contiguity
///
/// | . |  .  | . |
/// |:-:|:---:|:-:|
/// | 1 |  0  | 1 |
/// | 0 |  X  | 0 |
/// | 1 |  0  | 1 |
///
/// The tool outputs an HTML report (`--output`) which, for each input image (`--input`), reports the Moran's *I*
/// value and the variance, z-score, and p-value (significance) under normal and randomization sampling assumptions.
///
/// Use the `ImageCorrelation` tool instead when there is need to determine the correlation among multiple raster
/// inputs.
///
/// **NoData **values in the input image are ignored during the analysis.
///
/// # See Also
/// `ImageCorrelation`, `ImageCorrelationNeighbourhoodAnalysis`
pub struct ImageAutocorrelation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImageAutocorrelation {
    pub fn new() -> ImageAutocorrelation {
        // public constructor
        let name = "ImageAutocorrelation".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs Moran's I analysis on two or more input images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input raster files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Contiguity Type".to_owned(),
            flags: vec!["--contiguity".to_owned()],
            description: "Contiguity type.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "Rook".to_owned(),
                "King".to_owned(),
                "Bishop".to_owned(),
            ]),
            default_value: Some("Rook".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.tif, file2.tif, file3.tif\" -o=outfile.html --contiguity=Bishops",
                            short_exe,
                            name)
                .replace("*", &sep);

        ImageAutocorrelation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImageAutocorrelation {
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
        let mut input_files: String = String::new();
        let mut output_file = String::new();
        let mut contiguity = String::new();

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
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-contiguity" {
                if keyval {
                    contiguity = vec[1].to_string().to_lowercase();
                } else {
                    contiguity = args[i + 1].to_string().to_lowercase();
                }
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

        let (dx, dy) = if contiguity.contains("bishop") {
            (vec![1, 1, -1, -1], vec![-1, 1, 1, -1])
        } else if contiguity.contains("queen") || contiguity.contains("king") {
            (
                vec![1, 1, 1, 0, -1, -1, -1, 0],
                vec![-1, 0, 1, 1, 1, 0, -1, -1],
            )
        } else {
            // go with the rook default
            (vec![1, 0, -1, 0], vec![0, 1, 0, -1])
        };

        let mut files = input_files.split(";");
        let mut files_vec = files.collect::<Vec<&str>>();
        if files_vec.len() == 1 {
            files = input_files.split(",");
            files_vec = files.collect::<Vec<&str>>();
        }

        if output_file.len() == 0 {
            // output_file not specified and should be based on input file
            let p = path::Path::new(&files_vec[0]);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            output_file = files_vec[0].replace(&extension, ".html");
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut file_names = vec![];
        for a in 0..files_vec.len() {
            let value = files_vec[a];
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                file_names.push(input_file);
            }
        }

        let num_files = file_names.len();

        let distribution = Normal::new(0.0, 1.0).unwrap();

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();

        let mut image_totals = vec![0f64; num_files];
        let mut n = vec![0f64; num_files];
        let mut mean = vec![0f64; num_files];
        let mut e_i = vec![0f64; num_files];
        let mut std_dev = vec![0f64; num_files];
        let mut i = vec![0f64; num_files];
        let mut var_normality = vec![0f64; num_files];
        let mut var_randomization = vec![0f64; num_files];
        let mut z_n = vec![0f64; num_files];
        let mut z_r = vec![0f64; num_files];
        let mut p_value_n = vec![0f64; num_files];
        let mut p_value_r = vec![0f64; num_files];
        let mut rows: isize = 0;
        let mut columns: isize = 0;
        if verbose {
            println!("Calculating image averages...");
        }
        for a in 0..num_files {
            let value = &file_names[a]; //files_vec[a];
            let input_file = value.trim(); //.to_owned();
            let input = Arc::new(Raster::new(&input_file, "r")?);
            let nodata = input.configs.nodata;
            if a == 0 {
                rows = input.configs.rows as isize;
                columns = input.configs.columns as isize;
            } else {
                if input.configs.columns as isize != columns || input.configs.rows as isize != rows
                {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "All input images must have the same dimensions (rows and columns).",
                    ));
                }
            }

            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut total = 0f64;
                    let mut n = 0f64;
                    let mut z: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        for col in 0..columns {
                            z = input.get_value(row, col);
                            if z != nodata {
                                total += z;
                                n += 1f64;
                            }
                        }
                    }
                    tx.send((total, n)).unwrap();
                });
            }
            for np in 0..num_procs {
                let (total, image_n) = rx.recv().expect("Error receiving data from thread.");
                image_totals[a] += total;
                n[a] += image_n;

                if verbose && num_procs > 1 {
                    progress = (100.0_f64 * np as f64 / (num_procs - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 1 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            mean[a] = image_totals[a] / n[a];

            e_i[a] = -1f64 / (n[a] - 1f64);
            let mut total_deviation = 0f64;
            let mut w = 0f64;
            let mut numerator = 0f64;
            let mut s2 = 0f64;
            let mut wij: f64;
            let mut z: f64;
            let mut zn: f64;
            let mut x: isize;
            let mut y: isize;
            let num_neighbours = dx.len();
            let mut k = 0f64;
            for row in 0..rows {
                for col in 0..columns {
                    z = input.get_value(row, col);
                    if z != nodata {
                        total_deviation += (z - mean[a]) * (z - mean[a]);
                        k += (z - mean[a]) * (z - mean[a]) * (z - mean[a]) * (z - mean[a]);
                        wij = 0f64;
                        for i in 0..num_neighbours {
                            x = col + dx[i];
                            y = row + dy[i];
                            zn = input.get_value(y, x);
                            if zn != nodata {
                                w += 1f64;
                                numerator += (z - mean[a]) * (zn - mean[a]);
                                wij += 1f64;
                            }
                        }
                        s2 += wij * wij;
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress (Loop 2 of 2): {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let s1 = 4f64 * w;
            s2 = s2 * 4f64;

            std_dev[a] = (total_deviation / (n[a] - 1f64)).sqrt();

            i[a] = n[a] * numerator / (total_deviation * w);

            var_normality[a] =
                (n[a] * n[a] * s1 - n[a] * s2 + 3f64 * w * w) / ((w * w) * (n[a] * n[a] - 1f64));

            z_n[a] = (i[a] - e_i[a]) / var_normality[a].sqrt();
            p_value_n[a] = 2f64 * (1f64 - distribution.cdf(z_n[a].abs()));

            k = k / (n[a] * std_dev[a] * std_dev[a] * std_dev[a] * std_dev[a]);

            var_randomization[a] = (n[a]
                * ((n[a] * n[a] - 3f64 * n[a] + 3f64) * s1 - n[a] * s2 + 3f64 * w * w)
                - k * (n[a] * n[a] - n[a]) * s1
                - 2f64 * n[a] * s1
                + 6f64 * w * w)
                / ((n[a] - 1f64) * (n[a] - 2f64) * (n[a] - 3f64) * w * w);

            z_r[a] = (i[a] - e_i[a]) / var_randomization[a].sqrt();
            p_value_r[a] = 2f64 * (1f64 - distribution.cdf(z_r[a].abs()));

            if verbose {
                progress = (100.0_f64 * a as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop {} of {}: {}%", (a + 1), files_vec.len(), progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Spatial Autocorrelation</title>
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
                    text-align: centre;
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
            <h1>Spatial Autocorrelation Report</h1>
        ".as_bytes())?;

        // output the names of the input files.
        for a in 0..num_files {
            let value = &file_names[a];
            writer.write_all(
                &format!("<p><strong>Image {}</strong>: {}</p>", a + 1, value).as_bytes(),
            )?;

            writer.write_all("<div><table align=\"center\">".as_bytes())?;
            writer.write_all("<caption>Moran's I Results</caption>".as_bytes())?;

            writer.write_all(
                &format!(
                    "<tr><td>Number of cells included</td><td>{}</td class=\"numberCell\"></tr>",
                    n[a]
                )
                .as_bytes(),
            )?;
            // if (units[a].equals("")) {
            writer.write_all(
                &format!(
                    "<tr><td>Mean of cells included</td><td class=\"numberCell\">{:.*}</td></tr>",
                    4, mean[a]
                )
                .as_bytes(),
            )?;
            // } else {
            //     retstr.append("Mean of cells included:\t\t").append(df2.format(mean[a])).append(" ").append(units[a]).append("\n");
            // }
            writer.write_all(&format!("<tr><td>Spatial autocorrelation (Moran's I)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, i[a]).as_bytes())?;
            writer.write_all(
                &format!(
                    "<tr><td>Expected value</td> <td class=\"numberCell\">{:.*}</td></tr>",
                    4, e_i[a]
                )
                .as_bytes(),
            )?;
            writer.write_all(&format!("<tr><td>Variance of I (normality assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, var_normality[a]).as_bytes())?;
            writer.write_all(&format!("<tr><td>z test stat (normality assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, z_n[a]).as_bytes())?;
            writer.write_all(&format!("<tr><td>p-value (normality assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, p_value_n[a]).as_bytes())?;
            writer.write_all(&format!("<tr><td>Variance of I (randomization assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, var_randomization[a]).as_bytes())?;
            writer.write_all(&format!("<tr><td>z test stat (randomization assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, z_r[a]).as_bytes())?;
            writer.write_all(&format!("<tr><td>p-value (randomization assumption)</td> <td class=\"numberCell\">{:.*}</td></tr>", 4, p_value_r[a]).as_bytes())?;
            writer.write_all("</table></div>".as_bytes())?;
        }

        writer.write_all("</body>".as_bytes())?;

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
            if verbose {
                println!("Complete! Please see {} for output.", output_file);
            }
        }

        Ok(())
    }
}
