/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 18/10/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool re-categorizes data in a raster image by grouping cells that form  
/// discrete, contiguous areas into unique categories. Essentially this will produce
/// a patch map from an input categorical raster, assigning each feature unique
/// identifiers. The input raster should either be
/// Boolean (1's and 0's) or categorical. The input raster could be created using
/// the `Reclass` tool or one of the comparison operators (`GreaterThan`, `LessThan`,
/// `EqualTo`, `NotEqualTo`). Use the *treat zeros as background cells* options
/// (`--zero_back`) if you would like to only assigned contiguous groups of non-zero
/// values in the raster unique identifiers. Additionally, inter-cell connectivity
/// can optionally include diagonally neighbouring cells if the `--diag` flag is
/// specified.
///
/// # See Also
/// `Reclass`, `GreaterThan`, `LessThan`, `EqualTo`, `NotEqualTo`
pub struct Clump {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Clump {
    pub fn new() -> Clump {
        // public constructor
        let name = "Clump".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Groups cells that form discrete areas, assigning them unique identifiers.".to_string();

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

        parameters.push(ToolParameter {
            name: "Include diagonal connections?".to_owned(),
            flags: vec!["--diag".to_owned()],
            description: "Flag indicating whether diagonal connections should be considered."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Treat zero values as background?".to_owned(),
            flags: vec!["--zero_back".to_owned()],
            description: "Flag indicating whether zero values should be treated as a background."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --diag",
            short_exe, name
        )
        .replace("*", &sep);

        Clump {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Clump {
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
        let mut diag = false;
        let mut zero_back = false;

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
            } else if flag_val == "-diag" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    diag = true;
                }
            } else if flag_val == "-zero_back" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    zero_back = true;
                }
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

        let start = Instant::now();

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let out_nodata = -999f64;
        output.reinitialize_values(out_nodata);
        output.configs.nodata = out_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.configs.data_type = DataType::I32;

        let mut dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let mut dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut num_neighbours = 8;
        if !diag {
            dx = [0, 1, 0, -1, 0, 0, 0, 0];
            dy = [-1, 0, 1, 0, 0, 0, 0, 0];
            num_neighbours = 4;
        }
        let mut back_val = f64::NEG_INFINITY;
        if zero_back {
            back_val = 0f64;
        }
        let (mut zin, mut zout, mut zn): (f64, f64, f64);
        let (mut r, mut c): (isize, isize);
        let mut fid = 0f64;
        let mut num_solved_cells = 0;
        let num_cells = rows * columns;
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut count: usize; // this is just used to update the progress after every 1000 cells solved.
        for row in 0..rows {
            for col in 0..columns {
                zin = input[(row, col)];
                zout = output[(row, col)];
                if zin != nodata && zin != back_val && zout == out_nodata {
                    fid += 1f64;
                    output[(row, col)] = fid;
                    num_solved_cells += 1;
                    stack.push((row, col));
                    count = 0;
                    while !stack.is_empty() {
                        let cell = stack.pop().unwrap();
                        r = cell.0;
                        c = cell.1;
                        count += 1;
                        if count == 1000 {
                            count = 0;
                            if verbose {
                                progress = (100.0_f64 * num_solved_cells as f64
                                    / (num_cells - 1) as f64)
                                    as usize;
                                if progress != old_progress {
                                    println!("Performing analysis: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }
                        for i in 0..num_neighbours {
                            zn = input[(r + dy[i], c + dx[i])];
                            zout = output[(r + dy[i], c + dx[i])];
                            if zn == zin && zout == out_nodata {
                                output[(r + dy[i], c + dx[i])] = fid;
                                num_solved_cells += 1;
                                stack.push((r + dy[i], c + dx[i]));
                            }
                        }
                    }
                } else if zin == nodata {
                    num_solved_cells += 1;
                } else if zin == back_val {
                    num_solved_cells += 1;
                    output[(row, col)] = back_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "qual.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Diagonal connectivity: {}", diag));
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
