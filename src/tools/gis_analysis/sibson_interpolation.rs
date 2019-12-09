/* 

THIS WAS MY ATTEMPT AT GETTING A SIBSON'S METHOD INTERPOLATOR (NATURAL NEIGHBOUR). THERE WERE
TWO MAJOR ISSUES WITH THE CODE. FIRST, IT RELIES ON THE TRIANGULATION CODE, AND SEEMS TO GET
STUCK IN SOME DIFFICULT TO FIND INFITITE LOOP, AT LEAST ON THE DATA USED IN TESTING. SECONDLY,
THE CODE IS VERY SLOW. IN PART THIS IS BECAUSE THE TRIANGULATION CODE DOES NOT ALLOW FOR SINGLE
POINT INSERTION AND DELETION. THIS IS A MAJOR PROBLEM WITH OTHERWISE VERY PERFORMANCE OPTIMAL
CODE. AS SUCH, EACH GRID INTERSECTION REQUIRES 1) FINDING THE NATURAL NEIGHBOURS AND THEIR 
NEIGHBOURS, 2) TRIANGULATING THESE NEIGHBOURS, 3) INSERTING THE GRID INTERSECTION TO THE 
LOCAL TRIANGULATION, AND 4) COMPARING THE AREA OF THE VORONOI CELLS BEFORE AND AFTER
INSERTION. THIS APPROACH IS REALLY NOT SUITED. I'M TAKING THE TOOL OFFLINE UNTIL A BETTER
ALTERNATIVE CAN BE FOUND.

This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/10/2018
Last Modified: 05/10/2018
License: MIT
*/

