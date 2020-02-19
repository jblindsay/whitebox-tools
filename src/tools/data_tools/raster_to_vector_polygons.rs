/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/12/2019
Last Modified: 25/12/2019
License: MIT
*/

use crate::algorithms::is_clockwise_order;
use crate::raster::*;
use crate::structures::{Array2D, Point2D};
use crate::tools::*;
use crate::vector::*;
use std::collections::VecDeque;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
const EPSILON: f64 = std::f64::EPSILON;

/// Converts a raster dataset to a vector of the POLYGON geometry type. The user must specify
/// the name of a raster file (`--input`) and the name of the output (`--output`) vector. All grid cells containing
/// non-zero, non-NoData values will be considered a point. The vector's attribute table
/// will contain a field called 'VALUE' that will contain the cell value for each point
/// feature.
/// 
/// # See Also
/// `RasterToVectorPoints`, `RasterToVectorLines`
pub struct RasterToVectorPolygons {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterToVectorPolygons {
    pub fn new() -> RasterToVectorPolygons {
        // public constructor
        let name = "RasterToVectorPolygons".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Converts a raster dataset to a vector of the POLYGON shapetype.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Polygons File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygons file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=points.tif -o=out.shp",
            short_exe, name
        )
        .replace("*", &sep);

        RasterToVectorPolygons {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterToVectorPolygons {
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
        /*  Diagram 1:
         *  Cell Numbering
         *  _____________
         *  |     |     |
         *  |  0  |  1  |
         *  |_____|_____|
         *  |     |     |
         *  |  2  |  3  |
         *  |_____|_____|
         *
         */

        /*  Diagram 2:
         *  Edge Numbering (shared edges between cells)
         *  _____________
         *  |     |     |
         *  |     3     |
         *  |__2__|__0__|
         *  |     |     |
         *  |     1     |
         *  |_____|_____|
         *
         */

        /* Diagram 3:
         * Cell Edge Numbering
         *
         *  ___0___
         * |       |
         * |       |
         * 3       1
         * |       |
         * |___2___|
         *
         */

        let mut input_file = String::new();
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

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

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let res_x = input.configs.resolution_x;
        let res_y = input.configs.resolution_y;
        let half_res_x = res_x / 2f64;
        let half_res_y = res_y / 2f64;
        let west = input.configs.west;
        let north = input.configs.north;

        let get_x_from_column = |col| -> f64 { west + half_res_x + col as f64 * res_x};
        let get_y_from_row = |row| -> f64 { north - half_res_y - row as f64 * res_y };

        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 10u8, 0u8));
        output.attributes.add_field(&AttributeField::new(
            "VALUE",
            FieldDataType::Real,
            12u8,
            4u8,
        ));

        let dx = [0, 1, 0, -1, 1, 1, -1, -1];
        let dy = [-1, 0, 1, 0, -1, 1, 1, -1];
        let (mut rn, mut cn): (isize, isize);
        let (mut z, mut zn): (f64, f64);

