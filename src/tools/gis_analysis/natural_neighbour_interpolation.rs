/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 08/12/2019
Last Modified: 10/12/2019
License: MIT
*/

use crate::algorithms::{point_in_poly, polygon_area, triangulate, Triangulation};
use crate::raster::*;
use crate::structures::{BoundingBox, Point2D};
use crate::tools::*;
use crate::vector::{FieldData, ShapeType, ShapeTypeDimension, Shapefile};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use num_cpus;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to interpolate a set of input vector points (`--input`) onto a raster grid using
/// Sibson's (1981) natural neighbour method. Similar to inverse-distance-weight interpolation (`IdwInterpolation`),
/// the natural neighbour method performs a weighted averaging of nearby point values to estimate the attribute
/// (`--field`) value at grid cell intersections in the output raster (`--output`). However, the two methods differ
/// quite significantly in the way that neighbours are identified and in the weighting scheme. First, natural neigbhour
/// identifies neighbours to be used in the interpolation of a point by finding the points connected to the
/// estimated value location in a [Delaunay triangulation](https://en.wikipedia.org/wiki/Delaunay_triangulation), that
/// is, the so-called *natural neighbours*. This approach has the main advantage of not having to specify an arbitrary
/// search distance or minimum number of nearest neighbours like many other interpolators do. Weights in the natural
/// neighbour scheme are determined using an area-stealing approach, whereby the weight assigned to a neighbour's value
/// is determined by the proportion of its [Voronoi polygon](https://en.wikipedia.org/wiki/Voronoi_diagram) that would
/// be lost by inserting the interpolation point into the Voronoi diagram. That is, inserting the interpolation point into
/// the Voronoi diagram results in the creation of a new polygon and shrinking the sizes of the Voronoi polygons associated
/// with each of the natural neighbours. The larger the area by which a neighbours polygon is reduced through the
/// insertion, relative to the polygon of the interpolation point, the greater the weight given to the neighbour point's
/// value in the interpolation. Interpolation weights sum to one because the sum of the reduced polygon areas must
/// account for the entire area of the interpolation points polygon.
///
/// The user must specify the attribute field containing point values (`--field`). Alternatively, if the input Shapefile
/// contains z-values, the interpolation may be based on these values (`--use_z`). Either an output grid resolution
/// (`--cell_size`) must be specified or alternatively an existing base file (`--base`) can be used to determine the
/// output raster's (`--output`) resolution and spatial extent. Natural neighbour interpolation generally produces a
/// satisfactorily smooth surface within the region of data points but can produce spurious breaks in the surface
/// outside of this region. Thus, it is recommended that the output surface be clipped to the convex hull of the input
/// points (`--clip`).
///
/// # Reference
/// Sibson, R. (1981). "A brief description of natural neighbor interpolation (Chapter 2)". In V. Barnett (ed.).
/// Interpolating Multivariate Data. Chichester: John Wiley. pp. 21–36.
///
/// # See Also
/// `IdwInterpolation`, `NearestNeighbourGridding`
pub struct NaturalNeighbourInterpolation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NaturalNeighbourInterpolation {
    pub fn new() -> NaturalNeighbourInterpolation {
        // public constructor
        let name = "NaturalNeighbourInterpolation".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a raster grid based on Sibson's natural neighbour method.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
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

        parameters.push(ToolParameter{
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Base Raster File (optional)".to_owned(), 
            flags: vec!["--base".to_owned()], 
            description: "Optionally specified input base raster file. Not used when a cell size is specified.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Clip to convex hull?".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Clip the data to the convex hull of the points?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=HEIGHT -o=surface.tif --resolution=10.0 --clip",
            short_exe, name
        ).replace("*", &sep);

        NaturalNeighbourInterpolation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NaturalNeighbourInterpolation {
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
        let mut grid_res: f64 = 0.0;
        let mut base_file = String::new();
        let mut clip_to_hull = false;

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
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_z = true;
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-resolution" || flag_val == "-cell_size" {
                grid_res = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val.contains("clip") {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    clip_to_hull = true;
                }
            } else if flag_val == "-base" {
                base_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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

        let nodata = -32768.0f64;
        let mut output = if !base_file.trim().is_empty() || grid_res == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let mut base = Raster::new(&base_file, "r")?;
            base.configs.nodata = nodata;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            if grid_res == 0f64 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified grid resolution is incorrect. Either a non-zero grid resolution \nor an input existing base file name must be used.",
                ));
            }
            // base the output raster on the grid_res and the
            // extent of the input vector.
            let west: f64 = input.header.x_min;
            let north: f64 = input.header.y_max;
            let rows: isize = (((north - input.header.y_min) / grid_res).ceil()) as isize;
            let columns: isize = (((input.header.x_max - west) / grid_res).ceil()) as isize;
            let south: f64 = north - rows as f64 * grid_res;
            let east = west + columns as f64 * grid_res;

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

