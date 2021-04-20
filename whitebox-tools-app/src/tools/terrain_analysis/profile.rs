/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/02/2018
Last Modified: 12/10/2018
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

/// This tool can be used to plot the data profile, along a set of one or more vector lines (`--lines`), in
/// an input (`--surface`) digital elevation model (DEM), or other surface model. The data profile plots
/// surface height (y-axis) against distance along profile (x-axis). The tool outputs an interactive SVG line
/// graph embedded in an HTML document (`--output`). If the vector lines file contains multiple line features,
/// the output plot will contain each of the input profiles.
///
/// If you want to extract the [longitudinal profile](http://www.fao.org/docrep/003/X6841E/X6841E02.HTM) of a river,
/// use the `LongProfile` tool instead.
///
/// # See Also
/// `LongProfile`, `HypsometricAnalysis`
pub struct Profile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Profile {
    pub fn new() -> Profile {
        // public constructor
        let name = "Profile".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Plots profiles from digital surface models.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Line File".to_owned(),
            flags: vec!["--lines".to_owned()],
            description: "Input vector line file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Surface File".to_owned(),
            flags: vec!["--surface".to_owned()],
            description: "Input raster surface file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --lines=profile.shp --surface=dem.tif -o=profile.html", short_exe, name).replace("*", &sep);

        Profile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Profile {
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
        let mut profile_file = String::new();
        let mut surface_file = String::new();
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
            if flag_val == "-lines" {
                profile_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-surface" {
                surface_file = if keyval {
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

        if !profile_file.contains(&sep) && !profile_file.contains("/") {
            profile_file = format!("{}{}", working_directory, profile_file);
        }
        if !surface_file.contains(&sep) && !surface_file.contains("/") {
            surface_file = format!("{}{}", working_directory, surface_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading profile data...")
        };
        let profile_data = Shapefile::read(&profile_file)?;

        if verbose {
            println!("Reading DEM data...")
        };
        let surface = Raster::new(&surface_file, "r")?;

        let start = Instant::now();

        // make sure the input vector file is of lines type
        if profile_data.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of polyline base shape type.",
            ));
        }

        let nodata = surface.configs.nodata;
        // let west = surface.configs.west;
        // let north = surface.configs.north;
        // let cell_size_x = surface.configs.resolution_x;
        // let cell_size_y = surface.configs.resolution_y;

        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut series_names = vec![];
        // let (mut x_st, mut x_end, mut y_st, mut y_end): (f64, f64, f64, f64);
        // let (mut col_st, mut col_end, mut row_st, mut row_end): (f64, f64, f64, f64);
        let (mut st_row, mut st_col, mut end_row, mut end_col): (isize, isize, isize, isize);
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        let (mut row, mut col): (isize, isize);
        let mut z: f64;
        let mut dist: f64;
        let (mut dx, mut dy): (f64, f64);
        let (mut path_dist, mut dist_step): (f64, f64);
        let mut num_steps: isize;

        for record_num in 0..profile_data.num_records {
            let record = profile_data.get_record(record_num);

            for part in 0..record.num_parts as usize {
                let mut profile_xdata = vec![];
                let mut profile_ydata = vec![];

                start_point_in_part = record.parts[part] as usize;
                if part < record.num_parts as usize - 1 {
                    end_point_in_part = record.parts[part + 1] as usize - 1;
                } else {
                    end_point_in_part = record.num_points as usize - 1;
                }

                // row = surface.get_row_from_y(record.points[0].y);
                // col = surface.get_column_from_x(record.points[0].x);
                // z = surface.get_value(row, col);
                dist = 0f64;
                for i in start_point_in_part..end_point_in_part {
                    st_row = surface.get_row_from_y(record.points[i].y);
                    st_col = surface.get_column_from_x(record.points[i].x);
                    end_row = surface.get_row_from_y(record.points[i + 1].y);
                    end_col = surface.get_column_from_x(record.points[i + 1].x);

                    dx = (end_col - st_col) as f64;
                    dy = (end_row - st_row) as f64;

                    path_dist = (dx * dx + dy * dy).sqrt();
                    num_steps = path_dist.ceil() as isize;

                    dx = dx / path_dist;
                    dy = dy / path_dist;

                    dist_step = ((record.points[i].x - record.points[i + 1].x)
                        * (record.points[i].x - record.points[i + 1].x)
                        + (record.points[i].y - record.points[i + 1].y)
                            * (record.points[i].y - record.points[i + 1].y))
                        .sqrt()
                        / path_dist;

                    if num_steps > 0 {
                        for j in 1..num_steps {
                            col = (st_col as f64 + j as f64 * dx) as isize;
                            row = (st_row as f64 + j as f64 * dy) as isize;
                            z = surface.get_value(row, col);
                            dist += dist_step;
                            if z != nodata {
                                profile_xdata.push(dist);
                                profile_ydata.push(z);
                            }
                        }
                    }
                }

                if profile_xdata.len() > 1 {
                    xdata.push(profile_xdata.clone());
                    ydata.push(profile_ydata.clone());
                    if record.num_parts > 1 {
                        series_names.push(format!("Profile {} Part {}", record_num + 1, part + 1));
                    } else {
                        series_names.push(format!("Profile {}", record_num + 1));
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * record_num as f64 / (profile_data.num_records - 1) as f64)
                    as usize;
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
            <title>Profile</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Profile</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(
            (format!(
                "<p><strong>Input Surface</strong>: {}<br>",
                surface.get_short_filename()
            ))
            .as_bytes(),
        )?;

        writer.write_all(("</p>").as_bytes())?;
        let elapsed_time = get_formatted_elapsed_time(start);

        let multiples = xdata.len() > 2 && xdata.len() < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Distance".to_string(),
            y_axis_label: "Elevation".to_string(),
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
