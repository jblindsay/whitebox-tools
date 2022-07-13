/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/03/2018
Last Modified: 13/10/2018
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::rendering::html::*;
use whitebox_common::rendering::LineGraph;
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to plot an image stack profile (i.e. a signature) for a set of points (`--points`) and
/// a multispectral image stack (`--inputs`). The tool outputs an interactive SVG line graph embedded in an
/// HTML document (`--output`). If the input points vector contains multiple points, each input point will
/// be associated with a single line in the output plot. The order of vertices in each signature line is
/// determined by the order of images specified in the `--inputs` parameter. At least two input images are
/// required to run this operation. Note that this tool does not require multispectral images as
/// inputs; other types of data may also be used as the image stack. Also note that the input images should be
/// single-band, continuous greytone rasters. RGB colour images are not good candidates for this tool.
///
/// If you require the raster values to be saved in the vector points file's attribute table, or if you need
/// the raster values to be output as text, you may use the `ExtractRasterValuesAtPoints` tool instead.
///
/// # See Also
/// `ExtractRasterValuesAtPoints`
pub struct ImageStackProfile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImageStackProfile {
    pub fn new() -> ImageStackProfile {
        // public constructor
        let name = "ImageStackProfile".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Plots an image stack profile (i.e. signature) for a set of points and multispectral images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input multispectral image files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["--points".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output HTML file.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='image1.tif;image2.tif;image3.tif' --points=pts.shp -o=output.html", short_exe, name).replace("*", &sep);

        ImageStackProfile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImageStackProfile {
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
        let mut points_file = String::new();
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
            if flag_val == "-i" || flag_val == "-inputs" || flag_val == "-input" {
                input_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-points" {
                points_file = if keyval {
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

        let mut cmd = input_files_str.split(";");
        let mut input_files = cmd.collect::<Vec<&str>>();
        if input_files.len() == 1 {
            cmd = input_files_str.split(",");
            input_files = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_files.len();
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        if !points_file.contains(&sep) && !points_file.contains("/") {
            points_file = format!("{}{}", working_directory, points_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if !output_file.ends_with(".html") {
            output_file.push_str(".html");
        }

        if verbose {
            println!("Reading points data...")
        };
        let points = Shapefile::read(&points_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if points.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let num_points = points.num_records;
        if num_points == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input points file must contain at least one record.",
            ));
        }

        let mut xdata = vec![vec![0f64; num_files]; num_points];
        let mut ydata = vec![vec![0f64; num_files]; num_points];
        let mut series_names = vec![];
        let mut file_names = vec![];
        let mut z: f64;
        let (mut row, mut col): (isize, isize);
        for i in 0..num_files {
            if !input_files[i].trim().is_empty() {
                let mut input_file = input_files[i].trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                let image = Raster::new(&input_file, "r")?;
                let nodata = image.configs.nodata;
                file_names.push(image.get_short_filename());

                for record_num in 0..num_points {
                    if i == 0 {
                        series_names.push(format!("Point {}", record_num + 1));
                    }
                    let record = points.get_record(record_num);
                    row = image.get_row_from_y(record.points[0].y);
                    col = image.get_column_from_x(record.points[0].x);
                    z = image.get_value(row, col);
                    if z != nodata {
                        xdata[record_num][i] = (i + 1) as f64;
                        ydata[record_num][i] = z;
                    } else {
                        xdata[record_num][i] = (i + 1) as f64;
                        ydata[record_num][i] = 0f64; // I'm not sure about this approach.
                                                     // It would be better if it was a break in the line, but this
                                                     // cannot be represented as of yet.
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Image Stack Profile</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Image Stack Profile</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(("<p>Inputs:<br>").as_bytes())?;
        for i in 0..num_files {
            writer.write_all(
                (format!("<strong>Image {}</strong>: {}<br>", i + 1, file_names[i])).as_bytes(),
            )?;
        }

        writer.write_all(("</p>").as_bytes())?;
        let elapsed_time = get_formatted_elapsed_time(start);

        let multiples = num_points > 2 && num_points < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Image".to_string(),
            y_axis_label: "Value".to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

        writer.write("<p><table>".as_bytes()).unwrap();
        writer
            .write("<caption>Profile Data Table</caption>".as_bytes())
            .unwrap();

        writer.write("<tr>".as_bytes()).unwrap();
        writer.write("<th>Image</th>".as_bytes()).unwrap();
        for record_num in 0..num_points {
            writer
                .write(&format!("<th>Point {}</th>", record_num + 1).as_bytes())
                .unwrap();
        }
        writer.write("</tr>".as_bytes()).unwrap();

        for i in 0..num_files {
            writer.write("<tr>".as_bytes()).unwrap();
            writer
                .write(&format!("<td class=\"numberCell\">{}</td>", i + 1).as_bytes())
                .unwrap();
            for record_num in 0..num_points {
                writer
                    .write(
                        &format!("<td class=\"numberCell\">{}</td>", ydata[record_num][i])
                            .as_bytes(),
                    )
                    .unwrap();
            }
            writer.write("</tr>".as_bytes()).unwrap();
        }
        writer.write("</table></p>".as_bytes()).unwrap();
        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
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
