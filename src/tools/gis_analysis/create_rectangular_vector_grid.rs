/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 15/09/2018
Last Modified: 20/01/2019
License: MIT
*/

use crate::lidar::*;
use crate::raster::*;
use crate::structures::{BoundingBox, Point2D};
use crate::tools::*;
use crate::vector::ShapefileGeometry;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to create a rectangular vector grid. The extent of the rectangular
/// grid is based on the extent of a user-specified base file (any supported raster format,
/// shapefiles, or LAS files). The user must also specify the origin of the grid (`--xorig` 
/// and `--yorig`) and the grid cell width and height (`--width` and `--height`).
/// 
/// # See Also
/// `CreateHexagonalVectorGrid`
pub struct CreateRectangularVectorGrid {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CreateRectangularVectorGrid {
    pub fn new() -> CreateRectangularVectorGrid {
        // public constructor
        let name = "CreateRectangularVectorGrid".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Creates a rectangular vector grid.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["-i".to_owned(), "--base".to_owned(), "--input".to_owned()],
            description: "Input base file.".to_owned(),
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

        parameters.push(ToolParameter {
            name: "Grid Cell Width".to_owned(),
            flags: vec!["--width".to_owned()],
            description: "The grid cell width.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid Cell Height".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "The grid cell height.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grid origin x-coordinate".to_owned(),
            flags: vec!["--xorig".to_owned()],
            description: "The grid origin x-coordinate.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Grid origin y-coordinate".to_owned(),
            flags: vec!["--yorig".to_owned()],
            description: "The grid origin y-coordinate.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.shp --width=10.0 --height=10.0 --xorig=0.0 --yorig=0.0",
            short_exe, name
        ).replace("*", &sep);

        CreateRectangularVectorGrid {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CreateRectangularVectorGrid {
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
        let mut width = 0f64;
        let mut height = 0f64;
        let mut xorig = 0f64;
        let mut yorig = 0f64;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-base" {
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
            } else if flag_val == "-width" {
                width = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-height" {
                height = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-xorig" {
                xorig = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-yorig" {
                yorig = if keyval {
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

        if width <= 0f64 || height <= 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: The grid cell width must be greater than zero.",
            ));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // Get the spatial extent
        let (extent, proj_info) = if input_file.to_lowercase().ends_with(".shp") {
            let input = Shapefile::read(&input_file)?;
            (
                BoundingBox::new(
                    input.header.x_min,
                    input.header.x_max,
                    input.header.y_min,
                    input.header.y_max,
                ),
                input.projection,
            )
        } else if input_file.to_lowercase().ends_with(".las") {
            let mut input = LasFile::new(&input_file, "r")?;
            (
                BoundingBox::new(
                    input.header.min_x,
                    input.header.max_x,
                    input.header.min_y,
                    input.header.max_y,
                ),
                input.get_wkt(),
            )
        } else {
            // must be a raster
            let input = Raster::new(&input_file, "r")?;
            (
                BoundingBox::new(
                    input.configs.west,
                    input.configs.east,
                    input.configs.south,
                    input.configs.north,
                ),
                input.configs.coordinate_ref_system_wkt,
            )
        };

        let start_x_grid = (((extent.min_x - xorig) / width).floor()) as i32;
        let end_x_grid = (((extent.max_x - xorig) / width).ceil()) as i32;
        let start_y_grid = (((extent.min_y - yorig) / height).floor()) as i32;
        let end_y_grid = (((extent.max_y - yorig) / height).ceil()) as i32;
        let rows = ((end_y_grid - start_y_grid).abs()) as i32;

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;

        // set the projection information
        if !proj_info.is_empty() && proj_info.to_lowercase() != "not specified" {
            output.projection = proj_info;
        }

        // add the attributes
        let fid = AttributeField::new("FID", FieldDataType::Int, 5u8, 0u8);
        let row_att = AttributeField::new("ROW", FieldDataType::Int, 5u8, 0u8);
        let col_att = AttributeField::new("COLUMN", FieldDataType::Int, 5u8, 0u8);
        output.attributes.add_field(&fid);
        output.attributes.add_field(&row_att);
        output.attributes.add_field(&col_att);
        let mut rec_num = 1i32;
        let (mut x, mut y): (f64, f64);
        let mut r = 0f64;
        for row in start_y_grid..end_y_grid {
            for col in start_x_grid..end_x_grid {
                let mut points: Vec<Point2D> = Vec::with_capacity(5);

                // Point 1
                x = xorig + col as f64 * width;
                y = yorig + row as f64 * height;
                if x < extent.min_x {
                    x = extent.min_x;
                }
                if x > extent.max_x {
                    x = extent.max_x;
                }
                if y < extent.min_y {
                    y = extent.min_y;
                }
                if y > extent.max_y {
                    y = extent.max_y;
                }
                let p1 = Point2D::new(x, y);
                points.push(p1);

                // Point 2
                x = xorig + col as f64 * width;
                y = yorig + (row + 1) as f64 * height;
                if x < extent.min_x {
                    x = extent.min_x;
                }
                if x > extent.max_x {
                    x = extent.max_x;
                }
                if y < extent.min_y {
                    y = extent.min_y;
                }
                if y > extent.max_y {
                    y = extent.max_y;
                }
                points.push(Point2D::new(x, y));

                // Point 2
                x = xorig + (col + 1) as f64 * width;
                y = yorig + (row + 1) as f64 * height;
                if x < extent.min_x {
                    x = extent.min_x;
                }
                if x > extent.max_x {
                    x = extent.max_x;
                }
                if y < extent.min_y {
                    y = extent.min_y;
                }
                if y > extent.max_y {
                    y = extent.max_y;
                }
                points.push(Point2D::new(x, y));

                // Point 3
                x = xorig + (col + 1) as f64 * width;
                y = yorig + row as f64 * height;
                if x < extent.min_x {
                    x = extent.min_x;
                }
                if x > extent.max_x {
                    x = extent.max_x;
                }
                if y < extent.min_y {
                    y = extent.min_y;
                }
                if y > extent.max_y {
                    y = extent.max_y;
                }
                points.push(Point2D::new(x, y));

                // close the polygon
                points.push(Point2D::new(p1.x, p1.y));

                let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
                sfg.add_part(&points);
                output.add_record(sfg);

                output.attributes.add_record(
                    vec![
                        FieldData::Int(rec_num),
                        FieldData::Int(row),
                        FieldData::Int(col),
                    ],
                    false,
                );

                rec_num += 1i32;
            }

            r += 1f64;
            if verbose {
                progress = (100.0_f64 * r / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
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
