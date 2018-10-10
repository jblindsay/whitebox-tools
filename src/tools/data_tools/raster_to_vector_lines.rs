/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/10/2018
Last Modified: 09/10/2018
License: MIT
*/

use raster::*;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::{Array2D, Point2D};
use time;
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// Converts a raster lines features into vectors.
pub struct RasterToVectorLines {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterToVectorLines {
    pub fn new() -> RasterToVectorLines {
        // public constructor
        let name = "RasterToVectorLines".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Converts a raster lines features into vector of the POLYLINE shapetype".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster Lines File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster lines file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lines.tif -o=lines.shp",
            short_exe, name
        ).replace("*", &sep);

        RasterToVectorLines {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterToVectorLines {
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
        let mut input_file = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = Raster::new(&input_file, "r")?;

        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        // let num_cells = input.num_cells();

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 5u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "VALUE",
            FieldDataType::Real,
            10u8,
            4u8,
        ));

        let mut queue = VecDeque::with_capacity((rows * columns) as usize);

        // calculate the number of neighbouring cells
        let mut num_neighbours: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 1, -1)?;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut z: f64;
        let mut zn: f64;
        let mut count: i8;
        let mut num_cells = 0;
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != 0.0 && z != nodata {
                    count = 0i8;
                    for i in 0..8 {
                        zn = input.get_value(row + dy[i], col + dx[i]);
                        if zn != 0f64 && zn != nodata {
                            count += 1;
                        }
                    }
                    num_neighbours.set_value(row, col, count);
                    if count == 1 {
                        // It's a line end; add it to the queue
                        queue.push_back((row, col));
                    }
                    visited.set_value(row, col, 0);
                    num_cells += 1;
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

        let (mut row, mut col): (isize, isize);
        let (mut x, mut y): (f64, f64);
        let (mut row_n, mut col_n): (isize, isize);
        let mut current_id = 1i32;
        let mut current_val: f64;
        let mut vn: i8;
        let mut flag: bool;
        let mut num_solved_cells = 0;
        let mut r: isize;
        let mut c: isize;
        // let mut found: bool;
        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            row = cell.0;
            col = cell.1;
            if visited.get_value(row, col) == 0 {
                // it's still a non-traced line
                current_val = input.get_value(row, col);
                let mut points = vec![];

                // trace the line
                flag = true;
                while flag {
                    x = input.get_x_from_column(col);
                    y = input.get_y_from_row(row);
                    points.push(Point2D::new(x, y));
                    visited.set_value(row, col, 1);
                    num_solved_cells += 1;

                    // find the highest unvisited neighbour
                    let mut highest = 0i8;
                    // let mut found = false;
                    let mut other_unvisited_neighbours: Vec<(isize, isize)> = Vec::with_capacity(9);
                    r = 0isize;
                    c = 0isize;
                    for i in 0..8 {
                        row_n = row + dy[i];
                        col_n = col + dx[i];
                        vn = visited.get_value(row_n, col_n);
                        count = num_neighbours.get_value(row_n, col_n);
                        if vn == 0 && count > highest {
                            if highest > 0 {
                                other_unvisited_neighbours.push((r, c));
                            }
                            highest = count;
                            r = row_n;
                            c = col_n;
                        // found = true;
                        } else if vn == 0 {
                            other_unvisited_neighbours.push((row_n, col_n));
                        }
                    }
                    if highest == 0 {
                        // we only get here if no other unvisted neighbour was found...end of the line
                        flag = false;
                    // visited.set_value(row, col, 1);
                    } else {
                        row = r;
                        col = c;
                    }
                    if other_unvisited_neighbours.len() > 0 {
                        for a in other_unvisited_neighbours {
                            queue.push_back(a);
                        }
                    }
                }

                if points.len() > 1 {
                    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    sfg.add_part(&points);
                    output.add_record(sfg);
                    output.attributes.add_record(
                        vec![FieldData::Int(current_id), FieldData::Real(current_val)],
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

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
