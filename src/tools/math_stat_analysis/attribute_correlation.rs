/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/04/2018
Last Modified: 12/10/2018
License: MIT

NOTES: Correlation is calculated for each pair of numeric attributes.
*/

use crate::rendering::html::*;
use crate::tools::*;
use crate::vector::{FieldData, Shapefile};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to estimate the Pearson product-moment correlation coefficient (*r*) for each pair among a 
/// group of attributes associated with the database file of a shapefile. The *r*-value is a measure of the linear 
/// association in the variation of the attributes. The coefficient ranges from -1, indicated a perfect negative 
/// linear association, to 1, indicated a perfect positive linear association. An *r*-value of 0 indicates no correlation 
/// between the test variables.
/// 
/// Notice that this index is a measure of the linear association; two variables may be strongly related by a non-linear 
/// association (e.g. a power function curve) which will lead to an apparent weak association based on the Pearson 
/// coefficient. In fact, non-linear associations are very common among spatial variables, e.g. terrain indices such as 
/// slope and contributing area. In such cases, it is advisable that the input images are transformed prior to the 
/// estimation of the Pearson coefficient, or that an alternative, non-parametric statistic be used, e.g. the Spearman 
/// rank correlation coefficient.
/// 
/// The user must specify the name of the input vector Shapefile (`--input`). Correlations will be calculated for each 
/// pair of numerical attributes contained within the input file's attribute table and presented in a correlation matrix 
/// HMTL output (`--output`).
/// 
/// # See Also
/// `ImageCorrelation`, `AttributeScattergram`, `AttributeHistogram`
pub struct AttributeCorrelation {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AttributeCorrelation {
    pub fn new() -> AttributeCorrelation {
        // public constructor
        let name = "AttributeCorrelation".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Performs a correlation analysis on attribute fields from a vector database."
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
            name: "Output HTML File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output HTML file (default name will be based on input file if unspecified)."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Html),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.shp -o=outfile.html",
            short_exe, name
        )
        .replace("*", &sep);

