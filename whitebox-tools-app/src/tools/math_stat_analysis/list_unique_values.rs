/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/04/2018
Last Modified: 13/10/2018
License: MIT
*/

use whitebox_common::rendering::html::*;
use crate::tools::*;
use whitebox_vector::{FieldData, Shapefile};
use std::collections::HashMap;
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to list each of the unique values contained within a categorical field
/// of an input vector file's attribute table. The tool outputs an HTML formated report (`--output`)
/// containing a table of the unique values and their frequency of occurrence within the data. The user must
/// specify the name of an input shapefile (`--input`) and the name of one of the fields (`--field`)
/// contained in the associated attribute table. The specified field *should not contained floating-point
/// numerical data*, since the number of categories will likely equal the number of records, which may be
/// quite large. The tool effectively provides tabular output that is similar to the graphical output
/// provided by the `AttributeHistogram` tool, which, however, can be applied to continuous data.
///
/// # See Also
/// `AttributeHistogram`
pub struct ListUniqueValues {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ListUniqueValues {
    pub fn new() -> ListUniqueValues {
        // public constructor
        let name = "ListUniqueValues".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Lists the unique values contained in a field witin a vector's attribute table."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Input field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output HTML file (default name will be based on input file if unspecified)."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lakes.shp --field=HEIGHT -o=outfile.html",
            short_exe, name
        )
        .replace("*", &sep);

        ListUniqueValues {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ListUniqueValues {
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
        let mut input_file = String::new();
        let mut field_name = String::new();
        let mut output_file = String::new();

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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
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

        let start = Instant::now();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading vector data...")
        };

        let vector_data = Shapefile::read(&input_file)?;

        let mut freq_data = HashMap::new();
        let mut key: String;
        for record_num in 0..vector_data.num_records {
            key = match vector_data.attributes.get_value(record_num, &field_name) {
                FieldData::Int(val) => val.to_string(),
                FieldData::Real(val) => val.to_string(),
                FieldData::Text(val) => val.to_string(),
                FieldData::Date(val) => val.to_string(),
                FieldData::Bool(val) => val.to_string(),
                FieldData::Null => "null".to_string(),
            };
            // if key != "null" {
            let count = freq_data.entry(key).or_insert(0);
            *count += 1;
            // }

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Reading attribute data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if freq_data.len() > 250 {
            if verbose {
                println!("Warning: There are a large number of categories. A continuous attribute variable may have been input incorrectly.");
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "\n{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        let f = File::create(output_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <head>
            <meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">
            <title>List Unique Values</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>List Unique Values</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(
            &format!("<p><strong>Input</strong>: {}</p>", input_file.clone()).as_bytes(),
        )?;
        writer.write_all(
            &format!("<p><strong>Field Name</strong>: {}</p>", field_name.clone()).as_bytes(),
        )?;

        // The output table
        let mut s = "<p><table>
        <caption>Category Data</caption>
        <tr>
            <th class=\"headerCell\">Category</th>
            <th class=\"headerCell\">Frequency</th>
        </tr>";
        writer.write_all(s.as_bytes())?;

        if freq_data.contains_key("null") {
            match freq_data.get("null") {
                Some(count) => {
                    let s1 = &format!(
                        "<tr>
                        <td>null</td>
                    <td class=\"numberCell\">{}</td>
                        </tr>\n",
                        count
                    );
                    writer.write_all(s1.as_bytes())?;
                }
                None => {}
            }
        }

        for (category, count) in &freq_data {
            if category != "null" {
                let s1 = &format!(
                    "<tr>
                    <td>{}</td>
                    <td class=\"numberCell\">{}</td>
                </tr>\n",
                    category, count
                );
                writer.write_all(s1.as_bytes())?;
            }
        }

        s = "</table></p>";
        writer.write_all(s.as_bytes())?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        if verbose {
            if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                let output = Command::new("open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "windows") {
                // let output = Command::new("cmd /c start")
                let output = Command::new("explorer.exe")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "linux") {
                let output = Command::new("xdg-open")
                    .arg(output_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            }
            if verbose {
                println!("Complete! Please see {} for output.", output_file);
            }
        }

        Ok(())
    }
}
