/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/07/2017
Last Modified: 27/08/2021
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use whitebox_vector::Shapefile;

/// This tool can be used to create a new raster with the same coordinates and dimensions
/// (i.e. rows and columns) as an existing base image, or the same spatial extent as an input
/// vector file. The user must specify the name of the
/// base file (`--base`), the value that the new grid will be filled with (`--value` flag;
/// default of nodata), and the data type (`--data_type` flag; options include 'double',
/// 'float', and 'integer'). If an input vector base file is used, then it is necessary to specify
/// a value for the optional grid cell size (`--cell_size`) input parameter.
///
/// # See Also
/// `RasterCellAssignment`
pub struct NewRasterFromBase {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NewRasterFromBase {
    pub fn new() -> NewRasterFromBase {
        // public constructor
        let name = "NewRasterFromBase".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Creates a new raster using a base image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["-i".to_owned(), "--base".to_owned()],
            description: "Input base raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::RasterAndVector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Constant Value".to_owned(),
            flags: vec!["--value".to_owned()],
            description: "Constant value to fill raster with; either 'nodata' or numeric value."
                .to_owned(),
            parameter_type: ParameterType::StringOrNumber,
            default_value: Some("nodata".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Data Type".to_owned(), 
            flags: vec!["--data_type".to_owned()], 
            description: "Output raster data type; options include 'double' (64-bit), 'float' (32-bit), and 'integer' (signed 16-bit) (default is 'float').".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["double".to_owned(), "float".to_owned(), "integer".to_owned()]),
            default_value: Some("float".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=base.tif -o=NewRaster.tif --value=0.0 --data_type=integer
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=base.tif -o=NewRaster.tif --value=nodata", short_exe, name).replace("*", &sep);

        NewRasterFromBase {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NewRasterFromBase {
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
        let mut base_file = String::new();
        let mut output_file = String::new();
        let mut out_val_str = String::new();
        let mut data_type = String::new();
        let mut cell_size = 0f64;

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
            if flag_val == "-base" || flag_val == "-i" || flag_val == "-input" {
                base_file = if keyval {
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
            } else if flag_val == "-value" {
                out_val_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-data_type" || flag_val == "-datatype" {
                data_type = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-cell_size" {
                cell_size = if keyval {
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
            }
        }

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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !base_file.contains(&sep) && !base_file.contains("/") {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        let nodata = -32768.0;
        let mut out_val = nodata;
        if out_val_str.to_lowercase() != "nodata" {
            // try to parse the value
            out_val = out_val_str.parse::<f64>().unwrap();
        }

        // Get the spatial extent
        let mut output = if base_file.to_lowercase().ends_with(".shp") {
            // Note that this only works because at the moment, Shapefiles are the only supported vector.
            // If additional vector formats are added in the future, this will need updating.

            if cell_size <= 0f64 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The cell_size parameter must be set to a non-zero positive value if a vector base file is specified.",
                ));
            }

            let input = Shapefile::read(&base_file)?;
            // base the output raster on the cell_size and the
            // extent of the input vector.
            let west: f64 = input.header.x_min;
            let north: f64 = input.header.y_max;
            let rows: isize = (((north - input.header.y_min) / cell_size).ceil()) as isize;
            let columns: isize = (((input.header.x_max - west) / cell_size).ceil()) as isize;
            let south: f64 = north - rows as f64 * cell_size;
            let east = west + columns as f64 * cell_size;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows as usize;
            configs.columns = columns as usize;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = cell_size;
            configs.resolution_y = cell_size;
            configs.nodata = nodata;
            configs.photometric_interp = PhotometricInterpretation::Continuous;
            configs.projection = input.projection.clone();

            Raster::initialize_using_config(&output_file, &configs)
        } else {
            let base = Raster::new(&base_file, "r")?;

            Raster::initialize_using_file(&output_file, &base)
        };

        if output.configs.nodata != nodata || out_val != nodata {
            output.configs.nodata = nodata;
            output.reinitialize_values(out_val);
        }


        if data_type.to_lowercase().contains("i") {
            output.configs.data_type = DataType::I16;
        } else if data_type.to_lowercase().contains("d") {
            output.configs.data_type = DataType::F64;
        } else {
            output.configs.data_type = DataType::F32;
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Base raster file: {}", base_file));
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
