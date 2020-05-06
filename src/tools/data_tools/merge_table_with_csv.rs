/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Prof. John Lindsay
Created: 11/10/2018
Last Modified: 09/03/2020
License: MIT
*/

use crate::tools::*;
use crate::vector::{AttributeField, FieldData, FieldDataType, Shapefile};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Error, ErrorKind};
use std::path;
use std::{f64, i32};

/// This tool can be used to merge a vector's attribute table with data contained within a comma
/// separated values (CSV) text file. CSV files stores tabular data (numbers and text) in plain-text
/// form such that each row is a record and each column a field. Fields are typically separated by
/// commas although the tool will also support seimi-colon, tab, and space delimited files. The user
/// must specify the name of the vector (and associated attribute file) as well as the *primary key*
/// within the table. The *primary key* (`--pkey` flag) is the field within the
/// table that is being appended to that serves as the unique identifier. Additionally, the user must
/// specify the name of a CSV text file with either a *.csv or *.txt extension. The file must possess a
/// header row, i.e. the first row must contain information about the names of the various fields. The
/// *foreign key* (`--fkey` flag), that is the identifying field within the
/// CSV file that corresponds with the data contained within the *primary key* in the table, must also
/// be specified. Both the primary and foreign keys should either be strings (text) or integer values.
/// *Fields containing decimal values are not good candidates for keys.* Lastly, the user may optionally
/// specify the name of a field within the CSV file to import in the merge operation (`--import_field` flag).
/// If this flag is not specified, all of the fields within the CSV, with the exception of the foreign
/// key, will be appended to the attribute table.
///
/// Merging works for one-to-one and many-to-one database relations. A *one-to-one* relations exists when
/// each record in the attribute table corresponds to one record in the second table and each primary
/// key is unique. Since each record in the attribute table is associated with a geospatial feature in
/// the vector, an example of a one-to-one relation may be where the second file contains AREA and
/// PERIMETER fields for each polygon feature in the vector. This is the most basic type of relation.
/// A many-to-one relation would exist when each record in the first attribute table corresponds to one
/// record in the second file and the primary key is NOT unique. Consider as an example a vector and
/// attribute table associated with a world map of countries. Each country has one or more more polygon
/// features in the shapefile, e.g. Canada has its mainland and many hundred large islands. You may want
/// to append a table containing data about the population and area of each country. In this case, the
/// COUNTRY columns in the attribute table and the second file serve as the primary and foreign keys
/// respectively. While there may be many duplicate primary keys (all of those Canadian polygons) each
/// will correspond to only one foreign key containing the population and area data. This is a
/// *many-to-one* relation. The `JoinTables` tool does not support one-to-many nor many-to-many relations.
///
/// # See Also
/// `JoinTables`, `ReinitializeAttributeTable`, `ExportTableToCsv`
pub struct MergeTableWithCsv {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MergeTableWithCsv {
    /// public constructor
    pub fn new() -> MergeTableWithCsv {
        let name = "MergeTableWithCsv".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Merge a vector's attribute table with a table contained within a CSV text file."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Primary Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input primary vector file (i.e. the table to be modified).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Primary Key Field".to_owned(),
            flags: vec!["--pkey".to_owned()],
            description: "Primary key field.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input CSV File".to_owned(),
            flags: vec!["--csv".to_owned()],
            description: "Input CSV file (i.e. source of data to be imported).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Csv),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Foreign Key Field".to_owned(),
            flags: vec!["--fkey".to_owned()],
            description: "Foreign key field.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--csv".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Imported Field".to_owned(),
            flags: vec!["--import_field".to_owned()],
            description: "Imported field (all fields will be imported if not specified)."
                .to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--csv".to_string(),
            ),
            default_value: None,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=properties.shp --pkey=TYPE --csv=land_class.csv --fkey=VALUE --import_field=NEW_VALUE
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=properties.shp --pkey=TYPE --csv=land_class.csv --fkey=VALUE",
            short_exe, name
        ).replace("*", &sep);