/*
use algorithms::{point_in_poly, polygon_area, triangulate};
use raster::*;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::{BoundingBox, Point2D};
use tools::*;
use vector::*;

/// Creates a raster grid based on Sibson's interpolation method, sometimes called *natural neighbours*.
/// Sibson's method involves applying weights to each of the nearby points, determined by the Voronoi
/// diagram, of each grid intersection. Weights are determined by the captured area by the Voronoi cell
/// that is created when the grid intersection is inserted into the point set.
///
/// # See Also
/// 'VoronoiDiagram`, LidarTINGridding, ConstructVectorTIN
pub struct SibsonInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SibsonInterpolation {
    pub fn new() -> SibsonInterpolation {
        // public constructor
        let name = "SibsonInterpolation".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a raster grid based on a triangular irregular network (TIN) fitted to vector points."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::RasterAndVector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Input field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use Shapefile 'z' values?".to_owned(),
            flags: vec!["--use_z".to_owned()],
            description:
                "Use the 'z' dimension of the Shapefile's geometry instead of an attribute field?"
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Raster File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=HEIGHT -o=tin.shp --resolution=10.0
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --use_z -o=tin.shp --resolution=5.0",
            short_exe, name
        ).replace("*", &sep);

        SibsonInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SibsonInterpolation {
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
        let mut input_file: String = "".to_string();
        let mut field_name = String::new();
        let mut use_z = false;
        let mut use_field = false;
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;

        // read the arguments
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
            } else if flag_val == "-field" {
                field_name = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                use_field = true;
            } else if flag_val.contains("use_z") {
                use_z = true;
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
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

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point
            && input.header.shape_type.base_shape_type() != ShapeType::MultiPoint
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of POINT base shape type.",
            ));
        }

        if !use_z && !use_field {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "If vector data 'Z' data are unavailable (--use_z), an attribute field must be specified (--field=).",
            ));
        }

        if use_z && input.header.shape_type.dimension() != ShapeTypeDimension::Z {
            return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input vector data must be of 'POINTZ' or 'MULTIPOINTZ' ShapeType to use the --use_z flag.",
                ));
        } else if use_field {
            // What is the index of the field to be analyzed?
            let field_index = match input.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The specified field name does not exist in input shapefile.",
                    ))
                }
            };

            // Is the field numeric?
            if !input.attributes.is_field_numeric(field_index) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified attribute field is non-numeric.",
                ));
            }
        }

        let west: f64 = input.header.x_min;
        let north: f64 = input.header.y_max;
        let rows: isize = (((north - input.header.y_min) / grid_res).ceil()) as isize;
        let columns: isize = (((input.header.x_max - west) / grid_res).ceil()) as isize;
        let south: f64 = north - rows as f64 * grid_res;
        let east = west + columns as f64 * grid_res;
        let nodata = -32768.0f64;

        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = grid_res;
        configs.resolution_y = grid_res;
        configs.nodata = nodata;
        configs.data_type = DataType::F32;
        configs.photometric_interp = PhotometricInterpretation::Continuous;

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        let mut points: Vec<Point2D> = vec![];
        let mut z_values: Vec<f64> = vec![];

        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
                if use_z {
                    z_values.push(record.z_array[i]);
                } else if use_field {
                    match input.attributes.get_value(record_num, &field_name) {
                        FieldData::Int(val) => {
                            z_values.push(val as f64);
                        }
                        FieldData::Real(val) => {
                            z_values.push(val);
                        }
                        _ => {
                            // likely a null field
                            z_values.push(0f64);
                        }
                    }
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Reading points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // // Add a frame of hidden points surrounding the data, to serve as an artificial hull.
        // let mut ghost_box = BoundingBox::new(
        //     input.header.x_min,
        //     input.header.x_max,
        //     input.header.y_min,
        //     input.header.y_max,
        // );
        // expand the box by a factor of the average point spacing.
        let expansion = ((input.header.x_max - input.header.x_min)
            * (input.header.y_max - input.header.y_min)
            / input.num_records as f64)
            .sqrt();
        // ghost_box.expand_by(2.0 * expansion);

        let gap = expansion / 3f64; // One-third the average point spacing

        // let mut num_edge_points = ((ghost_box.max_x - ghost_box.min_x) / gap) as usize;
        // for x in 0..num_edge_points {
        //     points.push(Point2D::new(
        //         ghost_box.min_x + x as f64 * gap,
        //         ghost_box.min_y,
        //     ));
        //     points.push(Point2D::new(
        //         ghost_box.min_x + x as f64 * gap,
        //         ghost_box.max_y,
        //     ));
        // }

        // num_edge_points = ((ghost_box.max_y - ghost_box.min_y) / gap) as usize;
        // for y in 0..num_edge_points {
        //     points.push(Point2D::new(
        //         ghost_box.min_x,
        //         ghost_box.min_y + y as f64 * gap,
        //     ));
        //     points.push(Point2D::new(
        //         ghost_box.max_x,
        //         ghost_box.min_y + y as f64 * gap,
        //     ));
        // }

        // this is where the heavy-lifting is
        if verbose {
            println!("Performing triangulation...");
        }
        let delaunay = triangulate(&points).expect("No triangulation exists.");
        let num_triangles = delaunay.len();

        // if verbose {
        //     println!("Creating point-halfedge mapping...");
        // }
        // let mut point_edge_map = HashMap::new(); // point id to half-edge id
        // for edge in 0..delaunay.triangles.len() {
        //     let endpoint = delaunay.triangles[delaunay.next_halfedge(edge)];
        //     if !point_edge_map.contains_key(&endpoint) || delaunay.halfedges[edge] == EMPTY {
        //         point_edge_map.insert(endpoint, edge);
        //     }
        //     if verbose {
        //         progress =
        //             (100.0_f64 * edge as f64 / (delaunay.triangles.len() - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let (mut p1, mut p2, mut p3): (usize, usize, usize);
        let (mut top, mut bottom, mut left, mut right): (f64, f64, f64, f64);
        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let mut tri_points: Vec<Point2D> = vec![Point2D::new(0f64, 0f64); 4];
        let (mut x, mut y): (f64, f64);
        let mut z: f64;
        let mut i: usize;
        let mut bb: BoundingBox;
        let mut edges: Vec<usize>;
        let mut edges2: Vec<usize>;
        let mut num_edge_points: usize;
        let mut point_edge_map: HashMap<usize, usize>;
        let mut point_edge_map_prime: HashMap<usize, usize>;
        const EMPTY: usize = usize::max_value();
        for triangle in 0..num_triangles {
            i = triangle * 3;

            p1 = delaunay.triangles[i];
            p2 = delaunay.triangles[i + 1];
            p3 = delaunay.triangles[i + 2];

            tri_points[0] = points[p1].clone();
            tri_points[1] = points[p2].clone();
            tri_points[2] = points[p3].clone();
            tri_points[3] = points[p1].clone();

            // first, find all the edges connected to the three points of the triangle
            edges = delaunay.edges_around_point(i);
            edges2 = delaunay.edges_around_point(i + 1);
            edges.append(&mut edges2);
            edges2 = delaunay.edges_around_point(i + 2);
            edges.append(&mut edges2);

            let mut pnt_nums: Vec<usize> =
                edges.into_iter().map(|e| delaunay.triangles[e]).collect();

            // there will be duplicated points, so dedup the array
            pnt_nums.sort();
            pnt_nums.dedup();
            if pnt_nums[pnt_nums.len() - 1] == EMPTY {
                pnt_nums.pop();
            }

            // get the Point2D data and z value for each point
            let n_pnts = pnt_nums.len();
            let mut local_points: Vec<Point2D> = Vec::with_capacity(n_pnts);
            for j in 0..n_pnts {
                local_points.push(points[pnt_nums[j]].clone());
            }

            // add a hidden edge of points
            bb = BoundingBox::from_points(&local_points);
            bb.expand_by(2.0 * expansion);
            num_edge_points = ((bb.max_x - bb.min_x) / gap) as usize;
            for x in 1..=num_edge_points {
                local_points.push(Point2D::new(bb.min_x + x as f64 * gap, bb.min_y));
                local_points.push(Point2D::new(bb.min_x + x as f64 * gap, bb.max_y));
            }
            num_edge_points = ((bb.max_y - bb.min_y) / gap) as usize;
            for y in 0..num_edge_points {
                local_points.push(Point2D::new(bb.min_x, bb.min_y + y as f64 * gap));
                local_points.push(Point2D::new(bb.max_x, bb.min_y + y as f64 * gap));
            }

            // if triangle == 410708 {
            //     for a in pnt_nums {
            //         println!("{}", a);
            //     }
            //     for a in 0..local_points.len() {
            //         println!("{},{}", local_points[a].x, local_points[a].y);
            //     }
            // }
            // create a local triangulation
            let dt = triangulate(&local_points).expect("No triangulation exists.");

            // get the area of each of the Voronoi cells
            let mut poly_area = vec![0f64; n_pnts];
            point_edge_map = HashMap::new(); // point id to half-edge id
            for edge in 0..dt.triangles.len() {
                let endpoint = dt.triangles[dt.next_halfedge(edge)];
                if !point_edge_map.contains_key(&endpoint) || dt.halfedges[edge] == EMPTY {
                    point_edge_map.insert(endpoint, edge);
                }
            }
            for p in 0..n_pnts {
                // get the edge that is incoming to 'p'
                let edge = match point_edge_map.get(&p) {
                    Some(e) => *e,
                    None => EMPTY,
                };
                if edge != EMPTY {
                    let edges = dt.edges_around_point(edge);
                    let triangles: Vec<usize> =
                        edges.into_iter().map(|e| dt.triangle_of_edge(e)).collect();

                    let vertices: Vec<Point2D> = triangles
                        .into_iter()
                        .map(|t| dt.triangle_center(&local_points, t))
                        .collect();

                    if vertices[0] == vertices[vertices.len() - 1] {
                        poly_area[p] = polygon_area(&vertices);
                    }
                }
            }

            // find grid intersections with this triangle
            bottom = points[p1].y.min(points[p2].y.min(points[p3].y));
            top = points[p1].y.max(points[p2].y.max(points[p3].y));
            left = points[p1].x.min(points[p2].x.min(points[p3].x));
            right = points[p1].x.max(points[p2].x.max(points[p3].x));

            bottom_row = ((north - bottom) / grid_res).ceil() as isize;
            top_row = ((north - top) / grid_res).floor() as isize;
            left_col = ((left - west) / grid_res).floor() as isize;
            right_col = ((right - west) / grid_res).ceil() as isize;

            for row in top_row..=bottom_row {
                for col in left_col..=right_col {
                    x = west + col as f64 * grid_res;
                    y = north - row as f64 * grid_res;
                    if point_in_poly(&Point2D::new(x, y), &tri_points) {
                        // insert (x,y) into the local points list
                        local_points.push(Point2D::new(x, y));

                        // triangulate the points
                        let dt_prime =
                            triangulate(&local_points).expect("No triangulation exists.");

                        // get the changed area of each of the Voronoi cells
                        let mut weights = vec![0f64; n_pnts];
                        let mut sum_weight = 0f64;
                        point_edge_map_prime = HashMap::new(); // point id to half-edge id
                        for edge in 0..dt_prime.triangles.len() {
                            let endpoint = dt_prime.triangles[dt_prime.next_halfedge(edge)];
                            if !point_edge_map_prime.contains_key(&endpoint)
                                || dt_prime.halfedges[edge] == EMPTY
                            {
                                point_edge_map_prime.insert(endpoint, edge);
                            }
                        }

                        for p in 0..n_pnts {
                            // get the edge that is incoming to 'p'
                            let edge = match point_edge_map_prime.get(&p) {
                                Some(e) => *e,
                                None => EMPTY,
                            };
                            if edge != EMPTY {
                                let edges = dt_prime.edges_around_point(edge);
                                let triangles: Vec<usize> = edges
                                    .into_iter()
                                    .map(|e| dt_prime.triangle_of_edge(e))
                                    .collect();

                                let vertices: Vec<Point2D> = triangles
                                    .into_iter()
                                    .map(|t| dt_prime.triangle_center(&local_points, t))
                                    .collect();

                                if vertices[0] == vertices[vertices.len() - 1] {
                                    weights[p] = poly_area[p] - polygon_area(&vertices);
                                    sum_weight += weights[p];
                                }
                            }
                        }

                        // calculate the z values
                        z = 0f64;
                        for p in 0..n_pnts {
                            z += weights[p] / sum_weight * z_values[pnt_nums[p]];
                        }

                        output.set_value(row, col, z);

                        // remove (x,y) from the local points list
                        local_points.pop();
                    }
                }
            }

            if verbose {
                progress = (100.0_f64 * triangle as f64 / (num_triangles - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
        output.add_metadata_entry(
            format!("Elapsed Time (including I/O): {}", elapsed_time)
        );

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time: {}", elapsed_time)
            );
        }

        Ok(())
    }
}
*/
