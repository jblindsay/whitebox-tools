/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 03/09/2017
Last Modified: 12/10/2018
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::rendering::html::*;
use crate::tools::*;
use num_cpus;
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

/// This tool can be used to estimate the Pearson product-moment correlation coefficient (*r*) between two or
/// more input images (`--inputs`). The *r*-value is a measure of the linear association in the variation of
/// the input variables (images, in this case). The coefficient ranges from -1.0, indicated a perfect negative
/// linear association, to 1.0, indicated a perfect positive linear association. An *r*-value of 0.0 indicates
/// no correlation between the test variables.
///
/// Note that this index is a measure of the linear association; two variables may be strongly related by a
/// non-linear association (e.g. a power function curve) which will lead to an apparent weak association based
/// on the Pearson coefficient. In fact, non-linear associations are very common among spatial variables,
/// e.g. terrain indices such as slope and contributing area. In such cases, it is advisable that the input
/// images are transformed prior to the estimation of the Pearson coefficient, or that an alternative,
/// non-parametric statistic be used, e.g. the Spearman rank correlation coefficient.
///
/// The user must specify the names of two or more input images (`--inputs`). All input images must share the
/// same grid, as the coefficient requires a comparison of a pair of images on a grid-cell-by-grid-cell basis.
/// If more than two image names are selected, the correlation coefficient will be calculated for each pair of
/// images and reported in the HTML output report (`--output`) as a correlation matrix. Caution must be
/// exercised when attempted to estimate the significance of a correlation coefficient derived from image data.
/// The very high *N*-value (essentially the number of pixels in the image pair) means that even small correlation
/// coefficients can be found to be statistically significant, despite being practically insignificant.
///
/// **NoData** values in either of the two input images are ignored during the calculation of the correlation
/// between images.
///
/// # See Also
/// `ImageCorrelationNeighbourhoodAnalysis`, `ImageRegression`, `ImageAutocorrelation`
pub struct ImageCorrelation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImageCorrelation {
    pub fn new() -> ImageCorrelation {
        // public constructor
        let name = "ImageCorrelation".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs image correlation on two or more input images.".to_string();

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
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output HTML file (default name will be based on input file if unspecified)."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.tif, file2.tif, file3.tif\" -o=outfile.html",
                            short_exe,
                            name)
                .replace("*", &sep);

        ImageCorrelation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImageCorrelation {
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
                input_files = if keyval {
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
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

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

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();

        let mut image_totals = vec![0f64; num_files];
        let mut image_n = vec![0f64; num_files];
        let mut image_averages = vec![0f64; num_files];
        let mut correlation_matrix = vec![vec![-99f64; num_files]; num_files];
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
                            z = input[(row, col)];
                            if z != nodata {
                                // image_totals[a] += z;
                                // image_n[a] += 1f64;
                                total += z;
                                n += 1f64;
                            }
                        }
                    }
                    tx.send((total, n)).unwrap();
                });
            }
            for _ in 0..num_procs {
                let (total, n) = rx.recv().expect("Error receiving data from thread.");
                image_totals[a] += total;
                image_n[a] += n;
            }
            image_averages[a] = image_totals[a] / image_n[a];

            if verbose {
                progress = (100.0_f64 * a as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Calculating image averages ({} of {}): {}%",
                        (a + 1),
                        files_vec.len(),
                        progress
                    );
                    old_progress = progress;
                }
            }
        }
        let image_averages = Arc::new(image_averages);

        if verbose {
            println!("Calculating the correlation matrix:");
        }
        let mut i = 0;
        for a in 0..num_files {
            let value = &file_names[a];
            let image1 = Arc::new(Raster::new(&value, "r")?);
            let nodata1 = image1.configs.nodata;
            for b in 0..(i + 1) {
                if a == b {
                    correlation_matrix[a][b] = 1.0;
                } else {
                    let image2 = Arc::new(Raster::new(&file_names[b], "r")?);
                    let nodata2 = image2.configs.nodata;

                    let (tx, rx) = mpsc::channel();
                    for tid in 0..num_procs {
                        let image1 = image1.clone();
                        let image2 = image2.clone();
                        let image_averages = image_averages.clone();
                        let tx = tx.clone();
                        thread::spawn(move || {
                            let mut z1: f64;
                            let mut z2: f64;
                            let mut image1_total_deviation = 0f64;
                            let mut image2_total_deviation = 0f64;
                            let mut total_product_deviations = 0f64;
                            for row in (0..rows).filter(|r| r % num_procs == tid) {
                                for col in 0..columns {
                                    z1 = image1[(row, col)];
                                    z2 = image2[(row, col)];
                                    if z1 != nodata1 && z2 != nodata2 {
                                        image1_total_deviation +=
                                            (z1 - image_averages[a]) * (z1 - image_averages[a]);
                                        image2_total_deviation +=
                                            (z2 - image_averages[b]) * (z2 - image_averages[b]);
                                        total_product_deviations +=
                                            (z1 - image_averages[a]) * (z2 - image_averages[b]);
                                    }
                                }
                            }
                            tx.send((
                                image1_total_deviation,
                                image2_total_deviation,
                                total_product_deviations,
                            ))
                            .unwrap();
                        });
                    }
                    let mut image1_total_deviation = 0f64;
                    let mut image2_total_deviation = 0f64;
                    let mut total_product_deviations = 0f64;
                    for _ in 0..num_procs {
                        let (val1, val2, val3) =
                            rx.recv().expect("Error receiving data from thread.");
                        image1_total_deviation += val1;
                        image2_total_deviation += val2;
                        total_product_deviations += val3;
                    }
                    correlation_matrix[a][b] = total_product_deviations
                        / (image1_total_deviation * image2_total_deviation).sqrt();
                }
            }
            i += 1;

            if verbose {
                progress = (100.0_f64 * a as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Calculating the correlation matrix ({} of {}): {}%",
                        (a + 1),
                        files_vec.len(),
                        progress
                    );
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

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Image Correlation</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Image Correlation Report</h1>"#
                .as_bytes(),
        )?;

        // output the names of the input files.
        writer.write_all("<p><strong>Input files</strong>:</br>".as_bytes())?;
        for a in 0..num_files {
            let value = &file_names[a]; //files_vec[a];
            writer.write_all(
                format!("<strong>Image {}</strong>: {}</br>", a + 1, value).as_bytes(),
            )?;
        }
        writer.write_all("</p>".as_bytes())?;

        writer.write_all("<br><table align=\"center\">".as_bytes())?;
        writer.write_all("<caption>Pearson correlation matrix</caption>".as_bytes())?;

        let mut out_string = String::from("<tr><th></th>");
        for a in 0..num_files {
            out_string.push_str(&format!("<th>Image {}</th>", a + 1));
        }
        out_string.push_str("</tr>");

        for a in 0..num_files {
            out_string.push_str("<tr>");
            out_string.push_str(&format!("<td><strong>Image {}</strong></td>", a + 1));
            for b in 0..num_files {
                let value = correlation_matrix[a][b];
                if value != -99f64 {
                    let value_str = &format!("{:.*}", 4, value);
                    out_string.push_str(&format!("<td>{}</td>", value_str));
                } else {
                    out_string.push_str("<td></td>");
                }
            }
            out_string.push_str("</tr>");
        }

        writer.write_all(out_string.as_bytes())?;

        writer.write_all("</table>".as_bytes())?;
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
