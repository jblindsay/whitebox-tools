/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 18/03/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool performs an in-place division operation (i.e. `input1 /= input2`). The
/// user must specify the names of two input images (`--input1` and `--input2`) and the tool will
/// divide the `input1` raster by the `input2` values on a pixel-by-pixel basis. Therefore, the
/// `input1` raster is modified by this tool. Note that `input2` may either be an existing raster
/// file or a non-zero constant value. If `input2` is a raster, it must have the same dimensions (rows and
/// columns) as `input1`. Zero-valued pixels in `input2` will be assigned the NoData value in
/// `input1` after this operation.
///
/// The difference between this tool and the `Divide` tool is that `Divide` does not modify either of its
/// two operands, and instead creates a new output raster to save the resultant value into.
///
/// # See Also
/// `Divide`, `InPlaceAdd`, `InPlaceMultiply`, `InPlaceSubtract`
pub struct InPlaceDivide {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl InPlaceDivide {
    /// public constructor
    pub fn new() -> InPlaceDivide {
        let name = "InPlaceDivide".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs an in-place division operation (input1 /= input2).".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Raster File".to_owned(),
            flags: vec!["--input1".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input File Or Constant Value".to_owned(),
            flags: vec!["--input2".to_owned()],
            description: "Input raster file or constant value.".to_owned(),
            parameter_type: ParameterType::ExistingFileOrFloat(ParameterFileType::Raster),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1='in1.tif' --input2='in2.tif'\"
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1='in1.tif' --input2=10.5'",
            short_exe, name
        )
        .replace("*", &sep);

        InPlaceDivide {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for InPlaceDivide {
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
        let mut input1 = String::new();
        let mut input2 = String::new();

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
            if flag_val == "-input1" {
                input1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-input2" {
                input2 = if keyval {
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

        if !input1.contains(&sep) && !input1.contains("/") {
            input1 = format!("{}{}", working_directory, input1);
        }

        let mut input2_constant = f64::NEG_INFINITY;
        let input2_is_constant = match input2.parse::<f64>() {
            Ok(val) => {
                input2_constant = val;
                true
            }
            Err(_) => false,
        };
        if !input2_is_constant {
            if !input2.contains(&sep) && !input2.contains("/") {
                input2 = format!("{}{}", working_directory, input2);
            }
        }

        if verbose {
            println!("Reading data...")
        };
        let mut in1 = Raster::new(&input1, "rw")?;

        let mut start = Instant::now();

        let rows = in1.configs.rows as isize;
        let columns = in1.configs.columns as isize;
        let nodata1 = in1.configs.nodata;
        let (mut z1, mut z2): (f64, f64);

        if input2_is_constant {
            if input2_constant == 0f64 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Illegal division by zero.",
                ));
            }
            for row in 0..rows {
                for col in 0..columns {
                    z1 = in1.get_value(row, col);
                    if z1 != nodata1 {
                        in1.set_value(row, col, z1 / input2_constant);
                    }
                }

                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // !input2_is_constant
            if verbose {
                println!("Reading data...")
            };
            let in2 = Raster::new(&input2, "r")?;

            start = Instant::now();
            let nodata2 = in2.configs.nodata;

            // make sure the input files have the same size
            if in1.configs.rows != in2.configs.rows || in1.configs.columns != in2.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "The input files must have the same number of rows and columns and spatial extent."));
            }

            for row in 0..rows {
                for col in 0..columns {
                    z1 = in1.get_value(row, col);
                    z2 = in2.get_value(row, col);
                    if z1 != nodata1 && z2 != nodata2 && z2 != 0f64 {
                        in1.set_value(row, col, z1 / z2);
                    } else if z1 != nodata1 {
                        in1.set_value(row, col, nodata1);
                    }
                }

                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        in1.update_min_max();
        in1.update_display_min_max();
        let _ = match in1.write() {
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
