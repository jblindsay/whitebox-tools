/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/03/2018
Last Modified: 18/10/2019
License: MIT
*/

use crate::na::DMatrix;
use whitebox_raster::*;
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

/// Principal component analysis (PCA) is a common data reduction technique that is used to reduce the dimensionality of
/// multi-dimensional space. In the field of remote sensing, PCA is often used to reduce the number of bands of
/// multi-spectral, or hyper-spectral, imagery. Image correlation analysis often reveals a substantial level of correlation
/// among bands of multi-spectral imagery. This correlation represents data redundancy, i.e. fewer images than the number
/// of bands are required to represent the same information, where the information is related to variation within the imagery.
/// PCA transforms the original data set of *n* bands into *n* 'component' images, where each component image is uncorrelated
/// with all other components. The technique works by transforming the axes of the multi-spectral space such that it coincides
/// with the directions of greatest correlation. Each of these new axes are orthogonal to one another, i.e. they are at right
/// angles. PCA is therefore a type of coordinate system transformation. The PCA component images are arranged such that the
/// greatest amount of variance (or information) within the original data set, is contained within the first component and the
/// amount of variance decreases with each component. It is often the case that the majority of the information contained in a
/// multi-spectral data set can be represented by the first three or four PCA components. The higher-order components are often
/// associated with noise in the original data set.
///
/// The user must specify the names of the multiple input images (`--inputs`). Additionally, the user must specify whether to
/// perform a standardized PCA (`--standardized`) and the number of output components (`--num_comp`) to generate (all components
/// will be output unless otherwise specified). A standardized PCA is performed using the correlation matrix rather than the
/// variance-covariance matrix. This is appropriate when the variances in the input images differ substantially, such as would be
/// the case if they contained values that were recorded in different units (e.g. feet and meters) or on different scales (e.g.
/// 8-bit vs. 16 bit).
///
/// Several outputs will be generated when the tool has completed. The PCA report will be embedded within an output (`--output`)
/// HTML file, which should be automatically displayed after the tool has completed. This report contains useful data summarizing
/// the results of the PCA, including the explained variances of each factor, the Eigenvalues and Eigenvectors associated with
/// factors, the factor loadings, and a scree plot. The first table that is in the PCA report lists the amount of explained
/// variance (in non-cumulative and cumulative form), the Eigenvalue, and the Eigenvector for each component. Each of the PCA
/// components refer to the newly created, transformed images that are created by running the tool. The amount of explained
/// variance associated with each component can be thought of as a measure of how much information content within the original
/// multi-spectral data set that a component has. The higher this value is, the more important the component is. This same
/// information is presented in graphical form in the *scree plot*, found at the bottom of the PCA report. The Eigenvalue is
/// another measure of the information content of a component and the eigenvector describes the mathematical transformation
/// (rotation coordinates) that correspond to a particular component image.
///
/// Factor loadings are also output in a table within the PCA text report (second table). These loading values describe the
/// correlation (i.e. *r* values) between each of the PCA components (columns) and the original images (rows). These values
/// show you how the information contained in an image is spread among the components. An analysis of factor loadings can be
/// reveal useful information about the data set. For example, it can help to identify groups of similar images.
///
/// PCA is used to reduce the number of band images necessary for classification (i.e. as a data reduction technique), for
/// noise reduction, and for change detection applications. When used as a change detection technique, the major PCA components
/// tend to be associated with stable elements of the data set while variance due to land-cover change tend to manifest in the
/// high-order, 'change components'. When used as a noise reduction technique, an inverse PCA is generally performed, leaving
/// out one or more of the high-order PCA components, which account for noise variance.
///
/// Note: the current implementation reads every raster into memory at one time. This is because of the calculation of the
/// co-variances. As such, if the entire image stack cannot fit in memory, the tool will likely experience an out-of-memory error.
/// This tool should be run using the `--wd` flag to specify the working directory into which the component images will be
/// written.
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
            flags: vec!["--out_html".to_owned(), "--output".to_owned()],
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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' --output=report.html --num_comp=3 --standardized", short_exe, name).replace("*", &sep);

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
        let mut num_comp_set = false;
        let mut standardized = false;

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
            } else if flag_val == "-out_html" || flag_val == "-output" {
                output_html_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-num_comp" {
                num_comp = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
                num_comp_set = true;
            } else if flag_val == "-standardized" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    standardized = true;
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

        let wd = if working_directory.is_empty() {
            // set the working directory to that of the first input file.
            let p = path::Path::new(input_files[0].trim());
            // let wd = p.parent().unwrap().to_str().unwrap().to_owned();
            format!(
                "{}{}",
                p.parent().unwrap().to_str().unwrap().to_owned(),
                sep
            )
        } else {
            working_directory.clone().to_owned()
        };

        if !output_html_file.contains(&sep) && !output_html_file.contains("/") {
            output_html_file = format!("{}{}", wd, output_html_file);
        }

        if !output_html_file.ends_with(".html") {
            output_html_file.push_str(".html");
        }

        let start = Instant::now();

        let mut rows = -1isize;
        let mut columns = -1isize;

        let mut nodata = vec![0f64; num_files];
        let mut average = vec![0f64; num_files];
        let mut num_cells = vec![0f64; num_files];
        let mut input_raster: Vec<Raster> = Vec::with_capacity(num_files);
        let mut file_names = vec![];
        if verbose {
            println!("Calculating image means...");
        }
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
                <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
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
            let mut s = format!(
                "<td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.*}%</td><td class=\"numberCell\">{:.*}%</td><td class=\"numberCell\">{:.*}</td><td>[", 
                (j+1), 
                2, 
                explained_variance[pc], 
                2, 
                cum_explained_variance, 
                4, 
                eigenvalues[pc]);
            s.push_str(&format!("{:.*}", 6, eigenvectors[pc * num_files]));
            for k in 1..num_files {
                s.push_str(&format!(", {:.*}", 6, eigenvectors[pc * num_files + k]));
            }
            s.push_str("]</td>");
            writer.write_all(&format!("<tr>{}</tr>", s).as_bytes())?;
        }
        writer.write_all("</table></p>".as_bytes())?;

        ////////////////////////////
        // Factor Loadings Report //
        ////////////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>Factor Loadings</caption>".as_bytes())?;
        writer.write_all("<tr><th>Image</th>".as_bytes())?;
        for j in 0..num_files {
            writer.write_all(&format!("<th>PC{}</th>", (j + 1)).as_bytes())?;
        }
        writer.write_all("</tr>".as_bytes())?;
        if !standardized {
            for j in 0..num_files {
                let mut s = format!("<td class=\"numberCell\">{}</td>", (j + 1));
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





        // The following will print a full-precision version of the eigenvectors as a comment (hidden from the user).
        // It is intended to be parsed by the InversePCA tool to perform the inverse operation.

        writer.write_all("\n\n<!-- The following table of full-precision eigenvectors can be used for inverse PCA calculations.".as_bytes())?;
        for a in 0..num_files {
            let mut s = format!("\nEigenvector {}: [", a+1);
            pc = component_order[a];
            for k in 0..num_files {
                if k == 0 {
                    s.push_str(&format!("{}", eigenvectors[pc * num_files + k]));
                } else {
                    s.push_str(&format!(",{}", eigenvectors[pc * num_files + k]));
                }
            }
            s.push_str("]");
            writer.write_all(s.as_bytes())?;
        }

        writer.write_all("\n-->".as_bytes())?;




        writer.write_all("\n</body>".as_bytes())?;

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
        if num_comp == 0 && !num_comp_set {
            // if it's not set, then output all the components.
            num_comp = num_files;
        }
        for a in 0..num_comp {
            pc = component_order[a];
            let out_file = format!("{}PCA_component{}.tif", wd, (a + 1));
            let mut output = Raster::initialize_using_file(&out_file, &input_raster[0]);
            output.configs.data_type = DataType::F32;
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
