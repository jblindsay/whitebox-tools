/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: March 1, 2018
Last Modified: March 1, 2018
License: MIT
*/

use time;
use std::io::BufWriter;
use std::env;
use std::path;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::f64;
use raster::*;
use vector::{Shapefile, ShapeType};
use structures::Array2D;
use std::io::{Error, ErrorKind};
use tools::*;
use rendering::LineGraph;
use rendering::html::*;

pub struct MaxElevDevSignature {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MaxElevDevSignature {
    pub fn new() -> MaxElevDevSignature { // public constructor
        let name = "MaxElevDevSignature".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates the maximum elevation deviation over a range of spatial scales and for a set of points.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Input Vector Points File".to_owned(), 
            flags: vec!["--points".to_owned()], 
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(VectorGeometryType::Point)),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output HTML File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output HTML file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Minimum Search Neighbourhood Radius (grid cells)".to_owned(), 
            flags: vec!["--min_scale".to_owned()], 
            description: "Minimum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Maximum Search Neighbourhood Radius (grid cells)".to_owned(), 
            flags: vec!["--max_scale".to_owned()], 
            description: "Maximum search neighbourhood radius in grid cells.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Step Size".to_owned(), 
            flags: vec!["--step".to_owned()], 
            description: "Step size as any positive non-zero integer.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("10".to_owned()),
            optional: false
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --points=sites.tif --output=topo_position.html --min_scale=1 --max_scale=1000 --step=5", short_exe, name).replace("*", &sep);
    
        MaxElevDevSignature { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for MaxElevDevSignature {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut points_file = String::new();
        let mut output_file = String::new();
        let mut min_scale = 1isize;
        let mut max_scale = 100isize;
        let mut step = 10isize;
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters."));
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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                input_file = if keyval {
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
            } else if flag_val == "-min_scale" {
                min_scale = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
                if min_scale < 1 { min_scale = 1; }
            } else if flag_val == "-max_scale" {
                max_scale = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            } else if flag_val == "-step" {
                step = if keyval {
                    vec[1].to_string().parse::<isize>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<isize>().unwrap()
                };
            }
        }

        if max_scale < min_scale { 
            let ms = min_scale;
            min_scale = max_scale; 
            max_scale = ms;
        }

        if max_scale == min_scale {
            max_scale += 1;
        }

        if step < 1 { step = 1; }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !points_file.contains(&sep) && !points_file.contains("/") {
            points_file = format!("{}{}", working_directory, points_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };
        let input = Raster::new(&input_file, "r")?;
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        if verbose { println!("Reading points data...") };
        let points = Shapefile::new(&points_file, "r")?;

        // make sure the input vector file is of points type
        if points.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type."));
        }

        // read the points' corresponding row and columns into a list
        let mut signature_sites = vec![];
        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut series_names = vec![];
        for record_num in 0..points.num_records {
            let record = points.get_record(record_num);
            let row = input.get_row_from_y(record.points[0].y);
            let col = input.get_column_from_x(record.points[0].x);
            if row >= 0 && col >= 0 && row < rows && col < columns {
                signature_sites.push((row, col));
                xdata.push(vec![]);
                ydata.push(vec![]);
                series_names.push(format!("Site {}", record_num + 1));
            }
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding site row/column values: {}%", progress);
                    old_progress = progress;
                }
            }
        }
    
        // create the integral images
        let mut i: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut i2: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut i_n: Array2D<i32> = Array2D::new(rows, columns, 0, -1)?;

        let mut val: f64;
        let mut sum: f64;
        let mut sum_sqr: f64;
        let mut sum_n: i32;
        let (mut i_prev, mut i2_prev): (f64, f64);
        let mut n_prev: i32;
        for row in 0..rows {
            sum = 0f64;
            sum_sqr = 0f64;
            sum_n = 0;
            for col in 0..columns {
                val = input[(row, col)];
                if val == nodata {
                    val = 0f64;
                } else {
                    sum_n += 1;
                }
                sum += val;
                sum_sqr += val * val;
                if row > 0 {
                    i_prev = i[(row - 1, col)];
                    i2_prev = i2[(row - 1, col)];
                    n_prev = i_n[(row - 1, col)];
                    i[(row, col)] = sum + i_prev;
                    i2[(row, col)] = sum_sqr + i2_prev;
                    i_n[(row, col)] = sum_n + n_prev;
                } else {
                    i[(row, col)] = sum;
                    i2[(row, col)] = sum_sqr;
                    i_n[(row, col)] = sum_n;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating integral images: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for site in 0..signature_sites.len() {
            let (row, col) = signature_sites[site];
            let (mut x1, mut x2, mut y1, mut y2): (isize, isize, isize, isize);
            let mut n: i32;
            let (mut mean, mut sum, mut sum_sqr): (f64, f64, f64);
            let (mut v, mut s): (f64, f64);
            let mut z: f64;
            z = input.get_value(row, col);
            if z != nodata {
                for midpoint in (min_scale..max_scale).filter(|s| (s - min_scale) % step == 0) {
                    y1 = row - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }

                    y2 = row + midpoint;
                    if y2 >= rows {
                        y2 = rows - 1;
                    }
                    x1 = col - midpoint - 1;
                    if x1 < 0 {
                        x1 = 0;
                    }

                    x2 = col + midpoint;
                    if x2 >= columns {
                        x2 = columns - 1;
                    }

                    n = i_n[(y2, x2)] + i_n[(y1, x1)] - i_n[(y1, x2)] - i_n[(y2, x1)];
                    if n > 0 {
                        sum = i[(y2, x2)] + i[(y1, x1)] - i[(y1, x2)] - i[(y2, x1)];
                        sum_sqr = i2[(y2, x2)] + i2[(y1, x1)] - i2[(y1, x2)] - i2[(y2, x1)];
                        v = (sum_sqr - (sum * sum) / n as f64) / n as f64;
                        if v > 0f64 {
                            s = v.sqrt();
                            mean = sum / n as f64;
                            xdata[site].push((midpoint * 2 + 1) as f64);
                            ydata[site].push((z - mean) / s);
                        } else {
                            xdata[site].push((midpoint * 2 + 1) as f64);
                            ydata[site].push(0f64);
                        }
                    } else {
                        xdata[site].push((midpoint * 2 + 1) as f64);
                        ydata[site].push(0f64);
                    }
                }
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        if verbose { println!("\n{}",  &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")); }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Maximum Elevation Deviation</title>"#.as_bytes())?;
        
        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;
            
        writer.write_all(&r#"</head>
        <body>
            <h1>Maximum Elevation Deviation</h1>"#.as_bytes())?;
        
        writer.write_all((format!("<p><strong>Input DEM</strong>: {}<br>", input.get_short_filename())).as_bytes())?;
        
        writer.write_all(("</p>").as_bytes())?;
        let end = time::now();
        let elapsed_time = end - start;

        let multiples = xdata.len() > 2 && xdata.len() < 12;

        let graph = LineGraph {
            parent_id: "graph".to_string(),
            width: 700f64,
            height: 500f64,
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(), 
            x_axis_label: "Filter Size (cells)".to_string(),
            y_axis_label: "DEV".to_string(),
            draw_points: false,
            draw_gridlines: true,
            draw_legend: multiples,
            draw_grey_background: false,
        };

        writer.write_all(&format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes())?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        if verbose { println!("\n{}",  &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")); }

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