/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 31/09/2018
Last Modified: 09/09/2021
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::algorithms::is_clockwise_order;
use whitebox_common::structures::Point2D;
use crate::tools::*;
use whitebox_vector::ShapefileGeometry;
use whitebox_vector::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool creates a vector polygon footprint of the area covered by a raster grid or vector
/// layer. It will create a vector rectangle corresponding to the bounding box. The user must
/// specify the name of the input file, which may be either a Whitebox raster or a vector, and
/// the name of the output file.
///
/// If an input raster grid is specified which has an irregular shape, i.e. it contains NoData
/// values at the edges, the resulting vector will still correspond to the full grid extent,
/// ignoring the irregular boundary. If this is not the desired effect, you should reclass the
/// grid such that all cells containing valid values are assigned some positive, non-zero value,
/// and then use the `RasterToVectorPolygons` tool to vectorize the irregular-shaped extent
/// boundary.
///
/// # See Also
/// `MinimumBoundingEnvelope`, `RasterToVectorPolygons`
pub struct LayerFootprint {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LayerFootprint {
    pub fn new() -> LayerFootprint {
        // public constructor
        let name = "LayerFootprint".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a vector polygon footprint of the area covered by a raster grid or vector layer."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster or Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster or vector file.".to_owned(),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.shp",
            short_exe, name
        )
        .replace("*", &sep);

        LayerFootprint {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LayerFootprint {
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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let start = Instant::now();

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

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // is it a vector or a raster file?
        if input_file.to_lowercase().ends_with(".shp") {
            // The input file is a vector
            let input = Shapefile::read(&input_file)?;

            // create output file
            let mut output =
                Shapefile::initialize_using_file(&output_file, &input, ShapeType::Polygon, false)?;
            // Add an FID attribute to the table
            output
                .attributes
                .add_field(&AttributeField::new("FID", FieldDataType::Int, 3u8, 0u8));

            let mut envelope_points = vec![];
            envelope_points.push(Point2D::new(input.header.x_min, input.header.y_min));
            envelope_points.push(Point2D::new(input.header.x_max, input.header.y_min));
            envelope_points.push(Point2D::new(input.header.x_max, input.header.y_max));
            envelope_points.push(Point2D::new(input.header.x_min, input.header.y_max));
            envelope_points.push(Point2D::new(input.header.x_min, input.header.y_min));

            if !is_clockwise_order(&envelope_points) {
                // the first part is assumed to be the hull and must be in clockwise order.
                envelope_points.reverse();
            }

            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
            sfg.add_part(&envelope_points);
            output.add_record(sfg);
            output
                .attributes
                .add_record(vec![FieldData::Int(1i32)], false);

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
        } else {
            // it's likely a raster file instead
            let input = Raster::new(&input_file, "r")?;

            // create output file
            let mut output = Shapefile::new(&output_file, ShapeType::Polygon)?;
            output.projection = input.configs.coordinate_ref_system_wkt.clone();
            // Add an FID attribute to the table
            output
                .attributes
                .add_field(&AttributeField::new("FID", FieldDataType::Int, 3u8, 0u8));

            let mut envelope_points = vec![];
            envelope_points.push(Point2D::new(input.configs.west, input.configs.south));
            envelope_points.push(Point2D::new(input.configs.east, input.configs.south));
            envelope_points.push(Point2D::new(input.configs.east, input.configs.north));
            envelope_points.push(Point2D::new(input.configs.west, input.configs.north));
            envelope_points.push(Point2D::new(input.configs.west, input.configs.south));

            let mut sfg = ShapefileGeometry::new(ShapeType::Polygon);
            sfg.add_part(&envelope_points);
            output.add_record(sfg);
            output
                .attributes
                .add_record(vec![FieldData::Int(1i32)], false);

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
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
