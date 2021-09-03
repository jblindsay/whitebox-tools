/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/07/2017
Last Modified: 13/10/2018
License: MIT

NOTE: This algorithm can't easily be parallelized because the output raster must be read
and written to during the same loop.
*/

use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This image processing tool can be used to thicken single-cell wide lines within a raster file along diagonal
/// sections of the lines. Because of the limitation of the raster data format, single-cell wide raster lines can
/// be traversed along diaganol sections without passing through a line grid cell. This causes problems for various
/// raster analysis functions for which lines are intended to be barriers. This tool will thicken raster lines,
/// such that it is impossible to cross a line without passing through a line grid cell. While this can also be
/// achieved using a maximum filter, unlike the filter approach, this tool will result in the smallest possible
/// thickening to achieve the desired result.
///
/// All non-zero, positive values are considered to be foreground pixels while all zero valued cells or NoData cells
/// are considered background pixels.
///
/// Note: Unlike other filter-based operations in *WhiteboxTools*, this algorithm can't easily be parallelized because
/// the output raster must be read and written to during the same loop.
///
/// # See Also
/// `LineThinning`
pub struct ThickenRasterLine {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ThickenRasterLine {
    pub fn new() -> ThickenRasterLine {
        // public constructor
        let name = "ThickenRasterLine".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Thickens single-cell wide lines within a raster image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --input=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        ThickenRasterLine {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ThickenRasterLine {
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
            if vec[0].to_lowercase() == "-i"
                || vec[0].to_lowercase() == "--input"
                || vec[0].to_lowercase() == "--dem"
            {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        let input = Raster::new(&input_file, "r")?;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        println!("Initializing the output raster...");
        match output.set_data_from_raster(&input) {
            Ok(_) => (), // do nothings
            Err(err) => return Err(err),
        }

        let n1x = [0, 1, 0, -1];
        let n1y = [-1, 0, 1, 0];
        let n2x = [1, 1, -1, -1];
        let n2y = [-1, 1, 1, -1];
        let n3x = [1, 0, -1, 0];
        let n3y = [0, 1, 0, -1];
        let mut z: f64;
        let (mut zn1, mut zn2, mut zn3): (f64, f64, f64);
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z == nodata || z == 0.0 {
                    for i in 0..4 {
                        zn1 = output[(row + n1y[i], col + n1x[i])];
                        zn2 = output[(row + n2y[i], col + n2x[i])];
                        zn3 = output[(row + n3y[i], col + n3x[i])];
                        if (zn1 > 0.0 && zn3 > 0.0) && (zn2 == nodata || zn2 == 0.0) {
                            output[(row, col)] = zn1;
                            break;
                        }
                    }
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
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
