/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/06/2017
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
use whitebox_common::structures::Array2D;

/// This tool is used to generate a flow pointer grid using the simple D8 (O'Callaghan and Mark, 1984) algorithm. The
/// user must specify the name (`--dem`) of a digital elevation model (DEM) that has been hydrologically
/// corrected to remove all spurious depressions and flat areas. DEM pre-processing is usually achieved using
/// either the `BreachDepressions` or `FillDepressions` tool. The local drainage direction raster output (`--output`)
/// by this tool serves as a necessary input for several other spatial hydrology and stream network analysis tools
/// in the toolset. Some tools will calculate this flow pointer raster directly from the input DEM.
///
/// By default, D8 flow pointers use the following clockwise, base-2 numeric index convention:
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// | 64 | 128 | 1  |
/// | 32 |  0  | 2  |
/// | 16 |  8  | 4  |
///
/// Notice that grid cells that have no lower neighbours are assigned a flow direction of zero. In a DEM that has been
/// pre-processed to remove all depressions and flat areas, this condition will only occur along the edges of the grid.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// Grid cells possessing the NoData value in the input DEM are assigned the NoData value in the output image.
///
/// # Memory Usage
/// The peak memory usage of this tool is approximately 10 bytes per grid cell.
/// 
/// # Reference
/// O'Callaghan, J. F., & Mark, D. M. (1984). The extraction of drainage networks from digital elevation data.
/// Computer vision, graphics, and image processing, 28(3), 323-344.
///
/// # See Also
/// `DInfPointer`, `FD8Pointer`, `BreachDepressions`, `FillDepressions`
pub struct D8Pointer {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl D8Pointer {
    pub fn new() -> D8Pointer {
        // public constructor
        let name = "D8Pointer".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Calculates a D8 flow pointer raster from an input DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
            name: "Should the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        D8Pointer {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for D8Pointer {
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
        let mut esri_style = false;

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
            } else if vec[0].to_lowercase() == "-esri_pntr"
                || vec[0].to_lowercase() == "--esri_pntr"
                || vec[0].to_lowercase() == "--esri_style"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;
        let nodata = input.configs.nodata;
        let out_nodata = -32768i16;
        let columns = input.configs.columns as isize;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let grid_lengths = [
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                ];
                // let out_vals = match esri_style {
                //     true => [128f64, 1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64],
                //     false => [1f64, 2f64, 4f64, 8f64, 16f64, 32f64, 64f64, 128f64],
                // };
                let out_vals = match esri_style {
                    true => [128i16, 1, 2, 4, 8, 16, 32, 64],
                    false => [1i16, 2, 4, 8, 16, 32, 64, 128],
                };
                let (mut z, mut z_n, mut slope): (f64, f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    // let mut data = vec![out_nodata; columns as usize];
                    let mut data = vec![out_nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            let mut dir = 0;
                            let mut max_slope = f64::MIN;
                            for i in 0..8 {
                                z_n = input[(row + d_y[i], col + d_x[i])];
                                if z_n != nodata {
                                    slope = (z - z_n) / grid_lengths[i];
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i;
                                    }
                                }
                            }
                            if max_slope >= 0f64 {
                                data[col as usize] = out_vals[dir]; //(1 << dir) as f64;
                            } else {
                                data[col as usize] = 0i16; // 0f64;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut output: Array2D<i16> = Array2D::new(rows, columns, out_nodata, out_nodata)?;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let in_configs = input.configs.clone();
        drop(input);

        let mut output_raster = Raster::initialize_using_array2d(&output_file, &in_configs, output);

        let elapsed_time = get_formatted_elapsed_time(start);
        output_raster.configs.nodata = out_nodata as f64;
        output_raster.configs.data_type = DataType::I16;
        output_raster.configs.palette = "qual.plt".to_string();
        output_raster.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output_raster.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output_raster.add_metadata_entry(format!("Input file: {}", input_file));
        if esri_style {
            output_raster.add_metadata_entry("ESRI-style output: true".to_string());
        } else {
            output_raster.add_metadata_entry("ESRI-style output: false".to_string());
        }
        output_raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output_raster.write() {
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