        AttributeCorrelation {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AttributeCorrelation {
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
        let mut output_file = String::new();

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
        if output_file.len() == 0 {
            // output_file not specified and should be based on input file
            let p = path::Path::new(&input_file);
            let mut extension = String::from(".");
            let ext = p.extension().unwrap().to_str().unwrap();
            extension.push_str(ext);
            output_file = input_file.replace(&extension, ".html");
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading vector data...")
        };
        let vector_data = Shapefile::read(&input_file)?;

        // how many numeric attributes are in the table?
        let num_fields = vector_data.attributes.header.num_fields as usize;
        let mut numeric_attributes = 0;
        let mut is_numeric = vec![false; num_fields];
        let mut field_names = vec![];
        for field_num in 0..num_fields {
            field_names.push(vector_data.attributes.fields[field_num].name.clone());
            if vector_data.attributes.is_field_numeric(field_num) {
                numeric_attributes += 1;
                is_numeric[field_num] = true;
            }
        }

        if numeric_attributes < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The input vector file's attribute table does not contain at least two numeric feilds."));
        }

        let mut field_totals = vec![0f64; num_fields];
        let mut field_n = vec![0f64; num_fields];
        let mut field_averages = vec![0f64; num_fields];
        let mut correlation_matrix = vec![vec![-99f64; num_fields]; num_fields];
        if verbose {
            println!("Calculating attribute averages...");
        }

        for record_num in 0..vector_data.num_records {
            let rec = vector_data.attributes.get_record(record_num);
            for field_num in 0..num_fields {
                if is_numeric[field_num] {
                    match rec[field_num] {
                        FieldData::Int(val) => {
                            field_totals[field_num] += val as f64;
                            field_n[field_num] += 1f64;
                        }
                        // FieldData::Int64(val) => {
                        //     field_totals[field_num] += val as f64;
                        //     field_n[field_num] += 1f64;
                        // },
                        FieldData::Real(val) => {
                            field_totals[field_num] += val;
                            field_n[field_num] += 1f64;
                        }
                        _ => {
                            // do nothing
                        }
                    }
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for field_num in 0..num_fields {
            if is_numeric[field_num] {
                field_averages[field_num] = field_totals[field_num] / field_n[field_num];
            }
        }

        if verbose {
            println!("Calculating the correlation matrix:");
        }
        let mut i = 0;
        let nodata = -32768f64;
        for a in 0..num_fields {
            if is_numeric[a] {
                for b in 0..(i + 1) {
                    if is_numeric[b] {
                        if a == b {
                            correlation_matrix[a][b] = 1.0;
                        } else {
                            let mut z1: f64;
                            let mut z2: f64;
                            let mut field1_total_deviation = 0f64;
                            let mut field2_total_deviation = 0f64;
                            let mut total_product_deviations = 0f64;
                            for record_num in 0..vector_data.num_records {
                                let rec = vector_data.attributes.get_record(record_num);
                                z1 = match rec[a] {
                                    FieldData::Int(val) => val as f64,
                                    // FieldData::Int64(val) => val as f64,
                                    FieldData::Real(val) => val,
                                    _ => nodata,
                                };
                                z2 = match rec[b] {
                                    FieldData::Int(val) => val as f64,
                                    // FieldData::Int64(val) => val as f64,
                                    FieldData::Real(val) => val,
                                    _ => nodata,
                                };
                                if z1 != nodata && z2 != nodata {
                                    field1_total_deviation +=
                                        (z1 - field_averages[a]) * (z1 - field_averages[a]);
                                    field2_total_deviation +=
                                        (z2 - field_averages[b]) * (z2 - field_averages[b]);
                                    total_product_deviations +=
                                        (z1 - field_averages[a]) * (z2 - field_averages[b]);
                                }
                            }
                            correlation_matrix[a][b] = total_product_deviations
                                / (field1_total_deviation * field2_total_deviation).sqrt();
                        }
                    }
                }
            }
            i += 1;

            if verbose {
                progress = (100.0_f64 * a as f64 / (num_fields - 1) as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Calculating the correlation matrix ({} of {}): {}%",
                        (a + 1),
                        num_fields,
                        progress
                    );
                    old_progress = progress;
                }
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
            <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
            <title>Attribute Correlation</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Attributes Correlation Report</h1>"#
                .as_bytes(),
        )?;

        // output the names of the input files.
        writer.write_all("<p><strong>Attributes</strong>:</br>".as_bytes())?;
        i = 0;
        for a in 0..num_fields {
            if is_numeric[a] {
                let value = &field_names[a];
                writer.write_all(
                    format!("<strong>Field {}</strong>: {}</br>", i + 1, value).as_bytes(),
                )?;
                i += 1;
            }
        }
        writer.write_all("</p>".as_bytes())?;

        writer.write_all("<br><table align=\"center\">".as_bytes())?;
        writer.write_all("<caption>Pearson correlation matrix</caption>".as_bytes())?;

        let mut out_string = String::from("<tr><th></th>");
        i = 0;
        for a in 0..num_fields {
            if is_numeric[a] {
                out_string.push_str(&format!("<th>Field {}</th>", i + 1));
                i += 1;
            }
        }
        out_string.push_str("</tr>");

        i = 0;
        for a in 0..num_fields {
            if is_numeric[a] {
                out_string.push_str("<tr>");
                out_string.push_str(&format!("<td><strong>Field {}</strong></td>", i + 1));
                for b in 0..num_fields {
                    if is_numeric[b] {
                        let value = correlation_matrix[a][b];
                        if value != -99f64 {
                            let value_str = &format!("{:.*}", 4, value);
                            out_string.push_str(&format!("<td>{}</td>", value_str));
                        } else {
                            out_string.push_str("<td></td>");
                        }
                    }
                }
                out_string.push_str("</tr>");
                i += 1;
            }
        }

        writer.write_all(out_string.as_bytes())?;

        writer.write_all("</table>".as_bytes())?;
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
