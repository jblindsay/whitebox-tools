/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Prof. John Lindsay
Created: 07/08/2019
Last Modified: 28/01/2020
License: MIT
*/

use whitebox_common::spatial_ref_system::esri_wkt_from_epsg;
use crate::tools::*;
use whitebox_vector::{AttributeField, FieldData, FieldDataType, ShapeType, Shapefile};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Error, ErrorKind};
use std::path;
use std::{f64, i32};

/// This tool can be used to import a series of points contained within a comma-separated values
/// (*.csv) file (`--input`) into a vector shapefile of a POINT ShapeType. The input file must be an ASCII text
/// file with a .csv extensions. The tool will automatically detect the field data type; for numeric
/// fields, it will also determine the appropriate length and precision. The user must specify the
/// x-coordinate (`--xfield`) and y-coordiante (`--yfield`) fields. All fields are imported as
/// attributes in the output (`--output`) vector file. The tool assumes that the first line of the file is a header line from which field
/// names are retrieved.
///
/// # See Also
/// `MergeTableWithCsv`, `ExportTableToCsv`
pub struct CsvPointsToVector {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CsvPointsToVector {
    /// public constructor
    pub fn new() -> CsvPointsToVector {
        let name = "CsvPointsToVector".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a CSV text file to vector points.".to_string();

        let mut parameters = vec![];

        parameters.push(ToolParameter {
            name: "Input CSV File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input CSV file (i.e. source of data to be imported).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Csv),
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
            name: "X Field Number (zero-based)".to_owned(),
            flags: vec!["--xfield".to_owned()],
            description: "X field number (e.g. 0 for first field).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Y Field Number (zero-based)".to_owned(),
            flags: vec!["--yfield".to_owned()],
            description: "Y field number (e.g. 1 for second field).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "EPSG Projection".to_owned(),
            flags: vec!["--epsg".to_owned()],
            description: "EPSG projection (e.g. 2958).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: true,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=points.csv -o=points.shp --xfield=0 --yfield=1 --epsg=4326",
            short_exe, name
        ).replace("*", &sep);

        CsvPointsToVector {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CsvPointsToVector {
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
        let mut output_file = String::new();
        // let mut field_definitions = String::new();
        let mut x_field = 0;
        let mut y_field = 1;
        let mut epsg = 0u16;
        let mut projection_set = false;

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
            } else if flag_val == "-xfield" {
                x_field = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-yfield" {
                y_field = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-epsg" {
                epsg = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as u16
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val)) as u16
                };
                projection_set = true;
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        // File strings need a full directory
        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        if verbose {
            println!("Reading data...")
        };

        // read in the CSV file
        let mut data = vec![];
        let f = match File::open(input_file.clone()) {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Error opening the CSV file.",
                ));
            }
        };
        let f = BufReader::new(f);
        let mut csv_headers: Vec<String> = vec![];
        let mut csv_num_fields = 0;
        let mut field_types = vec![];
        let mut record_num = 0;
        let mut delimiter = ",";
        let mut field_indices_to_append = vec![];
        let mut field_lengths: Vec<u8> = vec![];
        let mut field_precision: Vec<u8> = vec![];
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            if !line_unwrapped.trim().is_empty() {
                let mut line_split = line_unwrapped.split(delimiter);
                let mut line_vec = line_split.collect::<Vec<&str>>();
                if line_vec.len() == 1 {
                    delimiter = ";";
                    line_split = line_unwrapped.split(delimiter);
                    line_vec = line_split.collect::<Vec<&str>>();
                    if line_vec.len() == 1 {
                        delimiter = " ";
                        line_split = line_unwrapped.split(delimiter);
                        line_vec = line_split.collect::<Vec<&str>>();
                    }
                }
                if record_num == 0 {
                    csv_num_fields = line_vec.len();
                    for i in 0..csv_num_fields {
                        csv_headers.push(line_vec[i].trim().to_owned());
                    }
                } else {
                    // is the record an appropriate length?
                    if line_vec.len() != csv_num_fields {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Not all records in the CSV file are the same length. Cannot read the table.",
                        ));
                    }
                    if record_num == 1 {
                        // the first data record
                        for a in 0..csv_num_fields {
                            if a == x_field || a == y_field {
                                field_types.push(FieldDataType::Real); // It has to be floating point data.
                            } else {
                                field_types.push(get_type(line_vec[a]));
                            }
                            field_indices_to_append.push(a);
                        }
                        field_lengths = vec![0u8; csv_num_fields];
                        field_precision = vec![0u8; csv_num_fields];
                    }
                    let mut imported_data: Vec<FieldData> = Vec::with_capacity(csv_num_fields);
                    for a in 0..csv_num_fields {
                        if line_vec[a].len() as u8 > field_lengths[a] {
                            field_lengths[a] = line_vec[a].len() as u8;
                        }
                        if a == x_field || a == y_field {
                            let prec = get_precision(line_vec[a]);
                            if prec > field_precision[a] {
                                field_precision[a] = prec;
                            }
                            imported_data
                                .push(FieldData::Real(line_vec[a].trim().parse::<f64>().unwrap()))
                        } else {
                            match field_types[a] {
                                FieldDataType::Int => imported_data.push(
                                    match line_vec[a].trim().parse::<i32>() {
                                        Ok(value) => FieldData::Int(value),
                                        Err(_e) => {
                                            if line_vec[a].contains(".") {
                                                field_types[a] = FieldDataType::Real;
                                                match line_vec[a].trim().parse::<f64>() {
                                                    Ok(value) => {
                                                        let prec = get_precision(line_vec[a]);
                                                        if prec > field_precision[a] {
                                                            field_precision[a] = prec;
                                                        }
                                                        FieldData::Real(value)
                                                    },
                                                    Err(_e) => FieldData::Null
                                                }
                                            } else {
                                                FieldData::Null
                                            }
                                        },
                                    }
                                ),
                                FieldDataType::Real => {
                                    match line_vec[a].trim().parse::<f64>() {
                                        Ok(value) => {
                                            let prec = get_precision(line_vec[a]);
                                            if prec > field_precision[a] {
                                                field_precision[a] = prec;
                                            }
                                            imported_data.push(FieldData::Real(value))
                                        },
                                        Err(_e) => imported_data.push(FieldData::Null)
                                    }
                                }
                                FieldDataType::Bool => imported_data.push(
                                    match line_vec[a].trim().parse::<bool>() {
                                        Ok(value) => FieldData::Bool(value),
                                        Err(_e) => FieldData::Null,
                                    }
                                ),
                                FieldDataType::Text => imported_data.push(
                                    FieldData::Text(line_vec[a].trim().to_string())
                                ),
                                FieldDataType::Date => imported_data
                                    .push(FieldData::Text(line_vec[a].trim().to_string())),
                            }
                        }
                    }
                    data.push(imported_data);
                }
            }
            record_num += 1;
        }

        // make sure that the x and y fields are numeric
        if field_types[x_field] != FieldDataType::Real
            || field_types[y_field] != FieldDataType::Real
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Either the x or y fields, or both, do not contain floating-point numerical data.",
            ));
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::Point)?;

        if projection_set {
            // set the projection information
            output.projection = esri_wkt_from_epsg(epsg.clone());
        }

        // add the attributes
        for a in 0..csv_num_fields {
            output.attributes.add_field(&AttributeField::new(
                &csv_headers[a],
                field_types[a].clone(),
                field_lengths[a],
                field_precision[a],
            ));
        }

        // print the attribute data
        let (mut x, mut y): (f64, f64);
        let mut rec_num = 1i32;
        for record_num in 0..data.len() {
            // geometries
            x = match data[record_num][x_field] {
                FieldData::Real(v) => v,
                _ => 0f64,
            };
            y = match data[record_num][y_field] {
                FieldData::Real(v) => v,
                _ => 0f64,
            };

            output.add_point_record(x, y);

            // attributes
            rec_num += 1;
            output
                .attributes
                .add_record(data[record_num].clone(), false);

            if verbose {
                progress = (100.0_f64 * (rec_num + 1) as f64 / data.len() as f64) as usize;
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

fn get_type(s: &str) -> FieldDataType {
    if s.trim().parse::<i32>().unwrap_or(i32::MIN) != i32::MIN {
        if s.trim().contains(".0") {
            return FieldDataType::Real;
        } else {
            return FieldDataType::Int;
        }
    } else if s.trim().parse::<f64>().unwrap_or(f64::INFINITY) != f64::INFINITY {
        return FieldDataType::Real;
    }
    let is_bool = match s.trim().to_lowercase().parse::<bool>() {
        Ok(_) => true,
        Err(_) => false,
    };
    if is_bool {
        return FieldDataType::Bool;
    }
    // There's no easy way to parse data type strings.
    FieldDataType::Text
}

fn get_precision(s: &str) -> u8 {
    let dec_pos = match s.chars().position(|c| c == '.') {
        Some(p) => p,
        None => return 0u8,
    };
    (s.len() - dec_pos - 1) as u8
}
