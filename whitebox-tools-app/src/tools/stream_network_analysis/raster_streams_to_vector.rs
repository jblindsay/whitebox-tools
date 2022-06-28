/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/09/2018
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::{Array2D, Point2D};
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool converts a raster stream file into a vector file. The user must specify: 1)
/// the name of the raster streams file, 2) the name of the D8 flow pointer file,
/// and 3) the name of the output vector file. Streams in the input raster streams
/// file are denoted by cells containing any positive, non-zero integer. A field in
/// the vector database file, called STRM_VAL, will correspond to this positive
/// integer value. The database file will also have a field for the length of each
/// link in the stream network. The flow pointer file must be calculated from a DEM with
/// all topographic depressions and flat areas removed and must be calculated using the
/// D8 flow pointer algorithm. The output vector will contain PolyLine features.
///
/// # See Also
/// `RasterizeStreams`, `RasterToVectorLines`
pub struct RasterStreamsToVector {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterStreamsToVector {
    pub fn new() -> RasterStreamsToVector {
        // public constructor
        let name = "RasterStreamsToVector".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Converts a raster stream file into a vector file.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
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

        parameters.push(ToolParameter {
            name: "Do all stream pixels should be represented by a vertex?".to_owned(),
            flags: vec!["--keep_all_vertices".to_owned()],
            description: "Avoid any simplification of the output vector.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=streams.tif --d8_pntr=D8.tif -o=output.shp
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=streams.tif --d8_pntr=D8.tif -o=output.shp --esri_pntr", short_exe, name).replace("*", &sep);

        RasterStreamsToVector {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterStreamsToVector {
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
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut keep_all_vertices = false;

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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
                };
            } else if flag_val == "-keep_all_vertices" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    keep_all_vertices = true;
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

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let num_cells = pntr.num_cells();
        let nodata = streams.configs.nodata;
        let pntr_nodata = pntr.configs.nodata;

        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;

        // set the projection information
        // output.projection = input.get_wkt();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "STRM_VAL",
            FieldDataType::Real,
            10u8,
            3u8,
        ));

        let mut stack = Vec::with_capacity((rows * columns) as usize);

        // calculate the number of inflowing cells
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let inflowing_vals = if esri_style {
            [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64]
        } else {
            [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64]
        };
        let mut num_solved_cells = 0;
        let mut count: i8;
        for row in 0..rows {
            for col in 0..columns {
                if streams.get_value(row, col) > 0.0 && streams.get_value(row, col) != nodata {
                    count = 0i8;
                    for i in 0..8 {
                        if streams.get_value(row + dy[i], col + dx[i]) > 0.0
                            && streams.get_value(row + dy[i], col + dx[i]) != nodata
                            && pntr.get_value(row + dy[i], col + dx[i]) == inflowing_vals[i]
                        {
                            count += 1;
                        }
                    }
                    num_inflowing.set_value(row, col, count);
                    if count == 0 {
                        // It's a headwater; add it to the stack
                        stack.push((row, col));
                    }
                } else {
                    num_solved_cells += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [999usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in dx and dy.
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
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 1usize;
            pntr_matches[2] = 2usize;
            pntr_matches[4] = 3usize;
            pntr_matches[8] = 4usize;
            pntr_matches[16] = 5usize;
            pntr_matches[32] = 6usize;
            pntr_matches[64] = 7usize;
            pntr_matches[128] = 0usize;
        }

        let (mut row, mut col): (isize, isize);
        let (mut x, mut y): (f64, f64);
        let mut dir: usize;
        let mut prev_dir: usize;
        let mut c: usize;
        let mut current_id = 1i32;
        let mut in_val: f64;
        let mut in_val_n: f64;
        let mut flag: bool;
        let mut already_added_point: bool;
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;

            if num_inflowing.get_value(row, col) != -1i8 {
                in_val = streams.get_value(row, col);
                num_inflowing.set_value(row, col, -1i8);

                let mut points = vec![];

                // descend the flowpath
                prev_dir = 99; // this way the first point in the line is always output.
                flag = true;
                while flag {
                    if pntr.get_value(row, col) != pntr_nodata {
                        dir = pntr.get_value(row, col) as usize;
                        already_added_point = if keep_all_vertices || dir != prev_dir {
                            x = pntr.get_x_from_column(col);
                            y = pntr.get_y_from_row(row);
                            points.push(Point2D::new(x, y));
                            prev_dir = dir;
                            true
                        } else {
                            false
                        };
                        if dir > 0
                            && streams.get_value(row, col) > 0.0
                            && streams.get_value(row, col) != nodata
                        {
                            if dir > 128 || pntr_matches[dir] == 999 {
                                return Err(Error::new(
                                    ErrorKind::InvalidInput,
                                    "An unexpected value has been identified in the pointer image. 
                                This tool requires a pointer grid that has been created using 
                                either the D8 or Rho8 tools.",
                                ));
                            }
                            c = pntr_matches[dir];
                            row += dy[c];
                            col += dx[c];

                            in_val_n = streams.get_value(row, col);
                            if num_inflowing.get_value(row, col) != 1 || in_val_n != in_val {
                                // it's a confluence, so stop descending the flowpath
                                x = pntr.get_x_from_column(col);
                                y = pntr.get_y_from_row(row);
                                points.push(Point2D::new(x, y));

                                // add the confluence to the stack
                                stack.push((row, col));

                                flag = false;
                            }
                        } else {
                            if !already_added_point {
                                // this way the last point in the line is always output.
                                x = pntr.get_x_from_column(col);
                                y = pntr.get_y_from_row(row);
                                points.push(Point2D::new(x, y));
                            }
                            flag = false;
                        }
                    } else {
                        flag = false;
                    }
                }

                if points.len() > 1 {
                    if points[points.len() - 1] == points[points.len() - 2] {
                        points.pop();
                    }
                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&points);
                    output.add_record(sfg);
                    output.attributes.add_record(
                        vec![FieldData::Int(current_id), FieldData::Real(in_val)],
                        false,
                    );

                    current_id += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
