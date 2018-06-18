/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/06/2018
Last Modified: 12/06/2018
License: MIT
*/

use num_cpus;
use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use time;
use tools::*;
use vector::*;

/// Locates the lowest and/or highest valued cells in a raster.
pub struct FindLowestOrHighestPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FindLowestOrHighestPoints {
    pub fn new() -> FindLowestOrHighestPoints {
        // public constructor
        let name = "FindLowestOrHighestPoints".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Locates the lowest and/or highest valued cells in a raster.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Points File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector points file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Type".to_owned(),
            flags: vec!["--out_type".to_owned()],
            description: "Output type; one of 'area' (default) and 'volume'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "lowest".to_owned(),
                "highest".to_owned(),
                "both".to_owned(),
            ]),
            default_value: Some("lowest".to_owned()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=out.shp --out_type=highest", short_exe, name).replace("*", &sep);

        FindLowestOrHighestPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FindLowestOrHighestPoints {
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
        let mut out_type = "lowest".to_owned();

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
            } else if flag_val == "-out_type" {
                out_type = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i + 1].to_lowercase()
                };
                if out_type.contains("low") {
                    out_type == "lowest".to_owned();
                }
                if out_type.contains("hi") {
                    out_type == "highest".to_owned();
                }
                if out_type.contains("b") {
                    out_type == "both".to_owned();
                }
            }
        }

        // let mut progress: usize;
        // let mut old_progress: usize = 1;

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut output = Shapefile::new(&output_file, ShapeType::Point)?;

        // add the attributes
        let fid = AttributeField::new("FID", FieldDataType::Int, 2u8, 0u8);
        let val = AttributeField::new("Value", FieldDataType::Real, 12u8, 4u8);
        output.attributes.add_field(&fid);
        output.attributes.add_field(&val);

        // loop through the raster, locating the min/max
        let rows_completed = Arc::new(Mutex::new(0..rows));
        let old_progress = Arc::new(Mutex::new(1));
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let rows_completed = rows_completed.clone();
            let old_progress = old_progress.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut low_z = f64::INFINITY;
                let mut low_row = 0isize;
                let mut low_col = 0isize;
                let mut high_z = f64::NEG_INFINITY;
                let mut high_row = 0isize;
                let mut high_col = 0isize;
                let mut progress: usize;
                // let mut old_progress: usize = 1;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            if z < low_z {
                                low_z = z;
                                low_col = col;
                                low_row = row;
                            }
                            if z > high_z {
                                high_z = z;
                                high_col = col;
                                high_row = row;
                            }
                        }
                    }
                    let r = match rows_completed.lock().unwrap().next() {
                        Some(val) => val,
                        None => 0, // There are no more tiles to interpolate
                    };
                    if verbose {
                        progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                        let mut p = old_progress.lock().unwrap();
                        if progress != *p {
                            println!("Progress: {}%", progress);
                            *p = progress;
                        }
                    }
                }
                tx.send((low_z, low_col, low_row, high_z, high_col, high_row))
                    .unwrap();
            });
        }

        let mut low_z = f64::INFINITY;
        let mut low_row = 0isize;
        let mut low_col = 0isize;
        let mut high_z = f64::NEG_INFINITY;
        let mut high_row = 0isize;
        let mut high_col = 0isize;
        for _ in 0..num_procs {
            let data = rx.recv().unwrap();
            if data.0 < low_z {
                low_z = data.0;
                low_col = data.1;
                low_row = data.2;
            }
            if data.3 > high_z {
                high_z = data.3;
                high_col = data.4;
                high_row = data.5;
            }
        }

        // add the vector record(s)
        let mut rec_num = 1i32;
        if out_type == "lowest" || out_type == "both" {
            output.add_point_record(
                input.get_x_from_column(low_col),
                input.get_y_from_row(low_row),
            );
            output
                .attributes
                .add_record(vec![FieldData::Int(rec_num), FieldData::Real(low_z)], false);
            rec_num += 1i32;
        }

        if out_type == "highest" || out_type == "both" {
            output.add_point_record(
                input.get_x_from_column(high_col),
                input.get_y_from_row(high_row),
            );
            output.attributes.add_record(
                vec![FieldData::Int(rec_num), FieldData::Real(high_z)],
                false,
            );
        }

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
