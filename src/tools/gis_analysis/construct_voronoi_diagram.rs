/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/09/2018
Last Modified: 21/09/2018
License: MIT
*/

use algorithms::triangulate;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use structures::Point2D;
use time;
use tools::*;
// use vector::ShapefileGeometry;
use vector::*;

/// This tool creates a vector Voronoi diagram for a set of vector points.
pub struct ConstructVoronoiDiagram {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ConstructVoronoiDiagram {
    pub fn new() -> ConstructVoronoiDiagram {
        // public constructor
        let name = "ConstructVoronoiDiagram".to_string();
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

        ConstructVoronoiDiagram {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ConstructVoronoiDiagram {
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
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;

        // set the projection information
        output.projection = input.projection.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 5u8, 0u8));

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

        if verbose {
            println!("Performing triangulation...");
        }
        // this is where the heavy-lifting is
        let delaunay = triangulate(&points).expect("No triangulation exists.");

        const EMPTY: usize = usize::max_value();
        let mut rec_num = 1i32;
        let mut halfedge: usize;
        // let mut cc1: Point2D;
        // let mut cc2: Point2D;

        for t in 0..delaunay.len() {
            let v = delaunay.voronoi_cell(&points, t * 3);
            let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
            sfg.add_part(&v);
            output.add_record(sfg);

            output
                .attributes
                .add_record(vec![FieldData::Int(rec_num)], false);

            rec_num += 1i32;

            if verbose {
                progress = (100.0_f64 * t as f64 / (delaunay.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Creating polygons: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // for e in 0..delaunay.triangles.len() {
        //     halfedge = delaunay.halfedges[e];
        //     if halfedge != EMPTY {
        //         // cc1 = delaunay.triangle_center(&points, e / 3);
        //         // cc2 = delaunay.triangle_center(&points, halfedge / 3);

        //         let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);

        //         // let mut poly_points: Vec<Point2D> = Vec::with_capacity(2);
        //         // poly_points.push(cc1);
        //         // poly_points.push(cc2);

        //         let v = delaunay.voronoi_cell(&points, e);

        //         // println!("{} {:?}", e, v);

        //         sfg.add_part(&v); //poly_points);
        //         output.add_record(sfg);

        //         output
        //             .attributes
        //             .add_record(vec![FieldData::Int(rec_num)], false);

        //         rec_num += 1i32;
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * e as f64 / (delaunay.triangles.len() - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Creating polygons: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

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
