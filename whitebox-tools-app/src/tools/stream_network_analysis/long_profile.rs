/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 20/02/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::rendering::html::*;
use whitebox_common::rendering::LineGraph;
use whitebox_common::structures::Array2D;
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

/// This tool can be used to create a [longitudinal profile](http://www.fao.org/docrep/003/X6841E/X6841E02.HTM) plot.
/// A longitudinal stream profile is a plot of elevation against downstream distance. Most long profiles use distance
/// from channel head as the distance measure. This tool, however, uses the distance to the stream network outlet cell,
/// or mouth, as the distance measure. The reason for this difference is that while for any one location within a stream
/// network there is only ever one downstream outlet, there is usually many upstream channel heads. Thus plotted using
/// the traditional downstream-distance method, the same point within a network will plot in many different long profile
/// locations, whereas it will always plot on one unique location in the distance-to-mouth method. One consequence of
/// this difference is that the long profile will be oriented from right-to-left rather than left-to-right, as would
/// traditionally be the case.
///
/// The tool outputs an interactive SVG line graph embedded in an HTML document (`--output`). The user must specify the
/// names of a D8 pointer (`--d8_pntr`) image (flow direction), a streams raster image
/// (`--streams`), and a digital elevation model (`--dem`). Stream cells are designated in the streams image as all
/// positive, nonzero values. Thus all non-stream or background grid cells are commonly assigned either zeros or NoData
/// values. The pointer image is used to traverse the stream network and should only be created using the D8 algorithm
/// (`D8Pointer`). The streams image should be derived using a flow accumulation based stream network extraction
/// algorithm, also based on the D8 flow algorithm.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// # See Also
/// `LongProfileFromPoints`, `Profile`, `D8Pointer`
pub struct LongProfile {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LongProfile {
    pub fn new() -> LongProfile {
        // public constructor
        let name = "LongProfile".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Plots the stream longitudinal profiles for one or more rivers.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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

        parameters.push(ToolParameter {
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streams.tif --dem=dem.tif -o=output.html --esri_pntr", short_exe, name).replace("*", &sep);

        LongProfile {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LongProfile {
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
        let mut d8_file = String::new();
        let mut streams_file = String::new();
        let mut dem_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;

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
            if flag_val == "-d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-dem" {
                dem_file = if keyval {
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
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
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

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading pointer data...")
        };
        let pntr = Raster::new(&d8_file, "r")?;
        if verbose {
            println!("Reading streams data...")
        };
        let streams = Raster::new(&streams_file, "r")?;
        if verbose {
            println!("Reading DEM data...")
        };
        let dem = Raster::new(&dem_file, "r")?;

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;

        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        // make sure the input files have the same size
        if dem.configs.rows != pntr.configs.rows || dem.configs.columns != pntr.configs.columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let cell_size_x = pntr.configs.resolution_x;
        let cell_size_y = pntr.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // find all the stream head cells and add them to a list
        if verbose {
            println!("Finding channel heads...");
        }
        let streams = Arc::new(streams);
        let pntr = Arc::new(pntr);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let streams = streams.clone();
            let pntr = pntr.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let inflowing_vals = if !esri_style {
                    [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64]
                } else {
                    [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64]
                };
                let mut num_neighbouring_stream_cells: i8;
                let (mut x, mut y): (isize, isize);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut heads = vec![];
                    for col in 0..columns {
                        if streams.get_value(row, col) > 0.0 {
                            // see if it is a headwater location
                            num_neighbouring_stream_cells = 0i8;
                            for c in 0..8 {
                                x = col + d_x[c];
                                y = row + d_y[c];
                                if streams.get_value(y, x) > 0.0
                                    && pntr.get_value(y, x) == inflowing_vals[c]
                                {
                                    num_neighbouring_stream_cells += 1i8;
                                    break;
                                }
                            }
                            if num_neighbouring_stream_cells == 0i8 {
                                heads.push(col);
                            }
                        }
                    }
                    tx1.send((row, heads)).unwrap();
                }
            });
        }

        let mut heads = vec![];
        for row in 0..rows {
            let (r, data) = rx.recv().expect("Error receiving data from thread.");
            for col in data {
                heads.push((r, col));
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding channel heads: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            println!("Traversing streams...");
        }
        // Now traverse each flowpath starting from each stream head and
        // retrieve the elevation and distance from outlet data.
        let mut xdata = vec![];
        let mut ydata = vec![];
        let series_names = vec![];
        let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
        let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [999usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 0usize;
            pntr_matches[2] = 1usize;
            pntr_matches[4] = 2usize;
            pntr_matches[8] = 3usize;
            pntr_matches[16] = 4usize;
            pntr_matches[32] = 5usize;
            pntr_matches[64] = 6usize;
            pntr_matches[128] = 7usize;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 1usize;
            pntr_matches[2] = 2usize;
            pntr_matches[4] = 3usize;
            pntr_matches[8] = 4usize;
            pntr_matches[16] = 5usize;
            pntr_matches[32] = 6usize;
            pntr_matches[64] = 7usize;
            pntr_matches[128] = 0usize;
        }
        let grid_lengths = [
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
        ];
        let mut flag: bool;
        let (mut x, mut y): (isize, isize);
        let mut dir: usize;
        let mut traverse_num = 1u16;
        let mut dist: f64;
        let num_heads = heads.len();
        let mut dist_traversed: Array2D<f64> = Array2D::new(rows, columns, -1f64, -32768f64)?;
        let mut link_id: Array2D<u16> = Array2D::new(rows, columns, 0, 0)?;
        let mut stream_lengths = vec![0f64; num_heads];
        for h in 0..num_heads {
            let (row, col) = heads[h];
            x = col;
            y = row;
            dist = 0f64;
            dist_traversed.set_value(y, x, dist);
            link_id.set_value(y, x, traverse_num);
            flag = true;
            while flag {
                // find the downslope neighbour
                if pntr.get_value(y, x) > 0.0 {
                    dir = pntr.get_value(y, x) as usize;
                    if dir > 128 || pntr_matches[dir] == 999 {
                        return Err(Error::new(ErrorKind::InvalidInput,
                            "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                    }

                    x += d_x[pntr_matches[dir]];
                    y += d_y[pntr_matches[dir]];
                    if streams.get_value(y, x) <= 0.0 {
                        //it's not a stream cell
                        flag = false;
                    } else {
                        dist += grid_lengths[pntr_matches[dir]];
                        if dist > dist_traversed.get_value(y, x) {
                            dist_traversed.set_value(y, x, dist);
                            link_id.set_value(y, x, traverse_num);
                        }
                    }
                } else {
                    flag = false;
                }
            }
            traverse_num += 1;
            stream_lengths[h] = dist;
            // update progress here
            if verbose {
                progress = (100.0_f64 * h as f64 / (num_heads - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 1 of 2: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for h in 0..num_heads {
            let (row, col) = heads[h];
            traverse_num = link_id.get_value(row, col);
            // series_names.push(format!("Profile {}", traverse_num));
            let mut profile_xdata = vec![];
            let mut profile_ydata = vec![];

            profile_xdata.push(stream_lengths[h]);
            profile_ydata.push(dem.get_value(row, col));

            x = col;
            y = row;
            dist = 0f64;
            flag = true;
            while flag {
                // find the downslope neighbour
                if pntr.get_value(y, x) > 0.0 {
                    dir = pntr.get_value(y, x) as usize;
                    x += d_x[pntr_matches[dir]];
                    y += d_y[pntr_matches[dir]];
                    if streams.get_value(y, x) <= 0.0 {
                        //it's not a stream cell
                        flag = false;
                    } else {
                        dist += grid_lengths[pntr_matches[dir]];
                        profile_xdata.push(stream_lengths[h] - dist);
                        profile_ydata.push(dem.get_value(y, x));
                        if link_id.get_value(y, x) != traverse_num {
                            flag = false;
                        }
                    }
                } else {
                    flag = false;
                }
            }

            let num_cells = profile_xdata.len();
            if num_cells > 1 {
                if profile_xdata[num_cells - 1] == 0f64 {
                    // Otherwise the origin of the plot won't be at zero.
                    profile_xdata[num_cells - 1] = 0.0000001f64;
                }

                xdata.push(profile_xdata.clone());
                ydata.push(profile_ydata.clone());

                traverse_num += 1;
            }
            // update progress here
            if verbose {
                progress = (100.0_f64 * h as f64 / (num_heads - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 2 of 2: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>Long Profile</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Long Profile</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(
            (format!(
                "<p><strong>Input Streams Raster</strong>: {}<br>",
                streams.get_short_filename()
            ))
            .as_bytes(),
        )?;
        writer.write_all(
            (format!(
                "<p><strong>Input DEM</strong>: {}<br>",
                dem.get_short_filename()
            ))
            .as_bytes(),
        )?;

        writer.write_all(("</p>").as_bytes())?;
        let elapsed_time = get_formatted_elapsed_time(start);

        let multiples = traverse_num > 2 && traverse_num < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: "Distance from Mouth".to_string(),
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