        // Clump the input raster
        let mut clumps: Array2D<u32> = Array2D::new(rows, columns, 0u32, 0u32)?;
        let mut visited: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut queue = VecDeque::new();
        let mut clump_val = 1u32;
        let mut clump_to_value = vec![];
        clump_to_value.push(0f64); // clump values start at 1
        for row in 0..rows {
            for col in 0..columns {
                z = input.get_value(row, col);
                if z != nodata && z != 0f64 && visited.get_value(row, col) != 1 {
                    clump_to_value.push(z);
                    clumps.set_value(row, col, clump_val);
                    visited.set_value(row, col, 1);
                    queue.push_back((row, col));
                    while let Some(cell) = queue.pop_front() {
                        for n in 0..8 {
                            rn = cell.0 + dy[n];
                            cn = cell.1 + dx[n];
                            zn = input.get_value(rn, cn);
                            if z == zn && visited.get_value(rn, cn) != 1 {
                                clumps.increment(rn, cn, clump_val);
                                visited.set_value(rn, cn, 1);
                                queue.push_back((rn, cn));
                            }
                        }
                    }
                    clump_val += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Clumping polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        drop(input);
        drop(visited);

        let mut edges: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut num_edges: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut cell_edges: u8;
        let mut z: u32;
        let mut zn: u32;
        for row in 0..rows {
            for col in 0..columns {
                z = clumps.get_value(row, col);
                if z != 0 {
                    cell_edges = 0u8;
                    for n in 0..8 {
                        zn = clumps.get_value(row + dy[n], col + dx[n]);
                        if z != zn {
                            cell_edges |= 1u8 << n;
                            if n < 4 { // Edges are only counted on the non-diagonal cells
                                num_edges.increment(row, col, 1u8);
                            }
                        }
                    }
                    edges.set_value(row, col, cell_edges);
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Finding edges: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut geometries = vec![ShapefileGeometry::new(ShapeType::Polygon); clump_val as usize-1];

        let mut cell: (isize, isize);
        let (mut x, mut y): (f64, f64);
        let (mut edge_x, mut edge_y): (f64, f64);
        let mut edge: u8;
        let mut next_edge: u8;
        let mut flag: bool;
        let mut edges_val: u8;
        let edge_update = [3, 0, 1, 2];
        let edge_offsets_pt1_x = [-half_res_x, half_res_x, half_res_x, -half_res_x];
        let edge_offsets_pt1_y = [half_res_y, half_res_y, -half_res_y, -half_res_y];
        let edge_offsets_pt2_x = [half_res_x, half_res_x, -half_res_x, -half_res_x];
        let edge_offsets_pt2_y = [half_res_y, -half_res_y, -half_res_y, half_res_y];
        let (mut p1, mut p2, mut p3): (Point2D, Point2D, Point2D);
        let prec = (5f64*EPSILON).tan();
        for row in 0..rows {
            for col in 0..columns {
                z = clumps.get_value(row, col);
                cell_edges = edges.get_value(row, col);
                if cell_edges > 0u8 && num_edges.get_value(row, col) > 0u8 && z != 0 {
                    // find the seed edge to initiate a trace from
                    for n in 0..4 {
                        if (cell_edges >> n) & 1u8 == 1u8 {
                            // we've found an active edge to start a trace from.
                            let mut points = vec![];
                            cell = (row, col);
                            edge = n;
                            flag = true;
                            while flag {
                                edges_val = edges.get_value(cell.0, cell.1);

                                x = get_x_from_column(cell.1);
                                y = get_y_from_row(cell.0);

                                edge_x = x + edge_offsets_pt1_x[edge as usize];
                                edge_y = y + edge_offsets_pt1_y[edge as usize];
                                points.push(Point2D::new(edge_x, edge_y));

                                edge_x = x + edge_offsets_pt2_x[edge as usize];
                                edge_y = y + edge_offsets_pt2_y[edge as usize];
                                points.push(Point2D::new(edge_x, edge_y));

                                if num_edges.get_value(cell.0, cell.1) > 0u8 {
                                    num_edges.decrement(cell.0, cell.1, 1u8);

                                    // is there an edge with the diagonal?
                                    if (edges_val >> (edge + 4)) & 1u8 == 0u8 {
                                        // There's no edge with the diagonal. Move to the diagonal cell.
                                        cell = (cell.0 + dy[edge as usize + 4], cell.1 + dx[edge as usize + 4]);
                                        edge = edge_update[edge as usize];
                                    } else {
                                        // there is an edge with the diagonal
                                        next_edge = edge + 1; // retrieve the value of the next edge
                                        if next_edge > 3 {
                                            next_edge = 0;
                                        }
                                        // is there an edge with the next edge?
                                        if (edges_val >> next_edge) & 1u8 == 0u8 {
                                            // no, move to the adjacent cell; same edge
                                            cell = (
                                                cell.0 + dy[next_edge as usize],
                                                cell.1 + dx[next_edge as usize],
                                            );
                                            // and remove the last point, since it's an unnecessary vertex along a straight
                                            points.pop();
                                        } else {
                                            // yes, same cell, update edge
                                            edge = next_edge;
                                        }
                                    }
                                } else {
                                    // Stopping condition. We've arrived at the start again.
                                    flag = false;
                                }
                            }

                            if points.len() > 1 {
                                // Remove unnecessary points
                                for a in (1..points.len()-1).rev() {
                                    p1 = points[a-1];
                                    p2 = points[a];
                                    p3 = points[a+1];
                                    if ((p2.y-p1.y)*(p3.x-p2.x)-(p3.y-p2.y)*(p2.x-p1.x)).abs() <= (((p2.x-p1.x)*(p3.x-p2.x)+(p2.y-p1.y)*(p3.y-p2.y))).abs() * prec {
                                        points.remove(a);
                                    }
                                }
                                if geometries[z as usize - 1].num_parts > 0 {
                                    // It's a hole.
                                    if is_clockwise_order(&points) {
                                        points.reverse();
                                    }
                                }
                                geometries[z as usize - 1].add_part(&points);
                            }

                            break;
                        }
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Vectorizing polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for fid in 0..geometries.len() {
            output.add_record(geometries[fid].clone());
            output.attributes.add_record(
                vec![FieldData::Int(fid as i32 + 1), FieldData::Real(clump_to_value[fid + 1])],
                false,
            );

            if verbose {
                progress = (100.0_f64 * fid as f64 / (geometries.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating geometries: {}%", progress);
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
