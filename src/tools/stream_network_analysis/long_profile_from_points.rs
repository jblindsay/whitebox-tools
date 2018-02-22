/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: February 21, 2018
Last Modified: February 21, 2018
License: MIT
*/
extern crate time;

use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::env;
use std::path;
use std::f64;
use raster::*;
use vector::{Shapefile, ShapeType};
use std::io::{Error, ErrorKind};
use tools::*;
use rendering::LineGraph;
use rendering::html::*;
use structures::Array2D;

pub struct LongProfileFromPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LongProfileFromPoints {
    pub fn new() -> LongProfileFromPoints { // public constructor
        let name = "LongProfileFromPoints".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Plots the longitudinal profiles from flow-paths initiating from a set of vector points.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input D8 Pointer File".to_owned(), 
            flags: vec!["--d8_pntr".to_owned()], 
            description: "Input raster D8 pointer file.".to_owned(),
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
            name: "Input DEM File".to_owned(), 
            flags: vec!["--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
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
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(), 
            flags: vec!["--esri_pntr".to_owned()], 
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.dep --points=stream_head.shp --dem=dem.dep -o=output.html --esri_pntr", short_exe, name).replace("*", &sep);
    
        LongProfileFromPoints { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for LongProfileFromPoints {
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
        let mut d8_file = String::new();
        let mut points_file = String::new();
        let mut dem_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        
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
            if flag_val == "-d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-points" {
                points_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                esri_style = true;
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

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !points_file.contains(&sep) && !points_file.contains("/") {
            points_file = format!("{}{}", working_directory, points_file);
        }
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading pointer data...") };
        let pntr = Raster::new(&d8_file, "r")?;

        if verbose { println!("Reading points data...") };
        let points = Shapefile::new(&points_file, "r")?;
        
        if verbose { println!("Reading DEM data...") };
        let dem = Raster::new(&dem_file, "r")?;
        
        let start = time::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        
        // make sure the input files have the same size
        if dem.configs.rows != pntr.configs.rows || dem.configs.columns != pntr.configs.columns {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
        }

        // make sure the input vector file is of points type
        if points.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type."));
        }

        let cell_size_x = pntr.configs.resolution_x;
        let cell_size_y = pntr.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        let mut heads = vec![];
        for record_num in 0..points.num_records {
            let record = points.get_record(record_num);
            let row = dem.get_row_from_y(record.points[0].y);
            let col = dem.get_column_from_x(record.points[0].x);
            heads.push((row, col));
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding channel heads: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose { println!("Traversing flowpaths..."); }
        // Now traverse each flowpath starting from each stream head and
        // retrieve the elevation and distance from outlet data.
        let mut xdata = vec![];
        let mut ydata = vec![];
        let series_names = vec![];
        let d_x = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let d_y = [ -1, 0, 1, 1, 1, 0, -1, -1 ];

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
        let grid_lengths = [diag_cell_size, cell_size_x, diag_cell_size, cell_size_y, diag_cell_size, cell_size_x, diag_cell_size, cell_size_y];
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
                    dist += grid_lengths[pntr_matches[dir]];
                    if dist > dist_traversed.get_value(y, x) {
                        dist_traversed.set_value(y, x, dist);
                        link_id.set_value(y, x, traverse_num);
                    }
                } else {
                    flag = false;
                }
            }
            traverse_num += 1;
            stream_lengths[h] = dist;
            // update progress here
            if verbose {
                progress = (100.0_f64 * h as f64 / (num_heads-1) as f64) as usize;
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
                    dist += grid_lengths[pntr_matches[dir]];
                    profile_xdata.push(stream_lengths[h] - dist);
                    profile_ydata.push(dem.get_value(y, x));
                    if link_id.get_value(y, x) != traverse_num {
                        flag = false;
                    }
                } else {
                    flag = false;
                }
            }

            let num_cells = profile_xdata.len();
            if num_cells > 1 {
                if profile_xdata[num_cells-1] == 0f64 {
                    // Otherwise the origin of the plot won't be at zero.
                    profile_xdata[num_cells-1] = 0.0000001f64; 
                }

                xdata.push(profile_xdata.clone());
                ydata.push(profile_ydata.clone());

                traverse_num += 1;
            }
            // update progress here
            if verbose {
                progress = (100.0_f64 * h as f64 / (num_heads-1) as f64) as usize;
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
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Long Profile From Points</title>"#.as_bytes())?;
        
        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;
            
        writer.write_all(&r#"</head>
        <body>
            <h1>Long Profile From Points</h1>"#.as_bytes())?;
        
        writer.write_all((format!("<p><strong>Input DEM</strong>: {}<br>", dem.get_short_filename())).as_bytes())?;
        
        writer.write_all(("</p>").as_bytes())?;
        let end = time::now();
        let elapsed_time = end - start;

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