/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 13/10/2018
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool identifies the stack position (index) of the maximum value within a raster stack on a cell-by-cell
/// basis. For example, if five raster images (`--inputs`) are input to the tool, the output raster (`--output`)
/// would show which of the five input rasters contained the highest value for each grid cell. The index value in
/// the output raster is the zero-order number of the raster stack, i.e. if the highest value in the stack is
/// contained in the first image, the output value would be 0; if the highest stack value were the second image,
/// the output value would be 1, and so on. If any of the cell values within the stack is NoData, the output raster
/// will contain the NoData value for the corresponding grid cell. The index value is related to the order of the
/// input images.
///
/// # Warning
/// Each of the input rasters must have the same spatial extent and number of rows and columns.
///
/// # See Also
/// `LowestPosition`, `PickFromList`
pub struct HighestPosition {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HighestPosition {
    pub fn new() -> HighestPosition {
        // public constructor
        let name = "HighestPosition".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Identifies the stack position of the maximum value within a raster stack on a cell-by-cell basis.".to_string();

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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' -o=output.tif", short_exe, name).replace("*", &sep);

        HighestPosition {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HighestPosition {
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
        let mut input_files = String::new();
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--inputs" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i + 1].to_string();
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

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        let start = Instant::now();

        // We need to initialize output and high_val here, but in reality this can't be done
        // until we know the size of rows and columns, which occurs during the first loop.
        let mut output: Raster = Raster::new(&output_file, "w")?;
        let mut high_val: Array2D<f64> = Array2D::new(0, 0, f64::MIN, f64::MIN)?; // use f64::MIN as the nodata value
        let mut rows = 0isize;
        let mut columns = 0isize;
        let mut in_nodata: f64;
        let mut z: f64;
        let mut read_first_file = false;
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
                in_nodata = input.configs.nodata;
                if !read_first_file {
                    read_first_file = true;
                    rows = input.configs.rows as isize;
                    columns = input.configs.columns as isize;

                    // initialize the output file and high_val
                    output = Raster::initialize_using_file(&output_file, &input);
                    high_val = Array2D::new(rows, columns, f64::MIN, in_nodata)?;
                }
                // check to ensure that all inputs have the same rows and columns
                if input.configs.rows as isize != rows || input.configs.columns as isize != columns
                {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
                }

                for row in 0..rows {
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != in_nodata {
                            if z > high_val[(row, col)] {
                                high_val[(row, col)] = z;
                                output[(row, col)] = i as f64;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress (loop {} of {}): {}%", i, num_files, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

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
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
