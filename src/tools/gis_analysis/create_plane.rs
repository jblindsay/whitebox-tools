/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 11/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::thread;

/// This tool can be used to create a new raster with values that are determined by the equation of a simple plane. The user
/// must specify the name of a base raster (`--base`) from which the output raster coordinate and dimensional information
/// will be taken. In addition the user must specify the values of the planar slope gradient (S; `--gradient`; `--aspect`)
/// in degrees, the planar slope direction or aspect (A; 0 to 360 degrees), and an constant value (k; `--constant`). The
/// equation of the plane is as follows:
///
/// > Z = tan(S) × sin(A - 180) × X + tan(S) × cos(A - 180) × Y + k
///
/// where X and Y are the X and Y coordinates of each grid cell in the grid. Notice that A is the direction,
/// or azimuth, that the plane is facing
pub struct CreatePlane {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl CreatePlane {
    pub fn new() -> CreatePlane {
        // public constructor
        let name = "CreatePlane".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Creates a raster image based on the equation for a simple plane.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Base File".to_owned(),
            flags: vec!["--base".to_owned()],
            description: "Input base raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
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
            name: "Gradient".to_owned(),
            flags: vec!["--gradient".to_owned()],
            description: "Slope gradient in degrees (-85.0 to 85.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("15.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Aspect".to_owned(),
            flags: vec!["--aspect".to_owned()],
            description: "Aspect (direction) in degrees clockwise from north (0.0-360.0)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("90.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Constant".to_owned(),
            flags: vec!["--constant".to_owned()],
            description: "Constant value.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=base.tif -o=NewRaster.tif --gradient=15.0 --aspect=315.0", short_exe, name).replace("*", &sep);

        CreatePlane {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for CreatePlane {
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
        let mut slope = 15.0;
        let mut aspect = 90.0;
        let mut constant_val = 0.0;

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
            if flag_val == "-base" {
                if keyval {
                    base_file = vec[1].to_string();
                } else {
                    base_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-slope" {
                if keyval {
                    slope = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    slope = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-aspect" {
                if keyval {
                    aspect = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    aspect = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-constant" {
                if keyval {
                    constant_val = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    constant_val = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !base_file.contains(&sep) && !base_file.contains("/") {
            base_file = format!("{}{}", working_directory, base_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let base = Raster::new(&base_file, "r")?;

        let start = Instant::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        if slope < -85.0 {
            slope = -85.0;
        }
        if slope > 85.0 {
            slope = 85.0;
        }
        if aspect > 360.0 {
            let mut flag = false;
            while !flag {
                aspect -= 360.0;
                if aspect <= 360.0 {
                    flag = true;
                }
            }
        }
        if aspect > 180.0 {
            aspect -= 180.0;
        } else {
            aspect += 180.0;
        }
        slope = slope.to_radians().tan();
        aspect = aspect.to_radians();

        let rows = base.configs.rows as isize;
        let columns = base.configs.columns as isize;
        let north = base.configs.north;
        let south = base.configs.south;
        let east = base.configs.east;
        let west = base.configs.west;
        let xrange = east - west;
        let yrange = north - south;
        let nodata = base.configs.nodata;

        let mut output = Raster::initialize_using_file(&output_file, &base);
        output.configs.data_type = DataType::F32;
        output.configs.palette = "spectrum.plt".to_string();
        if output.configs.data_type != DataType::F32 && output.configs.data_type != DataType::F64 {
            output.configs.data_type = DataType::F32;
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut x, mut y): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        x = west + xrange * (col as f64 / (columns as f64 - 1f64));
                        y = north - yrange * (row as f64 / (rows as f64 - 1f64));
                        data[col as usize] =
                            slope * aspect.sin() * x + slope * aspect.cos() * y + constant_val;
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Base raster file: {}", base_file));
        output.add_metadata_entry(format!("Slope: {}", slope));
        output.add_metadata_entry(format!("Aspect: {}", aspect));
        output.add_metadata_entry(format!("Constant: {}", constant_val));
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
