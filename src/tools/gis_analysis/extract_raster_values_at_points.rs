/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/06/2018
Last Modified: 17/06/2018
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// Extracts the values of raster(s) at vector point locations.
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--inputs" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i + 1].to_string();
                }
            } else if flag_val == "-points" {
                points_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
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

        let start = Instant::now();

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
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
        let mut x_vals = vec![];
        let mut y_vals = vec![];
        for record_num in 0..num_records {
            let record = points.get_record(record_num);
            y_vals.push(record.points[0].y);
            x_vals.push(record.points[0].x);
        }

        // add the attributes for each raster
        for i in 0..vec.len() {
            if !vec[i].trim().is_empty() {
                let val =
                    AttributeField::new(&format!("VALUE{}", i + 1), FieldDataType::Real, 12u8, 4u8);
                points.attributes.add_field(&val);
            }
        }

        let mut z: f64;
        let mut i = 1;
        for value in vec {
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
                }

                i += 1;
            }
        }
        // drop(attributes);

        // let start = time::now();
        // let rows = input.configs.rows as isize;
        // let columns = input.configs.columns as isize;
        // let nodata = input.configs.nodata;

        // // loop through the raster, locating the min/max
        // let rows_completed = Arc::new(Mutex::new(0..rows));
        // let old_progress = Arc::new(Mutex::new(1));
        // let num_procs = num_cpus::get() as isize;
        // let (tx, rx) = mpsc::channel();
        // for tid in 0..num_procs {
        //     let input = input.clone();
        //     let rows_completed = rows_completed.clone();
        //     let old_progress = old_progress.clone();
        //     let tx = tx.clone();
        //     thread::spawn(move || {
        //         let mut low_z = f64::INFINITY;
        //         let mut low_row = 0isize;
        //         let mut low_col = 0isize;
        //         let mut high_z = f64::NEG_INFINITY;
        //         let mut high_row = 0isize;
        //         let mut high_col = 0isize;
        //         let mut progress: usize;
        //         // let mut old_progress: usize = 1;
        //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //             for col in 0..columns {
        //                 z = input.get_value(row, col);
        //                 if z != nodata {
        //                     if z < low_z {
        //                         low_z = z;
        //                         low_col = col;
        //                         low_row = row;
        //                     }
        //                     if z > high_z {
        //                         high_z = z;
        //                         high_col = col;
        //                         high_row = row;
        //                     }
        //                 }
        //             }
        //             let r = match rows_completed.lock().unwrap().next() {
        //                 Some(val) => val,
        //                 None => 0, // There are no more tiles to interpolate
        //             };
        //             if verbose {
        //                 progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
        //                 let mut p = old_progress.lock().unwrap();
        //                 if progress != *p {
        //                     println!("Progress: {}%", progress);
        //                     *p = progress;
        //                 }
        //             }
        //         }
        //         tx.send((low_z, low_col, low_row, high_z, high_col, high_row))
        //             .unwrap();
        //     });
        // }

        // let mut low_z = f64::INFINITY;
        // let mut low_row = 0isize;
        // let mut low_col = 0isize;
        // let mut high_z = f64::NEG_INFINITY;
        // let mut high_row = 0isize;
        // let mut high_col = 0isize;
        // for _ in 0..num_procs {
        //     let data = rx.recv().unwrap();
        //     if data.0 < low_z {
        //         low_z = data.0;
        //         low_col = data.1;
        //         low_row = data.2;
        //     }
        //     if data.3 > high_z {
        //         high_z = data.3;
        //         high_col = data.4;
        //         high_row = data.5;
        //     }
        // }

        // // add the vector record(s)
        // let mut rec_num = 1i32;
        // if out_type == "lowest" || out_type == "both" {
        //     output.add_point_record(
        //         input.get_x_from_column(low_col),
        //         input.get_y_from_row(low_row),
        //     );
        //     output
        //         .attributes
        //         .add_record(vec![FieldData::Int(rec_num), FieldData::Real(low_z)], false);
        //     rec_num += 1i32;
        // }

        // if out_type == "highest" || out_type == "both" {
        //     output.add_point_record(
        //         input.get_x_from_column(high_col),
        //         input.get_y_from_row(high_row),
        //     );
        //     output.attributes.add_record(
        //         vec![FieldData::Int(rec_num), FieldData::Real(high_z)],
        //         false,
        //     );
        // }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        let _ = match points.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
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