        MergeTableWithCsv {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MergeTableWithCsv {
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
        let mut primary_key = String::new();
        let mut csv_file = String::new();
        let mut foreign_key = String::new();
        let mut import_field = String::new();

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
            } else if flag_val == "-primary_key" || flag_val == "-pkey" {
                primary_key = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-csv" {
                csv_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-foreign_key" || flag_val == "-fkey" {
                foreign_key = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-import_field" || flag_val == "-import" {
                import_field = if keyval {
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
        if !csv_file.contains(&sep) && !csv_file.contains("/") {
            csv_file = format!("{}{}", working_directory, csv_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // read in the CSV file
        let mut data_map = HashMap::new();
        let (mut pkey_value, mut fkey_value): (String, String);
        let f = File::open(csv_file.clone())?;
        let f = BufReader::new(f);
        let mut csv_headers: Vec<String> = vec![];
        let mut csv_num_fields = 0;
        let mut field_types = vec![];
        let mut record_num = 0;
        let mut delimiter = ",";
        let mut fkey_index = 99999;
        let mut import_index = 99999;
        let mut field_indices_to_append = vec![];
        let mut field_lengths: Vec<u8> = vec![];
        let mut field_precision: Vec<u8> = vec![];
        let mut a: usize;
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
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
                    if csv_headers[i] == foreign_key {
                        fkey_index = i;
                    }
                    if csv_headers[i] == import_field {
                        import_index = i;
                    }
                }
                // was the foreign key found?
                if fkey_index == 99999 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Foreign Key was not located in table.",
                    ));
                }
            } else {
                // is the record an appropriate length?
                if line_vec.len() != csv_num_fields {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Not all records in the CSV file are the same length. Cannot read the table.",
                    ));
                }
                fkey_value = line_vec[fkey_index].to_string();
                if record_num == 1 {
                    // the first data record
                    if import_index > csv_num_fields {
                        // import all of the fields except the foreign key
                        for i in 0..csv_num_fields {
                            if i != fkey_index {
                                field_types.push(get_type(line_vec[i]));
                                field_indices_to_append.push(i);
                            }
                        }
                    } else {
                        // only import the import field
                        field_types.push(get_type(line_vec[import_index]));
                        field_indices_to_append.push(import_index);
                    }
                    field_lengths = vec![0u8; field_indices_to_append.len()];
                    field_precision = vec![0u8; field_indices_to_append.len()];
                }
                let mut imported_data = vec![];
                for i in 0..field_indices_to_append.len() {
                    a = field_indices_to_append[i];
                    if line_vec[a].len() as u8 > field_lengths[i] {
                        field_lengths[i] = line_vec[a].len() as u8;
                    }
                    match field_types[i] {
                        FieldDataType::Int => imported_data
                            .push(FieldData::Int(line_vec[a].trim().parse::<i32>().unwrap())),
                        FieldDataType::Real => {
                            let prec = get_precision(line_vec[a]);
                            if prec > field_precision[i] {
                                field_precision[i] = prec;
                            }
                            imported_data
                                .push(FieldData::Real(line_vec[a].trim().parse::<f64>().unwrap()))
                        }
                        FieldDataType::Bool => imported_data
                            .push(FieldData::Bool(line_vec[a].trim().parse::<bool>().unwrap())),
                        FieldDataType::Text => {
                            imported_data.push(FieldData::Text(line_vec[a].trim().to_string()))
                        }
                        FieldDataType::Date => {
                            imported_data.push(FieldData::Text(line_vec[a].trim().to_string()))
                        }
                    }
                }
                data_map.insert(fkey_value, imported_data.clone());
            }
            record_num += 1;
        }

        // create output file
        let mut output =
            Shapefile::initialize_using_file(&input_file, &input, input.header.shape_type, true)?;

        // update the vector1 attribute table
        for i in 0..field_indices_to_append.len() {
            a = field_indices_to_append[i];
            output.attributes.add_field(&AttributeField::new(
                &csv_headers[a],
                field_types[i].clone(),
                field_lengths[i],
                field_precision[i],
            ));
        }

        // print the attribute data
        for record_num in 0..input.num_records {
            // geometries
            let record = input.get_record(record_num);
            output.add_record(record.clone());
            // attributes
            let mut atts = input.attributes.get_record(record_num);

            pkey_value = match input.attributes.get_value(record_num, &primary_key) {
                FieldData::Int(v) => v.to_string(),
                FieldData::Real(v) => v.to_string(),
                FieldData::Text(v) => v.to_string(),
                FieldData::Date(v) => v.to_string(),
                FieldData::Bool(v) => v.to_string(),
                FieldData::Null => "null".to_string(),
            };

            match data_map.get(&pkey_value) {
                Some(v) => {
                    for a in v {
                        atts.push(a.clone());
                    }
                }
                None => {
                    // return Err(Error::new(
                    //     ErrorKind::InvalidInput,
                    //     "Error mapping primary key value to foreign key value.",
                    // ));

                    // add nulls to the att table
                    for _ in 0..field_indices_to_append.len() {
                        atts.push(FieldData::Null);
                    }
                }
            }

            output.attributes.add_record(atts, false);

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
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
