/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 03/10/2018
Last Modified: 04/10/2018
License: MIT
*/

use algorithms::triangulate;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::BoundingBox;
use structures::Point2D;
use time;
use tools::*;
use vector::*;

/// # Description
/// This tool creates a vector Voronoi diagram for a set of vector points. The
/// Voronoi diagram is the dual graph of the Delaunay triangulation. The tool
/// operates by first constructing the Delaunay triangulation and then
/// connecting the circumcenters of each triangle. Each Voronoi cell contains
/// one point of the input vector points. All locations within the cell are
/// nearer to the contained point than any other input point.
///
/// A frame of 'ghost' points are inserted around the input point set to limit
/// the spatial extent of the diagram. The frame is set back from the bounding
/// box of the input points by 2 x the average spacing. The polygons of these
/// ghost points are not output, however, points that are situated along the
/// edges of the data will have rounded (paraboloic) outer sides as a result.
///
/// This tool works on vector input data only. If a Voronoi diagram is needed
/// for a set of raster points, use the `EuclideanAllocation` tool instead.
///
/// # See Also
/// `ConstructVectorTIN`, `EuclideanAllocation`
pub struct VoronoiDiagram {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl VoronoiDiagram {
    pub fn new() -> VoronoiDiagram {
        // public constructor
        let name = "VoronoiDiagram".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a vector Voronoi diagram for a set of vector points.".to_string();

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
            name: "Output Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Polygon,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp -o=tin.shp",
            short_exe, name
        ).replace("*", &sep);

        VoronoiDiagram {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for VoronoiDiagram {
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
        let mut output_file: String = "".to_string();

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

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = time::now();

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

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        output.projection = input.projection.clone();

        output
            .attributes
            .add_fields(&input.attributes.get_fields().clone());

        // Read the points in
        let mut points: Vec<Point2D> = vec![];

        for record_num in 0..input.num_records {
            let record = input.get_record(record_num);
            for i in 0..record.num_points as usize {
                points.push(Point2D::new(record.points[i].x, record.points[i].y));
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

        // Add a frame of ghost points surrounding the data, to serve as an artificial convex hull.
        let mut ghost_box = BoundingBox::new(
            input.header.x_min,
            input.header.x_max,
            input.header.y_min,
            input.header.y_max,
        );
        // expand the box by a factor of the average point spacing.
        let expansion = ((input.header.x_max - input.header.x_min)
            * (input.header.y_max - input.header.y_min)
            / input.num_records as f64)
            .sqrt()
            * 2.0;
        ghost_box.expand_by(expansion);

        let mut gap = (ghost_box.max_y - ghost_box.min_y) / 100f64;
        for y in 0..100 {
            points.push(Point2D::new(
                ghost_box.min_x,
                ghost_box.min_y + y as f64 * gap,
            ));
            points.push(Point2D::new(
                ghost_box.max_x,
                ghost_box.min_y + y as f64 * gap,
            ));
        }
        gap = (ghost_box.max_x - ghost_box.min_x) / 100f64;
        for x in 0..100 {
            points.push(Point2D::new(
                ghost_box.min_x + x as f64 * gap,
                ghost_box.min_y,
            ));
            points.push(Point2D::new(
                ghost_box.min_x + x as f64 * gap,
                ghost_box.max_y,
            ));
        }

        // Do the Delaunay triangulation
        if verbose {
            println!("Performing triangulation...");
        }
        // this is where the heavy-lifting is
        let delaunay = triangulate(&points).expect("No triangulation exists.");

        if verbose {
            println!("Creating point-halfedge mapping...");
        }
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

        // Now create the Voronoi cells
        const EMPTY: usize = usize::max_value();
        for p in 0..input.num_records {
            // get the edge that is incoming to 'p'
            let edge = match point_edge_map.get(&p) {
                Some(e) => *e,
                None => EMPTY,
            };
            if edge != EMPTY {
                let edges = delaunay.edges_around_point(edge);
                let triangles: Vec<usize> = edges
                    .into_iter()
                    .map(|e| delaunay.triangle_of_edge(e))
                    .collect();

                let vertices: Vec<Point2D> = triangles
                    .into_iter()
                    .map(|t| delaunay.triangle_center(&points, t))
                    .collect();

                if vertices[0] == vertices[vertices.len() - 1] {
                    // It's a closed polygon. Notice that in order to
                    // enable a duplication of the first and last point,
                    // delaunay.edges_around_point has been modified:
                    // } else if incoming == start {
                    //       result.push(incoming);
                    let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                    sfg.add_part(&vertices);
                    output.add_record(sfg);

                    // now get the attributes of the parent point.
                    output
                        .attributes
                        .add_record(input.attributes.get_record(p).clone(), false);
                }
            }

            if verbose {
                progress = (100.0_f64 * p as f64 / (input.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

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
                &format!("Elapsed Time: {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
