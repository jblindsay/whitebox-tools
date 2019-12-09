/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/10/2018
Last Modified: 09/12/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::vector::{FieldData, ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// Creates a raster grid based on a set of vector points and assigns grid values using a block maximum scheme.
pub struct BlockMaximumGridding {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BlockMaximumGridding {
    /// public constructor
    pub fn new() -> BlockMaximumGridding {
        let name = "BlockMaximumGridding".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Creates a raster grid based on a set of vector points and assigns grid values using a block maximum scheme.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector Points file.".to_owned(),
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
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Use z-coordinate instead of field?".to_owned(),
            flags: vec!["--use_z".to_owned()],
            description: "Use z-coordinate instead of field?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --field=ELEV -o=output.tif --cell_size=1.0
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.shp --use_z -o=output.tif --base=existing_raster.tif", short_exe, name).replace("*", &sep);

        BlockMaximumGridding {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BlockMaximumGridding {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut field_name = String::new();
        let mut use_z = false;
        let mut output_file = String::new();
        let mut grid_res = 0f64;
        let mut base_file = String::new();

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
            } else if flag_val == "-use_z" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_z = true;
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-cell_size" {
                grid_res = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-base" {
                base_file = if keyval {
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
        let vector_data = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        // Create the output raster. The process of doing this will
        // depend on whether a cell size or a base raster were specified.
        // If both are specified, the base raster takes priority.

        let nodata = -32768.0f64;

        let mut output = if !base_file.trim().is_empty() || grid_res == 0f64 {
            if !base_file.contains(&sep) && !base_file.contains("/") {
                base_file = format!("{}{}", working_directory, base_file);
            }
            let mut base = Raster::new(&base_file, "r")?;
            base.configs.nodata = nodata;
            Raster::initialize_using_file(&output_file, &base)
        } else {
            // base the output raster on the grid_res and the
            // extent of the input vector.
            let west: f64 = vector_data.header.x_min;
            let north: f64 = vector_data.header.y_max;
            let rows: isize = (((north - vector_data.header.y_min) / grid_res).ceil()) as isize;
            let columns: isize = (((vector_data.header.x_max - west) / grid_res).ceil()) as isize;
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

        // let half_grid_res = grid_res / 2f64;
        let ew_range = output.configs.east - west;
        let ns_range = north - output.configs.south;

        let (mut x, mut y, mut z, mut z_current): (f64, f64, f64, f64);
        let (mut row, mut col): (isize, isize);
        if !use_z {
            // use the specified attribute

            // What is the index of the field to be analyzed?
            let field_index = match vector_data.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    // Field not found
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Attribute not found in table.",
                    ));
                }
            };

            // Is the field numeric?
            if !vector_data.attributes.is_field_numeric(field_index) {
                // Warn user of non-numeric
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Non-numeric attributes cannot be rasterized.",
                ));
            }

            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                x = record.points[0].x;
                y = record.points[0].y;
                z = match vector_data.attributes.get_value(record_num, &field_name) {
                    FieldData::Int(val) => val as f64,
                    FieldData::Real(val) => val,
                    _ => nodata,
                };

                // col = (((columns - 1) as f64 * (x - west - half_grid_res) / ew_range).floor())
                //     as isize;
                // row =
                //     (((rows - 1) as f64 * (north - half_grid_res - y) / ns_range).floor()) as isize;
                col = (((columns - 1) as f64 * (x - west) / ew_range).floor()) as isize;
                row = (((rows - 1) as f64 * (north - y) / ns_range).floor()) as isize;
                z_current = output.get_value(row, col);
                if z_current == nodata || z > z_current {
                    output.set_value(row, col, z);
                }

                if verbose {
                    progress = (100.0_f64 * record_num as f64
                        / (vector_data.num_records - 1) as f64)
                        as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // use the z dimension of the point data.
            if vector_data.header.shape_type != ShapeType::PointZ
                && vector_data.header.shape_type != ShapeType::PointM
                && vector_data.header.shape_type != ShapeType::MultiPointZ
                && vector_data.header.shape_type != ShapeType::MultiPointM
            {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "The input vector data must be of PointZ, PointM, MultiPointZ, or MultiPointM shape type."));
            }

            // let mut p = 0;
            for record_num in 0..vector_data.num_records {
                let record = vector_data.get_record(record_num);
                for i in 0..record.z_array.len() {
                    x = record.points[i].x;
                    y = record.points[i].y;
                    z = record.z_array[i];
                    col = (((columns - 1) as f64 * (x - west) / ew_range).floor()) as isize;
                    row = (((rows - 1) as f64 * (north - y) / ns_range).floor()) as isize;

                    z_current = output.get_value(row, col);
                    if z_current == nodata || z > z_current {
                        output.set_value(row, col, z);
                    }
                }

                if verbose {
                    progress = (100.0_f64 * record_num as f64
                        / (vector_data.num_records - 1) as f64)
                        as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        };

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