            Raster::initialize_using_config(&output_file, &configs)
        };

        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;
        let west = output.configs.west;
        let north = output.configs.north;
        output.configs.nodata = nodata; // in case a base image is used with a different nodata value.
        output.configs.palette = "spectrum.pal".to_string();
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        let mut points: Vec<Point2D> = Vec::with_capacity(input.get_total_num_points());
        let mut z_values: Vec<f64> = Vec::with_capacity(input.get_total_num_points());

        const DIMENSIONS: usize = 2;
        const CAPACITY_PER_NODE: usize = 64;
        let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
        let mut p = 0;
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
                tree.add([points[p].x, points[p].y], p).unwrap();
                p += 1;
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

        if verbose {
            println!("Performing triangulation...");
        }
        // this is where the heavy-lifting is
        let delaunay = triangulate(&points).expect("No triangulation exists.");

        // get the hull
        let dont_clip_to_hull = !clip_to_hull;
        let mut hull_vertices: Vec<Point2D> = vec![points[delaunay.hull[0]].clone()];
        for a in (0..delaunay.hull.len()).rev() {
            hull_vertices.push(points[delaunay.hull[a]].clone());
        }

        if verbose {
            println!("Creating point-halfedge mapping...");
        }
        const EMPTY: usize = usize::max_value();
        let mut point_edge_map = HashMap::new(); // point id to half-edge id
        for edge in 0..delaunay.triangles.len() {
            let endpoint = delaunay.triangles[delaunay.next_halfedge(edge)];
            if !point_edge_map.contains_key(&endpoint) || delaunay.halfedges[edge] == EMPTY {
                point_edge_map.insert(endpoint, edge);
            }
            if verbose {
                progress =
                    (100.0_f64 * edge as f64 / (delaunay.triangles.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let res_x = output.configs.resolution_x;
        let res_y = output.configs.resolution_y;

        if verbose {
            println!("Interpolating...");
        }
        let points = Arc::new(points);
        let z_values = Arc::new(z_values);
        let delaunay = Arc::new(delaunay);
        let tree = Arc::new(tree);
        let hull_vertices = Arc::new(hull_vertices);
        let point_edge_map = Arc::new(point_edge_map);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let points = points.clone();
            let z_values = z_values.clone();
            let delaunay = delaunay.clone();
            let tree = tree.clone();
            let hull_vertices = hull_vertices.clone();
            let point_edge_map = point_edge_map.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut px, mut py): (f64, f64);
                let mut previous_nn = EMPTY;
                let mut delaunay2: Triangulation;
                let mut natural_neighbours: Vec<usize> = vec![];
                let mut nn_points: Vec<Point2D> = vec![];
                let mut num_neighbours = 0;
                let mut areas1: Vec<f64> = vec![];
                let mut areas2: Vec<f64>;
                let mut edge: usize;
                let mut edges: Vec<usize>;
                let mut point_num: usize;
                let mut vertices: Vec<Point2D>;
                let mut triangles: Vec<usize>;
                let mut ghost_box: BoundingBox;
                let mut expansion: f64;
                let mut gap: f64;
                let mut num_edge_points: usize;
                let mut endpoint: usize;
                let mut sum_diff: f64;
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        px = west + (col as f64 + 0.5) * res_x;
                        py = north - (row as f64 + 0.5) * res_y;
                        if dont_clip_to_hull || point_in_poly(&Point2D::new(px, py), &hull_vertices)
                        {
                            // find the nearest point
                            match tree.nearest(&[px, py], 1, &squared_euclidean) {
                                Ok(ret) => {
                                    point_num = *ret[0].1;

                                    if ret[0].0 > 0f64 {
                                        // point does not coincide with a sample

                                        if point_num != previous_nn {
                                            // get the edge that is incoming to 'point_num'
                                            edge = match point_edge_map.get(&point_num) {
                                                Some(e) => *e,
                                                None => EMPTY,
                                            };
                                            if edge != EMPTY {
                                                // find all the neighbours of point_num and their neighbours too
                                                natural_neighbours =
                                                    delaunay.natural_neighbours_2nd_order(edge);
                                                num_neighbours = natural_neighbours.len();

                                                nn_points = natural_neighbours
                                                    .clone()
                                                    .into_iter()
                                                    .map(|p| points[p].clone())
                                                    .collect();

                                                /////////////////////////////////////////////
                                                // Create the Voronoi diagram of the points
                                                /////////////////////////////////////////////

                                                // Add a frame of hidden points surrounding the data, to serve as an artificial hull.
                                                ghost_box = BoundingBox::from_points(&nn_points);

                                                // expand the box by a factor of the average point spacing.
                                                expansion = ((ghost_box.max_x - ghost_box.min_x)
                                                    * (ghost_box.max_y - ghost_box.min_y)
                                                    / num_neighbours as f64)
                                                    .sqrt();
                                                ghost_box.expand_by(2.0 * expansion);

                                                gap = expansion / 2f64; // One-half the average point spacing
                                                num_edge_points =
                                                    ((ghost_box.max_x - ghost_box.min_x) / gap)
                                                        as usize;
                                                for x in 0..num_edge_points {
                                                    nn_points.push(Point2D::new(
                                                        ghost_box.min_x + x as f64 * gap,
                                                        ghost_box.min_y,
                                                    ));
                                                    nn_points.push(Point2D::new(
                                                        ghost_box.min_x + x as f64 * gap,
                                                        ghost_box.max_y,
                                                    ));
                                                }

                                                num_edge_points =
                                                    ((ghost_box.max_y - ghost_box.min_y) / gap)
                                                        as usize;
                                                for y in 0..num_edge_points {
                                                    nn_points.push(Point2D::new(
                                                        ghost_box.min_x,
                                                        ghost_box.min_y + y as f64 * gap,
                                                    ));
                                                    nn_points.push(Point2D::new(
                                                        ghost_box.max_x,
                                                        ghost_box.min_y + y as f64 * gap,
                                                    ));
                                                }

                                                delaunay2 = triangulate(&nn_points)
                                                    .expect("No triangulation exists.");

                                                // measure their areas
                                                areas1 = vec![0f64; num_neighbours];
                                                let mut point_edge_map2 = HashMap::new(); // point id to half-edge id
                                                for edge in 0..delaunay2.triangles.len() {
                                                    endpoint = delaunay2.triangles
                                                        [delaunay2.next_halfedge(edge)];
                                                    if !point_edge_map2.contains_key(&endpoint)
                                                        || delaunay2.halfedges[edge] == EMPTY
                                                    {
                                                        point_edge_map2.insert(endpoint, edge);
                                                    }
                                                }
                                                for a in 0..num_neighbours {
                                                    edge = match point_edge_map2.get(&a) {
                                                        Some(e) => *e,
                                                        None => EMPTY,
                                                    };
                                                    if edge != EMPTY {
                                                        edges = delaunay2.edges_around_point(edge);
                                                        triangles = edges
                                                            .into_iter()
                                                            .map(|e| delaunay2.triangle_of_edge(e))
                                                            .collect();

                                                        vertices = triangles
                                                            .into_iter()
                                                            .map(|t| {
                                                                delaunay2
                                                                    .triangle_center(&nn_points, t)
                                                            })
                                                            .collect();

                                                        areas1[a] = polygon_area(&vertices);
                                                    }
                                                }

                                                previous_nn = point_num;
                                            }
                                        }

                                        if areas1.len() > 0 {
                                            // now add the grid cell centre point in and re-triangulate.
                                            nn_points.pop();
                                            nn_points.push(Point2D::new(px, py));
                                            let delaunay3 = triangulate(&nn_points)
                                                .expect("No triangulation exists.");
                                            let mut point_edge_map2 = HashMap::new(); // point id to half-edge id
                                            for edge in 0..delaunay3.triangles.len() {
                                                endpoint = delaunay3.triangles
                                                    [delaunay3.next_halfedge(edge)];
                                                if !point_edge_map2.contains_key(&endpoint)
                                                    || delaunay3.halfedges[edge] == EMPTY
                                                {
                                                    point_edge_map2.insert(endpoint, edge);
                                                }
                                            }
                                            areas2 = vec![0f64; num_neighbours];
                                            for a in 0..num_neighbours {
                                                edge = match point_edge_map2.get(&a) {
                                                    Some(e) => *e,
                                                    None => EMPTY,
                                                };
                                                if edge != EMPTY {
                                                    edges = delaunay3.edges_around_point(edge);
                                                    triangles = edges
                                                        .into_iter()
                                                        .map(|e| delaunay3.triangle_of_edge(e))
                                                        .collect();

                                                    vertices = triangles
                                                        .into_iter()
                                                        .map(|t| {
                                                            delaunay3.triangle_center(&nn_points, t)
                                                        })
                                                        .collect();

                                                    areas2[a] = polygon_area(&vertices);
                                                }
                                            }

                                            sum_diff = 0f64;
                                            for a in 0..num_neighbours {
                                                sum_diff += areas1[a] - areas2[a];
                                            }
                                            if sum_diff > 0f64 {
                                                z = 0f64;
                                                for a in 0..num_neighbours {
                                                    z += (areas1[a] - areas2[a]) / sum_diff
                                                        * z_values[natural_neighbours[a]];
                                                }
                                                data[col as usize] = z;
                                            }
                                        }
                                    } else {
                                        // point coincides with a sample
                                        data[col as usize] = z_values[point_num];
                                    }
                                }
                                Err(_) => {
                                    // no point found; output nodata
                                }
                            };
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Interpolating: {}%", progress);
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
        output.add_metadata_entry(format!("Field name: {}", field_name));
        output.add_metadata_entry(format!("Use z-field: {}", use_z));
        if grid_res > 0f64 {
            output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
        } else {
            output.add_metadata_entry(format!("Base file: {}", base_file));
        }
        output.add_metadata_entry(format!("Clip to hull: {}", clip_to_hull));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

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
