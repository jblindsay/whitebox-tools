/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 24/04/2018
Last Modified: 18/10/2019
License: MIT
*/

use crate::tools::*;
use whitebox_vector::{FieldData, Shapefile};
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufWriter, Error, ErrorKind};
use std::path;

/// This tool can be used to export a vector's attribute table to a comma separated values (CSV)
/// file. CSV files stores tabular data (numbers and text) in plain-text form such that each row
/// corresponds to a record and each column to a field. Fields are typically separated by commas
/// within records. The user must specify the name of the vector (and associated attribute file),
/// the name of the output CSV file, and whether or not to include the field names as a header
/// column in the output CSV file.
///
/// # See Also
/// `MergeTableWithCsv`
pub struct ExportTableToCsv {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ExportTableToCsv {
    /// public constructor
    pub fn new() -> ExportTableToCsv {
        let name = "ExportTableToCsv".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Exports an attribute table to a CSV text file.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output csv file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Csv),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Export field names as file header?".to_owned(),
            flags: vec!["--headers".to_owned()],
            description: "Export field names as file header?".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lines.shp -o=output.csv --headers",
            short_exe, name
        )
        .replace("*", &sep);

        ExportTableToCsv {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ExportTableToCsv {
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
        let mut headers = false;

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
            } else if flag_val == "-headers" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    headers = true;
                }
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

        let f = File::create(&output_file)?;
        let mut writer = BufWriter::new(f);

        if headers {
            // print the field names
            let mut s = String::new();
            for field in &vector_data.attributes.fields {
                s.push_str(&format!(",{}", field.name));
            }
            s = s.trim_start_matches(',').to_string();
            s.push_str("\n");
            writer.write_all(s.as_bytes())?;
        }

        // print the attribute data
        let num_fields = vector_data.attributes.header.num_fields as usize;
        let mut num_dec: f64;
        let mut multiplier: f64;
        for record_num in 0..vector_data.num_records {
            let mut s = String::new();
            let rec = vector_data.attributes.get_record(record_num);
            for field_num in 0..num_fields {
                num_dec = vector_data.attributes.fields[field_num].decimal_count as f64;
                multiplier = 10f64.powf(num_dec);
                match rec[field_num] {
                    FieldData::Int(ref val) => {
                        s.push_str(&format!(",{}", val));
                    }
                    FieldData::Real(ref val) => {
                        s.push_str(&format!(",{}", (val * multiplier).round() / multiplier));
                    }
                    FieldData::Text(ref val) => {
                        s.push_str(&format!(",\"{}\"", val));
                    }
                    FieldData::Date(ref val) => {
                        s.push_str(&format!(",{}", val));
                    }
                    FieldData::Bool(ref val) => {
                        s.push_str(&format!(",{}", val));
                    }
                    _ => {
                        s.push_str(",null");
                    }
                }
            }

            s = s.trim_start_matches(',').to_string();
            s.push_str("\n");
            writer.write_all(s.as_bytes())?;

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Writing attributes: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
