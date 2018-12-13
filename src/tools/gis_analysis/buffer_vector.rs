/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/11/2018
Last Modified: 07/12/2018
License: MIT
*/
extern crate kdtree;

use crate::algorithms::{
    find_split_points_at_line_intersections, interior_point, is_clockwise_order, point_in_poly,
    poly_in_poly,
};
use crate::structures::{BoundingBox, MultiPolyline, Polyline};
use crate::tools::*;
use crate::vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::env;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;

const EPSILON: f64 = std::f64::EPSILON;

/// This tool
///
/// # See Also
///
pub struct BufferVector {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BufferVector {
    pub fn new() -> BufferVector {
        // public constructor
        let name = "BufferVector".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Removes the interior, or shared, boundaries within a vector polygon coverage."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Distance".to_owned(),
            flags: vec!["--dist".to_owned(), "--distance".to_owned()],
            description: "Buffer distance.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("10.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Dissolve overlapping polygons?".to_owned(),
            flags: vec!["--dissolve".to_owned()],
            description: "Optional flag to request the output polygons be dissolved.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("True".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Snap Tolerance".to_owned(),
            flags: vec!["--snap".to_owned()],
            description: "Snap tolerance.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -input=layer1.shp -o=out_file.shp --dist=25.0 --dissolve",
            short_exe, name
        ).replace("*", &sep);

        BufferVector {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BufferVector {
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
        let mut distance = 10f64;
        let mut dissolve = false;
        let mut precision = EPSILON;

        // read the arguments
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
            if flag_val == "-i" || flag_val.contains("-input") {
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
            } else if flag_val.contains("-dist") {
                distance = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-dissolve" {
                dissolve = true;
            } else if flag_val == "-snap" {
                precision = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
                if precision < EPSILON {
                    precision = EPSILON;
                }
            }
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Shapefile::read(&input_file)?;
        let projection = input.projection.clone();

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&output_file, &input, ShapeType::Polygon, false)?;
        output.projection = projection;

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 7u8, 0u8));

        let num_vertices_in_circle = 32usize;
        let angular_resolution = 2f64 * std::f64::consts::PI / num_vertices_in_circle as f64;
        let (mut x, mut y): (f64, f64);
        let mut p: Point2D;
        let (mut slope, mut slope1, mut slope2): (f64, f64, f64);
        let (mut slope3, mut slope4): (f64, f64);
        let mut polygons: Vec<Polyline> = Vec::with_capacity(input.num_records);

