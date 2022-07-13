/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/07/2017
Last Modified: 18/10/2019
License: MIT

NOTES: Will need to add support for vector polygons eventually.
*/

use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool calculates the centroid, or average location, of raster polygon objects.
/// For vector features, use the `CentroidVector` tool instead.
///
/// # See Also
/// `CentroidVector`
pub struct Centroid {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Centroid {
    pub fn new() -> Centroid {
        // public constructor
        let name = "Centroid".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Calculates the centroid, or average location, of raster polygon objects.".to_string();

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
            name: "Output text?".to_owned(),
            flags: vec!["--text_output".to_owned()],
            description: "Optional text output.".to_owned(),
            parameter_type: ParameterType::Boolean,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=polygons.tif -o=output.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=polygons.tif -o=output.tif --text_output",
            short_exe, name
        )
        .replace("*", &sep);

        Centroid {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Centroid {
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
        let mut text_output = false;

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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
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
            } else if vec[0].to_lowercase() == "-text_output"
                || vec[0].to_lowercase() == "--text_output"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    text_output = true;
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
        let start = Instant::now();

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;

        let min_val = input.configs.minimum.floor() as usize;
        let max_val = input.configs.maximum.ceil() as usize;
        let range = max_val - min_val;

        let mut total_columns = vec![0usize; range + 1];
        let mut total_rows = vec![0usize; range + 1];
        let mut total_n = vec![0usize; range + 1];

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut z: f64;
        let mut a: usize;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z > 0f64 && z != nodata {
                    a = (z - min_val as f64) as usize;
                    total_columns[a] += col as usize;
                    total_rows[a] += row as usize;
                    total_n[a] += 1usize;
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

        let mut col: isize;
        let mut row: isize;
        for a in 0..range + 1 {
            if total_n[a] > 0 {
                col = (total_columns[a] / total_n[a]) as isize;
                row = (total_rows[a] / total_n[a]) as isize;
                output.set_value(row, col, (a + min_val) as f64);
            }
        }

        if text_output {
            let mut col: f64;
            let mut row: f64;
            println!("Patch Centroid\nPatch ID\tColumn\tRow");
            for a in 0..range + 1 {
                if total_n[a] > 0 {
                    col = total_columns[a] as f64 / total_n[a] as f64;
                    row = total_rows[a] as f64 / total_n[a] as f64;
                    println!("{}\t{}\t{}", (a + min_val), col, row);
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
