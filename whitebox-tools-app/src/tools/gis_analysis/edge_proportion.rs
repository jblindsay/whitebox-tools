/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 13/07/2017
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool will measure the edge proportion, i.e. the proportion of grid cells in a patch that are located along the
/// patch's boundary, for an input raster image (`--input`). Edge proportion is an indicator of polygon shape complexity
/// and elongation. The user must specify the name of the output raster file (`--output`), which will be raster layer
/// containing the input features assigned the edge proportion. The user may also optionally choose to output text data
/// for easy input to a spreadsheet or database.
///
/// Objects in the input raster are designated by their unique identifiers. Identifier values must
/// be positive, non-zero whole numbers.
///
/// # See Also
/// `ShapeComplexityIndexRaster`, `LinearityIndex`, `ElongationRatio`
pub struct EdgeProportion {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EdgeProportion {
    pub fn new() -> EdgeProportion {
        // public constructor
        let name = "EdgeProportion".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description =
            "Calculate the proportion of cells in a raster polygon that are edge cells."
                .to_string();

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
            name: "Output a text report?".to_owned(),
            flags: vec!["--output_text".to_owned()],
            description: "flag indicating whether a text report should also be output.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --output_text",
            short_exe, name
        )
        .replace("*", &sep);

        EdgeProportion {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EdgeProportion {
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
        let mut output_text = false;

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
            } else if vec[0].to_lowercase() == "-output_text"
                || vec[0].to_lowercase() == "--output_text"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    output_text = true;
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
            println!("Reading input data...");
        }
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let max_val = input.configs.maximum.floor() as usize;

        let start = Instant::now();

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut num_cells = vec![0usize; max_val + 1];
                let mut num_edge_cells = vec![0usize; max_val + 1];
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut z: f64;
                let mut zn: f64;
                let mut is_edge: bool;
                let mut bin: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z > 0f64 && z != nodata {
                            bin = z.floor() as usize;
                            num_cells[bin] += 1;
                            is_edge = false;
                            for n in 0..8 {
                                zn = input[(row + dy[n], col + dx[n])];
                                if zn != z {
                                    is_edge = true;
                                    break;
                                }
                            }
                            if is_edge {
                                num_edge_cells[bin] += 1;
                            }
                        }
                    }
                }
                tx.send((num_cells, num_edge_cells)).unwrap();
            });
        }

        let mut num_cells = vec![0usize; max_val + 1];
        let mut num_edge_cells = vec![0usize; max_val + 1];
        for tid in 0..num_procs {
            let (vec1, vec2) = rx.recv().expect("Error receiving data from thread.");
            for bin in 0..max_val + 1 {
                num_cells[bin] += vec1[bin];
                num_edge_cells[bin] += vec2[bin];
            }
            if verbose {
                progress = (100.0_f64 * tid as f64 / (num_procs - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut edge_props = vec![nodata; max_val + 1];
        for bin in 0..max_val + 1 {
            if num_cells[bin] > 0 {
                edge_props[bin] = num_edge_cells[bin] as f64 / num_cells[bin] as f64;
            }
        }
        let edge_props = Arc::new(edge_props);

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let edge_props = edge_props.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut bin: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z > 0f64 && z != nodata {
                            bin = z.floor() as usize;
                            data[col as usize] = edge_props[bin];
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;

        for r in 0..rows {
            let (row, data) = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 2): {}%", progress);
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

        if output_text {
            println!("Edge Proportion\nPatch ID\tValue");
            for bin in 0..max_val + 1 {
                if edge_props[bin] > 0f64 && edge_props[bin] != nodata {
                    println!("{}\t{}", bin, edge_props[bin]);
                }
            }
        }

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