        match input.header.shape_type.base_shape_type() {
            ShapeType::Point => {
                if distance < 0f64 {
                    return Err(Error::new(ErrorKind::InvalidInput, "Error: distance < 0.0"));
                }
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    p = record.points[0];

                    let mut circle =
                        Polyline::new_with_capacity(record_num, num_vertices_in_circle + 1);
                    for i in 0..num_vertices_in_circle {
                        slope = i as f64 * angular_resolution;
                        x = p.x + distance * slope.sin();
                        y = p.y + distance * slope.cos();
                        circle.push(Point2D::new(x, y));
                    }

                    // now add a last point same as the first.
                    circle.close_line();

                    polygons.push(circle);

                    if verbose {
                        progress = (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            ShapeType::MultiPoint => {
                if distance < 0f64 {
                    return Err(Error::new(ErrorKind::InvalidInput, "Error: distance < 0.0"));
                }
                let mut total_points = 0;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    total_points += record.points.len();
                }
                let mut points_read = 0;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    for i in 0..record.points.len() {
                        points_read += 1;
                        p = record.points[i];

                        let mut circle =
                            Polyline::new_with_capacity(points_read, num_vertices_in_circle + 1);
                        for i in 0..num_vertices_in_circle {
                            slope = i as f64 * angular_resolution;
                            x = p.x + distance * slope.sin();
                            y = p.y + distance * slope.cos();
                            circle.push(Point2D::new(x, y));
                        }

                        // now add a last point same as the first.
                        circle.close_line();

                        polygons.push(circle);
                        if verbose {
                            progress =
                                (100.0_f64 * points_read as f64 / total_points as f64) as usize;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }
                }
            }
            ShapeType::PolyLine => {
                if distance < 0f64 {
                    return Err(Error::new(ErrorKind::InvalidInput, "Error: distance < 0.0"));
                }
                let right_angle = std::f64::consts::PI / 2f64;
                let mut slope: f64;
                // let mut slopes = Vec::with_capacity(num_vertices_in_circle + 4);
                let mut total_points = 0;
                let (mut x, mut y): (f64, f64);
                let (mut x_translated, mut y_translated): (f64, f64);
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    total_points += record.points.len();
                }
                let mut p1: Point2D;
                let mut p2: Point2D;
                let mut p3: Point2D;
                // output.header.shape_type = ShapeType::PolyLine;
                for record_num in 0..input.num_records {
                    let record = input.get_record(record_num);
                    let mut line_polys: Vec<Polyline> = Vec::with_capacity(input.num_records);
                    let mut line1 = Polyline::new_empty(record_num);
                    let mut line2 = Polyline::new_empty(record_num);
                    for i in 0..record.points.len() {
                        p = record.points[i];
                        if i == 0 || i == record.points.len() - 1 {
                            slope = if i == 0 {
                                (record.points[i + 1].y - p.y).atan2(record.points[i + 1].x - p.x)
                            } else {
                                (p.y - record.points[i - 1].y).atan2(p.x - record.points[i - 1].x)
                            };

                            slope1 = slope + right_angle;
                            // if slope1 < 0.0 {
                            //     slope1 += 2.0 * PI;
                            // }
                            // if slope1 > 2.0 * PI {
                            //     slope1 -= 2.0 * PI;
                            // }
                            slope2 = slope - right_angle;
                            // if slope2 < 0.0 {
                            //     slope2 += 2.0 * PI;
                            // }
                            // if slope2 > 2.0 * PI {
                            //     slope2 -= 2.0 * PI;
                            // }

                            if i == 0 {
                                // x = p.x + distance * slope2.cos();
                                // y = p.y + distance * slope2.sin();
                                // line1.push(Point2D::new(x, y));
                                for j in 0..(num_vertices_in_circle / 2 + 1) {
                                    slope = slope2 - j as f64 * angular_resolution;
                                    x = p.x + distance * slope.cos();
                                    y = p.y + distance * slope.sin();
                                    line1.push(Point2D::new(x, y));
                                }
                            // x = p.x + distance * slope1.cos();
                            // y = p.y + distance * slope1.sin();
                            // line1.push(Point2D::new(x, y));
                            } else {
                                // x = p.x + distance * slope1.cos();
                                // y = p.y + distance * slope1.sin();
                                // line1.push(Point2D::new(x, y));
                                for j in 0..(num_vertices_in_circle / 2 + 1) {
                                    slope = slope1 - j as f64 * angular_resolution;
                                    x = p.x + distance * slope.cos();
                                    y = p.y + distance * slope.sin();
                                    line1.push(Point2D::new(x, y));
                                }
                                // x = p.x + distance * slope2.cos();
                                // y = p.y + distance * slope2.sin();
                                // line1.push(Point2D::new(x, y));
                            }

                            x = p.x + distance * slope2.cos();
                            y = p.y + distance * slope2.sin();
                            line2.push(Point2D::new(x, y));

                        // let mut added_slope1 = false;
                        // let mut added_slope2 = false;
                        // let mut circle =
                        //     Polyline::new_with_capacity(i, num_vertices_in_circle + 3);
                        // for j in 0..num_vertices_in_circle {
                        //     slope = j as f64 * angular_resolution;
                        //     if j > 0 {
                        //         if slope1 > ((j - 1) as f64 * angular_resolution)
                        //             && slope1 < slope
                        //         {
                        //             x = p.x + distance * slope1.cos();
                        //             y = p.y + distance * slope1.sin();
                        //             circle.push(Point2D::new(x, y));
                        //             added_slope1 = true;
                        //         }
                        //         if slope2 > ((j - 1) as f64 * angular_resolution)
                        //             && slope2 < slope
                        //         {
                        //             x = p.x + distance * slope2.cos();
                        //             y = p.y + distance * slope2.sin();
                        //             circle.push(Point2D::new(x, y));
                        //             added_slope2 = true;
                        //         }
                        //     }
                        //     if slope == slope1 {
                        //         added_slope1 = true;
                        //     }
                        //     if slope == slope2 {
                        //         added_slope2 = true;
                        //     }
                        //     // if (added_slope1 && !added_slope2)
                        //     //     || (!added_slope1 && added_slope2)
                        //     // {
                        //     x = p.x + distance * slope.cos();
                        //     y = p.y + distance * slope.sin();
                        //     circle.push(Point2D::new(x, y));
                        //     // }
                        // }

                        // if !added_slope1 {
                        //     x = p.x + distance * slope1.cos();
                        //     y = p.y + distance * slope1.sin();
                        //     circle.push(Point2D::new(x, y));
                        // }

                        // if !added_slope2 {
                        //     x = p.x + distance * slope2.cos();
                        //     y = p.y + distance * slope2.sin();
                        //     circle.push(Point2D::new(x, y));
                        // }

                        // // now add a last point same as the first.
                        // circle.close_line();
                        // line_polys.push(circle);

                        // x_translated = p.x + distance * slope1.cos();
                        // y_translated = p.y + distance * slope1.sin();
                        // line1.push(Point2D::new(x_translated, y_translated));

                        // x_translated = p.x + distance * slope2.cos();
                        // y_translated = p.y + distance * slope2.sin();
                        // line2.push(Point2D::new(x_translated, y_translated));
                        } else {
                            slope =
                                (p.y - record.points[i - 1].y).atan2(p.x - record.points[i - 1].x);

                            slope1 = slope + right_angle;
                            // if slope1 < 0.0 {
                            //     slope1 += 2.0 * PI;
                            // }
                            // if slope1 > 2.0 * PI {
                            //     slope1 -= 2.0 * PI;
                            // }
                            slope2 = slope - right_angle;
                            // if slope2 < 0.0 {
                            //     slope2 += 2.0 * PI;
                            // }
                            // if slope2 > 2.0 * PI {
                            //     slope2 -= 2.0 * PI;
                            // }

                            x = p.x + distance * slope1.cos();
                            y = p.y + distance * slope1.sin();
                            line1.push(Point2D::new(x, y));
                            // line_polys.push(line1);
                            // line1 = Polyline::new_empty(record_num);

                            x = p.x + distance * slope2.cos();
                            y = p.y + distance * slope2.sin();
                            line2.push(Point2D::new(x, y));
                            // line_polys.push(line2);
                            // line2 = Polyline::new_empty(record_num);

                            slope =
                                (record.points[i + 1].y - p.y).atan2(record.points[i + 1].x - p.x);

                            slope3 = slope + right_angle;
                            // if slope3 < 0.0 {
                            //     slope3 += 2.0 * PI;
                            // }
                            // if slope3 > 2.0 * PI {
                            //     slope3 -= 2.0 * PI;
                            // }
                            slope4 = slope - right_angle;
                            // if slope4 < 0.0 {
                            //     slope4 += 2.0 * PI;
                            // }
                            // if slope4 > 2.0 * PI {
                            //     slope4 -= 2.0 * PI;
                            // }

                            if record.points[i + 1].is_left(&record.points[i - 1], &p) < 0.0 {
                                // if Point2D::change_in_heading(
                                //     record.points[i - 1],
                                //     p,
                                //     record.points[i + 1],
                                // ) > 0.0
                                // {
                                let mut slope_change = slope3 - slope1;

                                let mut num_ticks =
                                    (slope_change.abs() / angular_resolution).floor() as usize;
                                if num_ticks > 0 {
                                    if slope_change > 0.0 {
                                        slope_change -= 2.0 * PI;
                                        num_ticks = (slope_change.abs() / angular_resolution)
                                            .floor()
                                            as usize;
                                    }
                                    for j in 0..num_ticks {
                                        slope = slope1 - j as f64 * angular_resolution;
                                        x = p.x + distance * slope.cos();
                                        y = p.y + distance * slope.sin();
                                        line1.push(Point2D::new(x, y));
                                    }
                                }

                                line_polys.push(line2);
                                line2 = Polyline::new_empty(record_num);
                            } else if record.points[i + 1].is_left(&record.points[i - 1], &p) > 0.0
                            {
                                // } else if Point2D::change_in_heading(
                                //     record.points[i - 1],
                                //     p,
                                //     record.points[i + 1],
                                // ) < 0.0
                                // {
                                let mut slope_change = slope4 - slope2;
                                let mut num_ticks =
                                    (slope_change.abs() / angular_resolution).floor() as usize;
                                if num_ticks > 0 {
                                    if slope_change < 0.0 {
                                        slope_change += 2.0 * PI;
                                        num_ticks = (slope_change.abs() / angular_resolution)
                                            .floor()
                                            as usize;
                                    }
                                    for j in 0..num_ticks {
                                        slope = slope2 + j as f64 * angular_resolution;
                                        x = p.x + distance * slope.cos();
                                        y = p.y + distance * slope.sin();
                                        line2.push(Point2D::new(x, y));
                                    }
                                }

                                // line1.push(Point2D::new(0.0, 0.0));
                                line_polys.push(line1);
                                line1 = Polyline::new_empty(record_num);
                                // for j in 0..num_ticks {
                                //     slope = slope1 + j as f64 * angular_resolution;
                                //     x = p.x + distance * slope.cos();
                                //     y = p.y + distance * slope.sin();
                                //     line1.push(Point2D::new(x, y));
                                // }
                            }

                            //         let mut added_slope1 = false;
                            //         let mut added_slope2 = false;
                            //         let mut added_slope3 = false;
                            //         let mut added_slope4 = false;
                            //         // let mut circle =
                            //         //     Polyline::new_with_capacity(i, num_vertices_in_circle + 3);
                            //         for j in 0..num_vertices_in_circle {
                            //             slope = j as f64 * angular_resolution;
                            //             if j > 0 {
                            //                 if slope1 > ((j - 1) as f64 * angular_resolution)
                            //                     && slope1 < slope
                            //                 {
                            //                     x = p.x + distance * slope1.cos();
                            //                     y = p.y + distance * slope1.sin();
                            //                     line1.push(Point2D::new(x, y));
                            //                     // circle.push(Point2D::new(x, y));
                            //                     added_slope1 = true;
                            //                 }
                            //                 if slope2 > ((j - 1) as f64 * angular_resolution)
                            //                     && slope2 < slope
                            //                 {
                            //                     x = p.x + distance * slope2.cos();
                            //                     y = p.y + distance * slope2.sin();
                            //                     line2.push(Point2D::new(x, y));
                            //                     // circle.push(Point2D::new(x, y));
                            //                     added_slope2 = true;
                            //                 }

                            //                 if slope3 > ((j - 1) as f64 * angular_resolution)
                            //                     && slope3 < slope
                            //                 {
                            //                     x = p.x + distance * slope3.cos();
                            //                     y = p.y + distance * slope3.sin();
                            //                     line1.push(Point2D::new(x, y));
                            //                     // circle.push(Point2D::new(x, y));
                            //                     added_slope3 = true;
                            //                 }

                            //                 if slope4 > ((j - 1) as f64 * angular_resolution)
                            //                     && slope4 < slope
                            //                 {
                            //                     x = p.x + distance * slope4.cos();
                            //                     y = p.y + distance * slope4.sin();
                            //                     line2.push(Point2D::new(x, y));
                            //                     // circle.push(Point2D::new(x, y));
                            //                     added_slope4 = true;
                            //                 }
                            //             }
                            //             if slope == slope1 {
                            //                 added_slope1 = true;
                            //             }
                            //             if slope == slope2 {
                            //                 added_slope2 = true;
                            //             }
                            //             if slope == slope3 {
                            //                 added_slope3 = true;
                            //             }
                            //             if slope == slope4 {
                            //                 added_slope4 = true;
                            //             }

                            //             if (added_slope1 && !added_slope3)
                            //                 || (!added_slope1 && added_slope3)
                            //             {
                            //                 x = p.x + distance * slope.cos();
                            //                 y = p.y + distance * slope.sin();
                            //                 line1.push(Point2D::new(x, y));
                            //             }

                            //             if (added_slope2 && !added_slope4)
                            //                 || (!added_slope2 && added_slope4)
                            //             {
                            //                 x = p.x + distance * slope.cos();
                            //                 y = p.y + distance * slope.sin();
                            //                 line2.push(Point2D::new(x, y));
                            //             }

                            //             // if (added_slope1 && !added_slope3)
                            //             //     || (!added_slope1 && added_slope3)
                            //             //     || (added_slope2 && !added_slope4)
                            //             //     || (!added_slope2 && added_slope4)
                            //             // {
                            //             //     x = p.x + distance * slope.cos();
                            //             //     y = p.y + distance * slope.sin();
                            //             //     circle.push(Point2D::new(x, y));
                            //             // }
                            //         }

                            //         if !added_slope1 {
                            //             x = p.x + distance * slope1.cos();
                            //             y = p.y + distance * slope1.sin();
                            //             line1.push(Point2D::new(x, y));
                            //             // circle.push(Point2D::new(x, y));
                            //         }

                            //         if !added_slope2 {
                            //             x = p.x + distance * slope2.cos();
                            //             y = p.y + distance * slope2.sin();
                            //             line2.push(Point2D::new(x, y));
                            //             // circle.push(Point2D::new(x, y));
                            //         }

                            //         if !added_slope3 {
                            //             x = p.x + distance * slope3.cos();
                            //             y = p.y + distance * slope3.sin();
                            //             line1.push(Point2D::new(x, y));
                            //             // circle.push(Point2D::new(x, y));
                            //         }

                            //         if !added_slope4 {
                            //             x = p.x + distance * slope4.cos();
                            //             y = p.y + distance * slope4.sin();
                            //             line2.push(Point2D::new(x, y));
                            //             // circle.push(Point2D::new(x, y));
                            //         }

                            x = p.x + distance * slope3.cos();
                            y = p.y + distance * slope3.sin();
                            line1.push(Point2D::new(x, y));

                            x = p.x + distance * slope4.cos();
                            y = p.y + distance * slope4.sin();
                            line2.push(Point2D::new(x, y));

                            // now add a last point same as the first.
                            // circle.close_line();
                            // line_polys.push(circle);
                        }
                    }

                    line_polys.push(line1);
                    line_polys.push(line2);

                    let dissolved = dissolve_polygons(line_polys, precision);
                    for i in 0..dissolved.len() {
                        for j in 0..dissolved[i].len() {
                            polygons.push(dissolved[i][j].clone());
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * record_num as f64 / (input.num_records - 1) as f64)
                            as usize;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            // ShapeType::Polygon => {}
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid input data ShapeType.",
                ));
            }
        }

        dissolve = false;

        if dissolve {
            if verbose {
                println!("Dissolving polygons...");
            }
            let dissolved = dissolve_polygons(polygons, precision);

            output.header.shape_type = ShapeType::PolyLine;
            for record_num in 0..dissolved.len() {
                // output the polygon
                let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                for i in 0..dissolved[record_num].len() {
                    sfg.add_part(&(dissolved[record_num][i].vertices));
                }
                output.add_record(sfg);

                output
                    .attributes
                    .add_record(vec![FieldData::Int(record_num as i32 + 1i32)], false);

                if verbose {
                    progress =
                        (100.0_f64 * (record_num + 1) as f64 / dissolved.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            output.header.shape_type = ShapeType::PolyLine;
            for record_num in 0..polygons.len() {
                // output the polygon
                let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                sfg.add_part(&polygons[record_num].vertices);
                output.add_record(sfg);

                output
                    .attributes
                    .add_record(vec![FieldData::Int(record_num as i32 + 1i32)], false);

                if verbose {
                    progress =
                        (100.0_f64 * (record_num + 1) as f64 / polygons.len() as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

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

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Link {
    id: usize,
    priority: f64,
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        (self.priority - other.priority).abs() < EPSILON && self.id == other.id
    }
}

impl Eq for Link {}

impl Ord for Link {
    fn cmp(&self, other: &Link) -> Ordering {
        // this sorts priorities from low to high
        // and when priorities are equal, id's from
        // high to low.
        let mut ord = other.priority.partial_cmp(&self.priority).unwrap();
        if ord == Ordering::Equal {
            ord = self.id.cmp(&other.id);
        }
        ord
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Link) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn get_other_endnode(index: usize) -> usize {
    if index % 2 == 0 {
        // it's a starting node and we need the end
        return index + 1;
    }
    // it's an end node and we need the starting node
    index - 1
}

fn is_first_node(index: usize) -> bool {
    index % 2 == 0
}

fn first_node_id(polyline: usize) -> usize {
    polyline * 2
}

fn last_node_id(polyline: usize) -> usize {
    polyline * 2 + 1
}

pub fn dissolve_polygons(polygons: Vec<Polyline>, precision: f64) -> Vec<MultiPolyline> {
    let mut polygons2 = polygons.clone();

    let mut feature_geometries: Vec<MultiPolyline> = vec![];
    for i in 0..polygons2.len() {
        let mut mp = MultiPolyline::new(i + 1);
        mp.push(&polygons2[i]);
        feature_geometries.push(mp);
    }

    return feature_geometries;

    // Remove any zero-length line segments
    for i in 0..polygons2.len() {
        for j in (1..polygons2[i].len()).rev() {
            if polygons2[i][j] == polygons2[i][j - 1] {
                polygons2[i].remove(j);
            }
        }
    }
    // Remove any single-point lines result from above.
    for i in (0..polygons2.len()).rev() {
        if polygons2[i].len() < 2 {
            polygons2.remove(i);
        }
    }

    // Find duplicate polylines and remove them
    let mut duplicate = vec![false; polygons2.len()];
    for i in 0..polygons2.len() {
        if !duplicate[i] {
            for j in (i + 1)..polygons2.len() {
                if polygons2[i].nearly_equals(&polygons2[j], precision) {
                    duplicate[j] = true;
                    break;
                }
            }
        }
    }
    for i in (0..polygons2.len()).rev() {
        if duplicate[i] {
            polygons2.remove(i);
        }
    }

    // hunt for intersections
    let mut features_bb = Vec::with_capacity(polygons2.len());
    for i in 0..polygons2.len() {
        features_bb.push(polygons2[i].get_bounding_box());
    }

    let mut polylines = vec![];
    let mut lengths = vec![];
    let mut line_length: f64;
    for i in 0..polygons2.len() {
        let mut pl = polygons2[i].clone();
        for j in i + 1..polygons2.len() {
            if features_bb[i].overlaps(features_bb[j]) {
                // find any intersections between the polylines
                find_split_points_at_line_intersections(&mut pl, &mut (polygons2[j]));
            }
        }
        let split_lines = pl.split();
        for j in 0..split_lines.len() {
            line_length = split_lines[j].length();
            if line_length > precision {
                polylines.push(split_lines[j].clone());
                lengths.push(line_length);
            }
        }
    }

    // Remove any zero-length line segments
    for i in 0..polylines.len() {
        for j in (1..polylines[i].len()).rev() {
            if polylines[i][j] == polylines[i][j - 1] {
                polylines[i].remove(j);
            }
        }
    }
    // Remove any single-point lines result from above.
    for i in (0..polylines.len()).rev() {
        if polylines[i].len() < 2 {
            polylines.remove(i);
        }
    }

    // let mut interior_line = vec![false; polylines.len()];
    // for i in 0..polylines.len() {
    //     let bb = polylines[i].get_bounding_box();
    //     let test_point = Point2D::midpoint(&(polylines[i][0]), &(polylines[i][1]));
    //     for j in 0..polygons2.len() {
    //         if bb.overlaps(features_bb[j]) && polylines[i].id != polygons2[j].id {
    //             if point_in_poly(&test_point, &polygons2[j].vertices) {
    //                 interior_line[i] = true;
    //                 break;
    //             }
    //         }
    //     }
    // }

    // for i in (0..polylines.len()).rev() {
    //     if interior_line[i] {
    //         polylines.remove(i);
    //     }
    // }

    // let mut feature_geometries: Vec<MultiPolyline> = vec![];
    // for i in 0..polylines.len() {
    //     let mut mp = MultiPolyline::new(i + 1);
    //     mp.push(&polylines[i]);
    //     feature_geometries.push(mp);
    // }

    // return feature_geometries;

    let num_endnodes = polylines.len() * 2;
    /*
        The structure of endnodes is as such:
        1. the starting node for polyline 'a' is a * 2.
        2. the ending node for polyline 'a' is a * 2 + 1.
        3. endnode to polyline = e / 2
        4. is an endnode a starting point? e % 2 == 0
    */
    let mut endnodes: Vec<Vec<usize>> = vec![vec![]; num_endnodes];

    // now add the endpoints of each polyline into a kd tree
    let dimensions = 2;
    let capacity_per_node = 64;
    let mut kdtree = KdTree::new_with_capacity(dimensions, capacity_per_node);
    let mut p1: Point2D;
    let mut p2: Point2D;
    let mut p3: Point2D;
    let mut p4: Point2D;
    for i in 0..polylines.len() {
        p1 = polylines[i].first_vertex();
        kdtree.add([p1.x, p1.y], first_node_id(i)).unwrap();

        p2 = polylines[i].last_vertex();
        kdtree.add([p2.x, p2.y], last_node_id(i)).unwrap();
    }

    // Find the neighbours of each endnode and check for dangling arcs
    // and self-closing arcs which form single-line polys.
    let mut is_acyclic_arc = vec![false; polylines.len()];
    let mut node_angles: Vec<Vec<f64>> = vec![vec![]; num_endnodes];
    let mut heading: f64;
    for i in 0..polylines.len() {
        p1 = polylines[i].first_vertex();
        p2 = polylines[i].last_vertex();

        // check the first vertex
        let ret = kdtree
            .within(&[p1.x, p1.y], precision, &squared_euclidean)
            .unwrap();
        if ret.len() == 1 {
            is_acyclic_arc[i] = true;
        } else {
            p3 = polylines[i][1];
            for a in 0..ret.len() {
                let index = *ret[a].1;
                if index == last_node_id(i) && polylines[i].len() <= 2 {
                    is_acyclic_arc[i] = true;
                }
                if index != first_node_id(i) && !is_acyclic_arc[index / 2] {
                    p4 = if is_first_node(index) {
                        polylines[index / 2][1]
                    } else {
                        polylines[index / 2][polylines[index / 2].len() - 2]
                    };
                    heading = Point2D::change_in_heading(p3, p1, p4);
                    if !heading.is_nan() {
                        node_angles[first_node_id(i)].push(heading);
                        endnodes[first_node_id(i)].push(index);
                    }
                }
            }
            if endnodes[first_node_id(i)].len() == 0 {
                is_acyclic_arc[i] = true;
            }
        }

        // the first vertex showed that this line is a dangling arc,
        // don't bother connecting it to the graph
        if !is_acyclic_arc[i] {
            // check the last vertex
            let ret = kdtree
                .within(&[p2.x, p2.y], precision, &squared_euclidean)
                .unwrap();
            if ret.len() == 1 {
                is_acyclic_arc[i] = true;
            } else {
                p3 = polylines[i][polylines[i].len() - 2];
                for a in 0..ret.len() {
                    let index = *ret[a].1;
                    if index != last_node_id(i) && !is_acyclic_arc[index / 2] {
                        p4 = if is_first_node(index) {
                            polylines[index / 2][1]
                        } else {
                            polylines[index / 2][polylines[index / 2].len() - 2]
                        };
                        heading = Point2D::change_in_heading(p3, p2, p4);
                        if !heading.is_nan() {
                            node_angles[last_node_id(i)].push(heading);
                            endnodes[last_node_id(i)].push(index);
                        }
                    }
                }
                if endnodes[last_node_id(i)].len() == 0 {
                    is_acyclic_arc[i] = true;
                }
            }
        }
    }

    // Find connecting arcs. These are arcs that don't form loops. The only way to
    // travel from one endnode to the other is to travel through the polyline. They
    // can be safely removed from the graph.
    let mut source_node: usize;
    let mut target_node: usize;
    for i in 0..polylines.len() {
        if !is_acyclic_arc[i] {
            source_node = last_node_id(i); // start with the end node of the polyline
            target_node = first_node_id(i); // end with the start node of the polyline

            let mut prev = vec![num_endnodes; num_endnodes];

            // set the source node's prev value to anything other than num_endnodes
            prev[source_node] = num_endnodes + 1;

            // initialize the queue
            let mut queue = BinaryHeap::with_capacity(num_endnodes);
            for a in &endnodes[source_node] {
                prev[*a] = source_node;
                queue.push(Link {
                    id: *a,
                    priority: 0f64,
                });
            }

            let mut target_found = false;
            while !queue.is_empty() && !target_found {
                let link = queue.pop().unwrap();
                if link.id == target_node {
                    // This happens for a single-line polygon.
                    target_found = true;
                    break;
                }
                let other_side = get_other_endnode(link.id);
                prev[other_side] = link.id;
                for a in &endnodes[other_side] {
                    if prev[*a] == num_endnodes {
                        prev[*a] = other_side;
                        if *a == target_node {
                            target_found = true;
                            break;
                        }
                        if !is_acyclic_arc[*a / 2] {
                            queue.push(Link {
                                id: *a,
                                priority: link.priority + lengths[link.id / 2],
                            });
                        }
                    }
                }
            }

            if !target_found {
                is_acyclic_arc[i] = true;
            }
        }
    }

    let mut node1: usize;
    let mut node2: usize;
    for i in 0..polylines.len() {
        node1 = first_node_id(i);
        for n in (0..endnodes[node1].len()).rev() {
            node2 = endnodes[node1][n];
            if is_acyclic_arc[node2 / 2] {
                endnodes[node1].remove(n);
                node_angles[node1].remove(n);
            }
        }
        node1 = last_node_id(i);
        for n in (0..endnodes[node1].len()).rev() {
            node2 = endnodes[node1][n];
            if is_acyclic_arc[node2 / 2] {
                endnodes[node1].remove(n);
                node_angles[node1].remove(n);
            }
        }
    }

    /////////////////////////////////////////////////////////////////////////////////////////
    // This is the main part of the analysis. It is responsible for building the polygons. //
    /////////////////////////////////////////////////////////////////////////////////////////
    let mut bb: Vec<BoundingBox> = vec![];
    let mut current_node: usize;
    let mut neighbour_node: usize;
    let mut num_neighbours: usize;
    let mut existing_polygons = HashSet::new();
    let mut existing_hull = HashSet::new();
    let mut feature_geometries: Vec<MultiPolyline> = vec![];
    // let mut overlay_poly_id: Vec<usize> = vec![];
    let mut hull_geometries: Vec<Polyline> = vec![];
    let mut fid = 1;
    let mut p: Point2D;
    let mut max_val: f64;
    let mut max_val_index: usize;
    let mut k: usize;
    let mut num_vertices: usize;
    let mut other_side: usize;
    let mut target_found: bool;
    let mut assigned = vec![0usize; polylines.len()];
    let mut is_clockwise: bool;
    // let mut overlaps_with_other: bool;
    let mut overlaps_with_poly: bool;
    // let mut poly_is_hole: bool;
    // let mut other_is_hole: bool;
    let mut last_index: usize;

    for i in 0..polylines.len() {
        let mut mp = MultiPolyline::new(i + 1);
        mp.push(&polylines[i]);
        feature_geometries.push(mp);
    }

    return feature_geometries;

    // println!("12962 {:?}", endnodes[12962]);
    // println!("12963 {:?}", endnodes[12963]);
    // println!("length: {}", polylines[6481].length());

    for i in 0..polylines.len() {
        // if i == 3602 {
        //     println!("I'm here at {}", i);
        // }
        if !is_acyclic_arc[i] && assigned[i] < 2 {
            // if i == 3602 {
            //     println!("I'm here at {}", i);
            // }
            // starting at the last vertex, traverse a chain of lines, always
            // taking the rightmost line at each junction. Stop when you encounter
            // the first vertex of the line.

            source_node = last_node_id(i); // start with the end node of the polyline
            target_node = first_node_id(i); // end with the start node of the polyline

            let mut prev = vec![num_endnodes; num_endnodes];

            target_found = false;

            current_node = source_node;
            loop {
                num_neighbours = endnodes[current_node].len();
                if num_neighbours > 1 {
                    // We're at a junction and we should take the
                    // rightmost line.
                    max_val = node_angles[current_node][0];
                    max_val_index = 0;
                    for a in 1..num_neighbours {
                        if node_angles[current_node][a] > max_val {
                            max_val = node_angles[current_node][a];
                            max_val_index = a;
                        }
                        if endnodes[current_node][a] == target_node {
                            neighbour_node = endnodes[current_node][a];
                            prev[neighbour_node] = current_node;
                            break;
                        }
                    }
                    neighbour_node = endnodes[current_node][max_val_index];
                    other_side = get_other_endnode(neighbour_node);
                    if prev[other_side] != num_endnodes && other_side != target_node {
                        break;
                    }
                    prev[neighbour_node] = current_node;
                    prev[other_side] = neighbour_node;
                    if neighbour_node == target_node || other_side == target_node {
                        target_found = true;
                        break;
                    }
                    current_node = other_side;
                } else if num_neighbours == 1 {
                    // There's only one way forward, so take it.
                    neighbour_node = endnodes[current_node][0];
                    other_side = get_other_endnode(neighbour_node);
                    // if i == 3602 {
                    //     println!(
                    //         "neighbour_node {} ({}) other_side {} ({})",
                    //         neighbour_node,
                    //         neighbour_node / 2,
                    //         other_side,
                    //         other_side / 2
                    //     );
                    // }
                    if prev[other_side] != num_endnodes && other_side != target_node {
                        break;
                    }
                    prev[neighbour_node] = current_node;
                    prev[other_side] = neighbour_node;
                    if neighbour_node == target_node || other_side == target_node {
                        target_found = true;
                        break;
                    }
                    current_node = other_side;
                } else {
                    // because we've removed links to dangling arcs, this should never occur
                    break;
                }
            }

            if target_found {
                // if i == 3602 {
                //     println!("target found at {}", i);
                // }
                // traverse from the target to the source
                let mut lines: Vec<usize> = vec![];
                let mut backlinks: Vec<usize> = vec![];
                k = target_node;
                num_vertices = 0;
                while k != source_node {
                    k = prev[k];
                    backlinks.push(k);
                    let pl = k / 2;
                    if !is_first_node(k) {
                        // don't add polylines twice. Add at the ending node.
                        lines.push(pl);
                        num_vertices += polylines[pl].len() - 1;
                    }
                }
                backlinks.push(target_node);

                // join the lines
                lines.reverse();
                backlinks.reverse();
                let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
                for a in 0..lines.len() {
                    let pl = lines[a];
                    let mut v = (polylines[pl].vertices).clone();
                    if backlinks[a * 2] > backlinks[a * 2 + 1] {
                        v.reverse();
                    }
                    if a < lines.len() - 1 {
                        v.pop();
                    }
                    vertices.append(&mut v);
                }

                // Is it clockwise order?
                is_clockwise = is_clockwise_order(&vertices);

                // don't add the same poly more than once
                let mut test_poly = lines.clone();
                test_poly.sort();
                if !existing_polygons.contains(&test_poly) {
                    if is_clockwise {
                        existing_polygons.insert(test_poly);
                        for a in 0..lines.len() {
                            assigned[lines[a]] += 1;
                        }
                        if vertices.len() > 3 {
                            // a minimum of four points are needed to form a closed polygon (triangle)
                            if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                                p = vertices[0];
                                if vertices[0].distance(&vertices[vertices.len() - 1]) < precision {
                                    last_index = vertices.len() - 1;
                                    vertices[last_index] = p;
                                } else {
                                    vertices.push(p);
                                }
                            }
                            p = interior_point(&vertices);
                            overlaps_with_poly = false;
                            for j in 0..polygons2.len() {
                                if point_in_poly(&p, &(polygons2[j].vertices)) {
                                    overlaps_with_poly = true;
                                    break;
                                    // if polygons[j].source_file != feature_source_file {
                                    //     other_poly_id = polygons[j].id;
                                    //     if !is_part_a_hole2[j] && !other_is_hole {
                                    //         overlaps_with_other = true;
                                    //     } else {
                                    //         overlaps_with_other = false;
                                    //         other_is_hole = true;
                                    //     }
                                    // } else {
                                    //     if !is_part_a_hole2[j] && !poly_is_hole {
                                    //         overlaps_with_poly = true;
                                    //     } else {
                                    //         overlaps_with_poly = false;
                                    //         poly_is_hole = true;
                                    //     }
                                    // }
                                }
                            }

                            if overlaps_with_poly {
                                // output the polygon
                                let sfg = Polyline::new(&vertices, fid);
                                fid += 1;
                                bb.push(sfg.get_bounding_box());
                                let mut mp = MultiPolyline::new(fid);
                                mp.push(&sfg);
                                feature_geometries.push(mp);
                            // if i == 3602 {
                            //     println!("Polygon found at {}", i);
                            // }
                            // if i == 3600 {
                            //     println!("Polygon found at {} {:?}", i, lines.clone());
                            // }
                            } else {
                                let mut test_poly = lines.clone();
                                test_poly.sort();
                                if !existing_hull.contains(&test_poly) {
                                    existing_hull.insert(test_poly);
                                    for a in 0..lines.len() {
                                        assigned[lines[a]] += 1;
                                    }
                                    hull_geometries.push(Polyline::new(&vertices, 0));
                                    // if i == 3602 {
                                    //     println!("Possible hull found at {}", i);
                                    // }
                                }
                            }
                        }
                    } else {
                        // This could be a hull.
                        test_poly = lines.clone();
                        test_poly.sort();
                        if !existing_hull.contains(&test_poly) {
                            existing_hull.insert(test_poly);
                            for a in 0..lines.len() {
                                assigned[lines[a]] += 1;
                            }
                            if vertices.len() > 3 {
                                // a minimum of four points are needed to form a closed polygon (triangle)
                                if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                                    p = vertices[0];
                                    if vertices[0].distance(&vertices[vertices.len() - 1])
                                        < precision
                                    {
                                        last_index = vertices.len() - 1;
                                        vertices[last_index] = p;
                                    } else {
                                        vertices.push(p);
                                    }
                                }
                                p = interior_point(&vertices);
                                overlaps_with_poly = false;
                                for j in 0..polygons2.len() {
                                    if point_in_poly(&p, &(polygons2[j].vertices)) {
                                        overlaps_with_poly = true;
                                        break;
                                    }
                                }
                                if !overlaps_with_poly {
                                    hull_geometries.push(Polyline::new(&vertices, 0));
                                    // if i == 3602 {
                                    //     println!("ccw hull found at {}", i);
                                    // }
                                }
                            }
                        }
                    }
                }
            }

            if assigned[i] < 2 {
                ///////////////////////////////////////
                // now check for a left-side polygon //
                ///////////////////////////////////////
                source_node = first_node_id(i); // start with the first node of the polyline
                target_node = last_node_id(i); // end with the last node of the polyline

                let mut prev = vec![num_endnodes; num_endnodes];

                target_found = false;

                current_node = source_node;
                loop {
                    num_neighbours = endnodes[current_node].len();
                    if num_neighbours > 1 {
                        // // We're at a junction and we should take the
                        // // rightmost line.
                        // max_val = node_angles[current_node][0];
                        // max_val_index = 0;
                        // for a in 1..num_neighbours {
                        //     if node_angles[current_node][a] > max_val {
                        //         max_val = node_angles[current_node][a];
                        //         max_val_index = a;
                        //     }
                        //     if endnodes[current_node][a] == target_node {
                        //         neighbour_node = endnodes[current_node][a];
                        //         prev[neighbour_node] = current_node;
                        //         break;
                        //     }
                        // }
                        // neighbour_node = endnodes[current_node][max_val_index];

                        // We're at a junction and we should take the
                        // leftmost line.
                        let mut min_val = node_angles[current_node][0];
                        let mut min_val_index = 0;
                        for a in 1..num_neighbours {
                            if node_angles[current_node][a] < min_val {
                                min_val = node_angles[current_node][a];
                                min_val_index = a;
                            }
                            if endnodes[current_node][a] == target_node {
                                neighbour_node = endnodes[current_node][a];
                                prev[neighbour_node] = current_node;
                                break;
                            }
                        }
                        neighbour_node = endnodes[current_node][min_val_index];
                        other_side = get_other_endnode(neighbour_node);
                        if prev[other_side] != num_endnodes && other_side != target_node {
                            break;
                        }
                        prev[neighbour_node] = current_node;
                        prev[other_side] = neighbour_node;
                        if neighbour_node == target_node || other_side == target_node {
                            target_found = true;
                            break;
                        }
                        current_node = other_side;
                    } else if num_neighbours == 1 {
                        // There's only one way forward, so take it.
                        neighbour_node = endnodes[current_node][0];
                        other_side = get_other_endnode(neighbour_node);
                        if prev[other_side] != num_endnodes && other_side != target_node {
                            break;
                        }
                        prev[neighbour_node] = current_node;
                        prev[other_side] = neighbour_node;
                        if neighbour_node == target_node || other_side == target_node {
                            target_found = true;
                            break;
                        }
                        current_node = other_side;
                    } else {
                        // because we've removed links to danling arcs, this should never occur
                        break;
                    }
                }

                if target_found {
                    // if i == 3602 {
                    //     println!("second target found at {}", i);
                    // }
                    // traverse from the target to the source
                    let mut lines: Vec<usize> = vec![];
                    let mut backlinks: Vec<usize> = vec![];
                    k = target_node;
                    num_vertices = 0;
                    while k != source_node {
                        k = prev[k];
                        backlinks.push(k);
                        let pl = k / 2;
                        if is_first_node(k) {
                            // don't add polylines twice. Add at the first node.
                            lines.push(pl);
                            num_vertices += polylines[pl].len() - 1;
                        }
                    }
                    backlinks.push(target_node);

                    // join the lines and then output the polygon
                    lines.reverse();
                    backlinks.reverse();
                    let mut vertices: Vec<Point2D> = Vec::with_capacity(num_vertices);
                    for a in 0..lines.len() {
                        let pl = lines[a];
                        let mut v = (polylines[pl].vertices).clone();
                        if backlinks[a * 2] > backlinks[a * 2 + 1] {
                            v.reverse();
                        }
                        if a < lines.len() - 1 {
                            v.pop();
                        }
                        vertices.append(&mut v);
                    }

                    // if i == 3602 {
                    //     println!("Lines at {} {:?}", i, lines.clone());
                    // }

                    // Is it clockwise order?
                    is_clockwise = is_clockwise_order(&vertices);

                    // don't add the same poly more than once
                    let mut test_poly = lines.clone();
                    test_poly.sort();
                    if !existing_polygons.contains(&test_poly) {
                        if is_clockwise {
                            existing_polygons.insert(test_poly);
                            for a in 0..lines.len() {
                                assigned[lines[a]] += 1;
                            }
                            if vertices.len() > 3 {
                                // a minimum of four points are needed to form a closed polygon (triangle)
                                if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                                    p = vertices[0];
                                    if vertices[0].distance(&vertices[vertices.len() - 1])
                                        < precision
                                    {
                                        last_index = vertices.len() - 1;
                                        vertices[last_index] = p;
                                    } else {
                                        vertices.push(p);
                                    }
                                }
                                p = interior_point(&vertices);
                                // overlaps_with_other = false;
                                overlaps_with_poly = false;
                                // poly_is_hole = false;
                                // other_is_hole = false;
                                // let mut other_poly_id = 0;
                                for j in 0..polygons2.len() {
                                    if point_in_poly(&p, &(polygons2[j].vertices)) {
                                        overlaps_with_poly = true;
                                        break;
                                        // if polygons[j].source_file != feature_source_file {
                                        //     other_poly_id = polygons[j].id;
                                        //     if !is_part_a_hole2[j] && !other_is_hole {
                                        //         overlaps_with_other = true;
                                        //     } else {
                                        //         overlaps_with_other = false;
                                        //         other_is_hole = true;
                                        //     }
                                        // } else {
                                        //     if !is_part_a_hole2[j] && !poly_is_hole {
                                        //         overlaps_with_poly = true;
                                        //     } else {
                                        //         overlaps_with_poly = false;
                                        //         poly_is_hole = true;
                                        //     }
                                        // }
                                    }
                                }
                                if overlaps_with_poly {
                                    // output the polygon
                                    let sfg = Polyline::new(&vertices, fid);
                                    fid += 1;
                                    bb.push(sfg.get_bounding_box());
                                    let mut mp = MultiPolyline::new(fid);
                                    mp.push(&sfg);
                                    feature_geometries.push(mp);
                                // if i == 3602 {
                                //     println!("second polygon found at {}", i);
                                // }
                                } else {
                                    let mut test_poly = lines.clone();
                                    test_poly.sort();
                                    if !existing_hull.contains(&test_poly) {
                                        existing_hull.insert(test_poly);
                                        for a in 0..lines.len() {
                                            assigned[lines[a]] += 1;
                                        }
                                        hull_geometries.push(Polyline::new(&vertices, 0));
                                        // if i == 3602 {
                                        //     println!("second hull found at {}", i);
                                        // }
                                    }
                                }
                            }
                        } else {
                            // This could be a hull.
                            test_poly = lines.clone();
                            test_poly.sort();
                            if !existing_hull.contains(&test_poly) {
                                for a in 0..lines.len() {
                                    assigned[lines[a]] += 1;
                                }
                                existing_hull.insert(test_poly);
                                if vertices.len() > 3 {
                                    // a minimum of four points are needed to form a closed polygon (triangle)
                                    if !vertices[0].nearly_equals(&vertices[vertices.len() - 1]) {
                                        p = vertices[0];
                                        if vertices[0].distance(&vertices[vertices.len() - 1])
                                            < precision
                                        {
                                            last_index = vertices.len() - 1;
                                            vertices[last_index] = p;
                                        } else {
                                            vertices.push(p);
                                        }
                                    }
                                    p = interior_point(&vertices);
                                    // overlaps_with_other = false;
                                    overlaps_with_poly = false;
                                    // poly_is_hole = false;
                                    // other_is_hole = false;
                                    // let mut other_poly_id = 0;
                                    for j in 0..polygons2.len() {
                                        if point_in_poly(&p, &(polygons2[j].vertices)) {
                                            overlaps_with_poly = true;
                                            break;
                                            // if polygons[j].source_file != feature_source_file {
                                            //     // other_poly_id = polygons[j].id;
                                            //     if !is_part_a_hole2[j] && !other_is_hole {
                                            //         overlaps_with_other = true;
                                            //     } else {
                                            //         overlaps_with_other = false;
                                            //         other_is_hole = true;
                                            //     }
                                            // } else {
                                            //     if !is_part_a_hole2[j] && !poly_is_hole {
                                            //         overlaps_with_poly = true;
                                            //     } else {
                                            //         overlaps_with_poly = false;
                                            //         poly_is_hole = true;
                                            //     }
                                            // }
                                        }
                                    }
                                    if !overlaps_with_poly {
                                        hull_geometries.push(Polyline::new(&vertices, 0));
                                        // if i == 3602 {
                                        //     println!("second ccw hull found at {}", i);
                                        // }
                                        // } else {
                                        //     let mut test_poly = lines.clone();
                                        //     test_poly.sort();
                                        //     existing_polygons.insert(test_poly);
                                        //     for a in 0..lines.len() {
                                        //         assigned[lines[a]] += 1;
                                        //     }
                                        //     // output the polygon
                                        //     let mut sfg = Polyline::new(&vertices, fid);
                                        //     fid += 1;
                                        //     bb.push(sfg.get_bounding_box());
                                        //     let mut mp = MultiPolyline::new(fid);
                                        //     mp.push(&sfg);
                                        //     feature_geometries.push(mp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // can any of the hulls be added as holes in other polygons?
    for a in 0..hull_geometries.len() {
        let hull_bb = hull_geometries[a].get_bounding_box();
        for b in 0..bb.len() {
            if hull_bb.entirely_contained_within(bb[b]) {
                if poly_in_poly(
                    &(hull_geometries[a].vertices),
                    &(feature_geometries[b][0].vertices),
                ) {
                    if is_clockwise_order(&hull_geometries[a].vertices) {
                        hull_geometries[a].vertices.reverse();
                    }
                    feature_geometries[b].push(&hull_geometries[a]);
                }
            }
        }
    }

    feature_geometries
}
