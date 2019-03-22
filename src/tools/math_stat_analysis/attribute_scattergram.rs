/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/04/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::rendering::html::*;
use crate::rendering::Scattergram;
use crate::tools::*;
use crate::vector::{FieldData, Shapefile};
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to create a [scattergram](https://en.wikipedia.org/wiki/Scatter_plot) for 
/// two numerical fields (`--fieldx` and `--fieldy`) contained within an input vector's attribute 
/// table (`--input`). The user must specify the name of an input shapefile and the name of two of 
/// the fields contained it the associated attribute table. The tool output (`--output`) is an 
/// HTML formated report containing a graphical scattergram plot.
/// 
/// # See Also
/// `AttributeHistogram` 
pub struct AttributeScattergram {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AttributeScattergram {
    pub fn new() -> AttributeScattergram {
        // public constructor
        let name = "AttributeScattergram".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Creates a scattergram for two field values of a vector's attribute table.".to_string();

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
            name: "Field Name X".to_owned(),
            flags: vec!["--fieldx".to_owned()],
            description: "Input field name in attribute table for the x-axis.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name Y".to_owned(),
            flags: vec!["--fieldy".to_owned()],
            description: "Input field name in attribute table for the y-axis.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
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

        parameters.push(ToolParameter {
            name: "Draw the trendline?".to_owned(),
            flags: vec!["--trendline".to_owned()],
            description: "Draw the trendline.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lakes.shp --fieldx=HEIGHT --fieldy=area -o=outfile.html --trendline",
                            short_exe, name).replace("*", &sep);

        AttributeScattergram {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AttributeScattergram {
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
        let mut field_name_x = String::new();
        let mut field_name_y = String::new();
        let mut output_file = String::new();
        let mut trendline = false;

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
            } else if flag_val == "-fieldx" {
                field_name_x = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-fieldy" {
                field_name_y = if keyval {
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
            } else if flag_val == "-trendline" {
                trendline = true;
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

        // What is the index of the x-variable field to be analyzed?
        let field_index_x = match vector_data.attributes.get_field_num(&field_name_x) {
            Some(i) => i,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified x-variable field name does not exist in input shapefile.",
                ))
            }
        };

        // Is the field numeric?
        if !vector_data.attributes.is_field_numeric(field_index_x) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The specified x-variable attribute field is non-numeric.",
            ));
        }

        // What is the index of the y-variable field to be analyzed?
        let field_index_y = match vector_data.attributes.get_field_num(&field_name_y) {
            Some(i) => i,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The specified y-variable field name does not exist in input shapefile.",
                ))
            }
        };

        // Is the field numeric?
        if !vector_data.attributes.is_field_numeric(field_index_y) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The specified y-variable attribute field is non-numeric.",
            ));
        }

        let mut xdata = vec![];
        let mut ydata = vec![];
        let mut series_xdata = vec![];
        let mut series_ydata = vec![];
        let mut series_names = vec![];

        // Find the min and max values of the field
        let mut x: f64;
        let mut y: f64;
        let nodata = -32768f64;
        for record_num in 0..vector_data.num_records {
            x = match vector_data.attributes.get_value(record_num, &field_name_x) {
                FieldData::Int(val) => val as f64,
                // FieldData::Int64(val) => {
                //     val as f64
                // },
                FieldData::Real(val) => val,
                _ => {
                    nodata // likely a null field
                }
            };

            y = match vector_data.attributes.get_value(record_num, &field_name_y) {
                FieldData::Int(val) => val as f64,
                // FieldData::Int64(val) => {
                //     val as f64
                // },
                FieldData::Real(val) => val,
                _ => {
                    nodata // likely a null field
                }
            };

            if x != nodata && y != nodata {
                series_xdata.push(x);
                series_ydata.push(y);
            }

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Reading data: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        xdata.push(series_xdata.clone());
        ydata.push(series_ydata.clone());
        series_names.push(format!("Series {} - {}", field_name_x, field_name_y));

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
            <title>Scattergram Analysis</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"</head>
        <body>
            <h1>Scatergram Analysis</h1>"#
                .as_bytes(),
        )?;

        writer.write_all(
            &format!("<p><strong>Input</strong>: {}</p>", input_file.clone()).as_bytes(),
        )?;
        writer.write_all(
            &format!(
                "<p><strong>X Field Name</strong>: {}</p>",
                field_name_x.clone()
            )
            .as_bytes(),
        )?;
        writer.write_all(
            &format!(
                "<p><strong>Y Field Name</strong>: {}</p>",
                field_name_y.clone()
            )
            .as_bytes(),
        )?;

        let graph = Scattergram {
            parent_id: "graph".to_string(),
            data_x: xdata.clone(),
            data_y: ydata.clone(),
            series_labels: series_names.clone(),
            x_axis_label: field_name_x.to_string(),
            y_axis_label: field_name_y.to_string(),
            width: 700f64,
            height: 500f64,
            draw_trendline: trendline,
            draw_gridlines: true,
            draw_legend: false,
            draw_grey_background: false,
        };

        writer.write_all(
            &format!("<div id='graph' align=\"center\">{}</div>", graph.get_svg()).as_bytes(),
        )?;

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
