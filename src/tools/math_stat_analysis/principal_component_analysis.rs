/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/03/2018
Last Modified: 13/10/2018
License: MIT

Note: The current implementation reads every raster into memory at one time. This is
because of the calculation of the co-variances. As such, if the entire image stack can't
fit in memory, the tool will not work.
*/

use crate::na::DMatrix;
use crate::raster::*;
use crate::rendering::html::*;
use crate::rendering::LineGraph;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

pub struct PrincipalComponentAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PrincipalComponentAnalysis {
    pub fn new() -> PrincipalComponentAnalysis {
        // public constructor
        let name = "PrincipalComponentAnalysis".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs a principal component analysis (PCA) on a multi-spectral dataset."
                .to_string();

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
            name: "Output HTML Report File".to_owned(),
            flags: vec!["--out_html".to_owned()],
            description: "Output HTML report file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Num. of Component Images (blank for all)".to_owned(),
            flags: vec!["--num_comp".to_owned()],
            description: "Number of component images to output; <= to num. input images".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Perform Standaradized PCA?".to_owned(),
            flags: vec!["--standardized".to_owned()],
            description: "Perform standardized PCA?".to_owned(),
            parameter_type: ParameterType::Boolean,
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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' --out_html=report.html --num_comp=3 --standardized", short_exe, name).replace("*", &sep);

        PrincipalComponentAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PrincipalComponentAnalysis {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut output_html_file = String::new();
        let mut num_comp = 0usize;
        let mut standardized = false;

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            } else if flag_val == "-out_html" {
                output_html_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-num_comp" {
                num_comp = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap() as usize
                };
            } else if flag_val == "-standardized" {
                standardized = true;
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

        if !output_html_file.contains(&sep) && !output_html_file.contains("/") {
            output_html_file = format!("{}{}", working_directory, output_html_file);
        }

        if !output_html_file.ends_with(".html") {
            output_html_file.push_str(".html");
        }

        let mut cmd = input_files_str.split(";");
        let mut input_files = cmd.collect::<Vec<&str>>();
        if input_files.len() == 1 {
            cmd = input_files_str.split(",");
            input_files = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_files.len();
        if num_files < 3 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect about the input files. At least three inputs are required to operate this tool."));
        }

        let start = Instant::now();

        let mut rows = -1isize;
        let mut columns = -1isize;

        let mut nodata = vec![0f64; num_files];
        let mut average = vec![0f64; num_files];
        let mut num_cells = vec![0f64; num_files];
        let mut input_raster: Vec<Raster> = Vec::with_capacity(num_files);
        let mut wd = "".to_string();
        let mut file_names = vec![];
        println!("Calculating image means...");
        for i in 0..num_files {
            if !input_files[i].trim().is_empty() {
                // quality control on the image file name.
                let mut input_file = input_files[i].trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                // read the image
                // let input = Raster::new(&input_file, "r")?;
                input_raster.push(Raster::new(&input_file, "r")?);

                // get the nodata value, the number of valid cells, and the average
                nodata[i] = input_raster[i].configs.nodata;
                num_cells[i] = input_raster[i].num_valid_cells() as f64;
                average[i] = input_raster[i].calculate_mean();
                file_names.push(input_raster[i].get_short_filename());

                // initialize the rows and column and check that each image has the same dimensions
                if rows == -1 || columns == -1 {
                    rows = input_raster[i].configs.rows as isize;
                    columns = input_raster[i].configs.columns as isize;
                    wd = input_file
                        .replace(&format!("{}.dep", input_raster[i].get_short_filename()), "");
                } else {
                    if input_raster[i].configs.rows as isize != rows
                        || input_raster[i].configs.columns as isize != columns
                    {
                        return Err(Error::new(ErrorKind::InvalidInput,
                            "All input images must share the same dimensions (rows and columns) and spatial extent."));
                    }
                }
            } else {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "There is something incorrect about the input files. At least one is an empty string."));
            }
        }

        if rows == -1 || columns == -1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Something is incorrect with the specified input files.",
            ));
        }

        // Calculate the covariance matrix and total deviations
        let mut image_total_deviations = vec![0f64; num_files];
        let mut covariances = vec![vec![0f64; num_files]; num_files];
        let mut correlation_matrix = vec![vec![0f64; num_files]; num_files];
        let mut z1: f64;
        let mut z2: f64;

        for row in 0..rows {
            for col in 0..columns {
                for i in 0..num_files {
                    z1 = input_raster[i].get_value(row, col);
                    if z1 != nodata[i] {
                        image_total_deviations[i] += (z1 - average[i]) * (z1 - average[i]);

                        for a in 0..num_files {
                            z2 = input_raster[a].get_value(row, col);
                            if z2 != nodata[a] {
                                covariances[i][a] += (z1 - average[i]) * (z2 - average[a]);
                            }
                        }
                    }
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

        for i in 0..num_files {
            for a in 0..num_files {
                correlation_matrix[i][a] = covariances[i][a]
                    / (image_total_deviations[i] * image_total_deviations[a]).sqrt();
            }
        }

        for i in 0..num_files {
            for a in 0..num_files {
                covariances[i][a] = covariances[i][a] / (num_cells[i] - 1f64);
            }
        }

        // Calculate the eigenvalues and eigenvectors
        let cov = if !standardized {
            let mut vals: Vec<f64> = Vec::with_capacity(num_files * num_files);
            for i in 0..num_files {
                for a in 0..num_files {
                    vals.push(covariances[i][a]);
                }
            }
            DMatrix::from_row_slice(num_files, num_files, &vals)
        } else {
            let mut vals: Vec<f64> = Vec::with_capacity(num_files * num_files);
            for i in 0..num_files {
                for a in 0..num_files {
                    vals.push(correlation_matrix[i][a]);
                }
            }
            DMatrix::from_row_slice(num_files, num_files, &vals)
        };

        let eig = cov.clone().symmetric_eigen();
        let eigenvalues = eig.eigenvalues.as_slice().to_vec();
        let eigenvectors = eig.eigenvectors.as_slice().to_vec();

        let mut total_eigenvalue = 0f64;
        for i in 0..num_files {
            total_eigenvalue += eigenvalues[i];
        }

        let mut explained_variance = vec![0f64; num_files];
        for i in 0..num_files {
            explained_variance[i] = 100f64 * eigenvalues[i] / total_eigenvalue;
        }

        // find the order of components from highest explained variance to lowest
        let mut prev_var = 100f64;
        let mut component_order = vec![0usize; num_files];
        for i in 0..num_files {
            let mut k = 0usize;
            let mut max_ev = 0f64;
            for j in 0..num_files {
                if explained_variance[j] > max_ev && explained_variance[j] < prev_var {
                    k = j;
                    max_ev = explained_variance[j];
                }
            }
            component_order[i] = k;
            prev_var = max_ev;
        }

        let mut pc: usize;

        let mut xdata = vec![vec![0f64; num_files]; 1];
        let mut ydata = vec![vec![0f64; num_files]; 1];
        let series_names = vec![];
        for i in 0..num_files {
            pc = component_order[i];
            xdata[0][i] = (i + 1) as f64;
            ydata[0][i] = explained_variance[pc];
        }

        // Output html file
        let f = File::create(output_html_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Principal Component Analysis Report</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"
            </head>
            <body>
                <h1>Principal Component Analysis Report</h1>
                "#
            .as_bytes(),
        )?;

        writer.write_all(("<p><strong>Inputs</strong>:<br>").as_bytes())?;
        for i in 0..num_files {
            writer.write_all((format!("Image {}: {}<br>", i + 1, file_names[i])).as_bytes())?;
        }
        writer.write_all(("</p>").as_bytes())?;

        /////////////////////////////////////////
        // Principal Component Analysis Report //
        /////////////////////////////////////////
        let mut cum_explained_variance = 0f64;
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>PCA Report</caption>".as_bytes())?;
        writer.write_all("<tr><th>PC</th><th>Explained Variance</th><th>Cum. Variance</th><th>Eigenvalue</th><th>Eigenvector</th></tr>".as_bytes())?;
        for j in 0..num_files {
            pc = component_order[j];
            cum_explained_variance += explained_variance[pc];
            let mut s = format!("<td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.*}%</td><td class=\"numberCell\">{:.*}%</td><td class=\"numberCell\">{:.*}</td><td>[", (j+1), 2, explained_variance[pc], 2, cum_explained_variance, 4, eigenvalues[pc]);
            s.push_str(&format!("{:.*}", 6, eigenvectors[pc * num_files]));
            for k in 1..num_files {
                s.push_str(&format!(", {:.*}", 6, eigenvectors[pc * num_files + k]));
            }
            s.push_str("]</td>");
            writer.write_all(&format!("<tr>{}</tr>", s).as_bytes())?;
        }
        writer.write_all("</table></p>".as_bytes())?;

        /////////////////////////////////////////
        // Factor Loadings Report //
        /////////////////////////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>Factor Loadings</caption>".as_bytes())?;
        writer.write_all("<tr><th>Image</th>".as_bytes())?;
        for j in 0..num_files {
            writer.write_all(&format!("<th>PC{}</th>", (j + 1)).as_bytes())?;
        }
        writer.write_all("</tr>".as_bytes())?;
        if !standardized {
            for j in 0..num_files {
                let mut s = format!("<td class=\"numberCell\">{}<td>", (j + 1));
                for k in 0..num_files {
                    pc = component_order[k];
                    let loading = (eigenvectors[pc * num_files + j] * eigenvalues[pc].sqrt())
                        / covariances[j][j].sqrt();
                    s.push_str(&format!("<td class=\"numberCell\">{:.*}</td>", 3, loading));
                }
                writer.write_all(&format!("<t>{}</tr>", s).as_bytes())?;
            }
        } else {
            for j in 0..num_files {
                let mut s = format!("<td class=\"numberCell\">{}</td>", (j + 1));
                for k in 0..num_files {
                    pc = component_order[k];
                    let loading = eigenvectors[pc * num_files + j] * eigenvalues[pc].sqrt();
                    s.push_str(&format!("<td class=\"numberCell\">{:.*}</td>", 3, loading));
                }
                writer.write_all(&format!("<t>{}</tr>", s).as_bytes())?;
            }
        }
        writer.write_all("</table></p>".as_bytes())?;

        writer.write_all("<h2>Scree Plot</h2>".as_bytes())?;
        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 500f64,
            height: 450f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Component".to_string(),
            y_axis_label: "Explained Variance (%)".to_string(),
            draw_points: true,
            draw_gridlines: true,
            draw_legend: false,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

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

            println!("Please see {} for output report.", output_html_file);
        }

        // Output the component images
        for a in 0..num_comp {
            pc = component_order[a];
            let out_file = format!("{}PCA_component{}.dep", wd, (a + 1));
            let mut output = Raster::initialize_using_file(&out_file, &input_raster[0]);
            for row in 0..rows {
                for col in 0..columns {
                    z1 = input_raster[0].get_value(row, col);
                    if z1 != nodata[0] {
                        z1 = 0f64;
                        for k in 0..num_files {
                            z1 += input_raster[k].get_value(row, col)
                                * eigenvectors[pc * num_files + k];
                        }
                        output.set_value(row, col, z1);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Saving component image {}: {}%", (a + 1), progress);
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time = get_formatted_elapsed_time(start);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

            if verbose {
                println!("Saving component image {}...", (a + 1))
            };
            let _ = match output.write() {
                Ok(_) => (),
                Err(e) => return Err(e),
            };
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
