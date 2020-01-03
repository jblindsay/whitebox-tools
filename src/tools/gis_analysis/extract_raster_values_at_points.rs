/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/06/2018
Last Modified: 18/10/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to extract the values of one or more rasters (`--inputs`) at the sites of a set of vector points.
/// By default, the data is output to the attribute table of the input points (`--points`) vector; however,
/// if the `--out_text` parameter is specified, the tool will additionally output point values as text data
/// to standard output (*stdout*). Attribute fields will be added to the table of the points file, with field
/// names, *VALUE1*, *VALUE2*, *VALUE3*, etc. each corresponding to the order of input rasters.
///
/// If you need to plot a chart of values from a raster stack at a set of points, the `ImageStackProfile` may be
/// more suitable for this application.
///
/// # See Also
/// `ImageStackProfile`, `FindLowestOrHighestPoints`
pub struct ExtractRasterValuesAtPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ExtractRasterValuesAtPoints {
    pub fn new() -> ExtractRasterValuesAtPoints {
        // public constructor
        let name = "ExtractRasterValuesAtPoints".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Extracts the values of raster(s) at vector point locations.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input raster files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Points File".to_owned(),
            flags: vec!["--points".to_owned()],
            description: "Input vector points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output text?".to_owned(),
            flags: vec!["--out_text".to_owned()],
            description:
                "Output point values as text? Otherwise, the only output is to to the points file's attribute table."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='image1.tif;image2.tif;image3.tif' -points=points.shp", short_exe, name).replace("*", &sep);

        ExtractRasterValuesAtPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ExtractRasterValuesAtPoints {
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
        let mut input_files = String::new();
        let mut points_file = String::new();
        let mut output_text = false;

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
            if flag_val == "-i" || flag_val == "-inputs" {
                input_files = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-points" {
                points_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val.contains("-out_text") {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    output_text = true;
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let start = Instant::now();

        let mut cmd = input_files.split(";");
        let mut v = cmd.collect::<Vec<&str>>();
        if v.len() == 1 {
            cmd = input_files.split(",");
            v = cmd.collect::<Vec<&str>>();
        }
        let num_files = v.len();
        if num_files < 1 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input files. At least one input is required to operate this tool."));
        }

        let mut points = Shapefile::read(&points_file)?;
        points.file_mode = "rw".to_string(); // we need to be able to modify the attributes table
        let num_records = points.num_records;

        // make sure the input vector file is of points type
        if points.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of Point base shape type.",
            ));
        }

        let (mut row, mut col): (isize, isize);
        let mut x_vals = Vec::with_capacity(num_records);
        let mut y_vals = Vec::with_capacity(num_records);
        let mut raster_values = vec![vec![0f64; num_files]; num_records];
        for record_num in 0..num_records {
            let record = points.get_record(record_num);
            y_vals.push(record.points[0].y);
            x_vals.push(record.points[0].x);
        }

        // add the attributes for each raster
        for i in 0..num_files {
            if !v[i].trim().is_empty() {
                let val =
                    AttributeField::new(&format!("VALUE{}", i + 1), FieldDataType::Real, 12u8, 6u8);
                points.attributes.add_field(&val);
            }
        }

        let mut z: f64;
        let mut i = 1;
        for value in v {
            if !value.trim().is_empty() {
                if verbose {
                    println!("Reading data...")
                };

                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;

                for record_num in 0..num_records {
                    row = input.get_row_from_y(y_vals[record_num]);
                    col = input.get_column_from_x(x_vals[record_num]);
                    z = input.get_value(row, col);
                    points.attributes.set_value(
                        record_num,
                        &format!("VALUE{}", i),
                        FieldData::Real(z),
                    );

                    if output_text {
                        raster_values[record_num][i - 1] = z;
                    }
                }

                i += 1;
            }
        }

        if output_text {
            println!("Point values:");
            for record_num in 0..num_records {
                println!(
                    "Point {} values: {:?}",
                    record_num + 1,
                    raster_values[record_num]
                );
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        let _ = match points.write() {
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
