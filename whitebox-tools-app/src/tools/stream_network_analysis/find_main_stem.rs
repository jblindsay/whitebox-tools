/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/06/2017
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to identify the main channel in a stream network. The user must specify the names of
/// a D8 pointer (flow direction) raster (`--d8_pntr`), and a streams raster (`--streams`). The pointer raster
/// is used to traverse the stream network and should only be created using the `D8Pointer`. By default, the pointer
/// raster is assumed to use the clockwise indexing method used by WhiteboxTools:
///
/// | .  |  .  |  . |
/// |:--:|:---:|:--:|
/// | 64 | 128 | 1  |
/// | 32 |  0  | 2  |
/// | 16 |  8  | 4  |
///
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// The streams raster should have been created using one of the DEM-based stream mapping methods, i.e. contributing
/// area thresholding. Stream grid cells are designated in the streams image as all positive, non-zero values. All
/// non-stream cells will be assigned the NoData value in the output image, unless the `--zero_background` parameter
/// is specified.
///
/// The algorithm operates by traversing each stream and identifying the longest stream-path draining to each outlet.
/// When a confluence is encountered, the traverse follows the branch with the larger distance-to-head.
///
/// # See Also
/// `D8Pointer`
pub struct FindMainStem {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FindMainStem {
    pub fn new() -> FindMainStem {
        // public constructor
        let name = "FindMainStem".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Finds the main stem, based on stream lengths, of each stream network.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
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
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Should a background value of zero be used?".to_owned(),
            flags: vec!["--zero_background".to_owned()],
            description: "Flag indicating whether a background value of zero should be used."
                .to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streams.tif -o=output.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streams.tif -o=output.tif --esri_pntr --zero_background", short_exe, name).replace("*", &sep);

        FindMainStem {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FindMainStem {
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
        let mut d8_file = String::new();
        let mut streams_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut background_val = f64::NEG_INFINITY;

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
            if vec[0].to_lowercase() == "-d8_pntr" || vec[0].to_lowercase() == "--d8_pntr" {
                if keyval {
                    d8_file = vec[1].to_string();
                } else {
                    d8_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-streams" || vec[0].to_lowercase() == "--streams" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i + 1].to_string();
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
            } else if vec[0].to_lowercase() == "-zero_background"
                || vec[0].to_lowercase() == "--zero_background"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    background_val = 0f64;
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

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading pointer data...")
        };
        let pntr = Raster::new(&d8_file, "r")?;
        let pntr_nodata = pntr.configs.nodata;
        if verbose {
            println!("Reading streams data...")
        };
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let num_cells = pntr.num_cells();
        let nodata = streams.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }
        let cell_size_x = streams.configs.resolution_x;
        let cell_size_y = streams.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        let mut stack = Vec::with_capacity((rows * columns) as usize);

        // calculate the number of inflowing cells
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let mut trib_length: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
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
        let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
        if esri_style {
            inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
        }
        let mut num_solved_cells = 0;
        let mut count: i8;
        let mut current_id = 1f64;
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 {
                    count = 0i8;
                    for i in 0..8 {
                        if streams[(row + d_y[i], col + d_x[i])] > 0.0
                            && pntr[(row + d_y[i], col + d_x[i])] == inflowing_vals[i]
                        {
                            count += 1;
                        }
                    }
                    num_inflowing[(row, col)] = count;
                    if count == 0 {
                        // It's a headwater; add it to the stack
                        stack.push((row, col));
                        output[(row, col)] = current_id;
                        current_id += 1f64;
                        trib_length[(row, col)] = 0f64;
                    }
                } else {
                    if pntr[(row, col)] != pntr_nodata {
                        output[(row, col)] = background_val;
                    } else {
                        output[(row, col)] = nodata;
                    }
                    num_solved_cells += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // There are current_id channel heads
        let mut trib_is_main = vec![false; current_id as usize + 1];

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [999usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 0usize;
            pntr_matches[2] = 1usize;
            pntr_matches[4] = 2usize;
            pntr_matches[8] = 3usize;
            pntr_matches[16] = 4usize;
            pntr_matches[32] = 5usize;
            pntr_matches[64] = 6usize;
            pntr_matches[128] = 7usize;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 1usize;
            pntr_matches[2] = 2usize;
            pntr_matches[4] = 3usize;
            pntr_matches[8] = 4usize;
            pntr_matches[16] = 5usize;
            pntr_matches[32] = 6usize;
            pntr_matches[64] = 7usize;
            pntr_matches[128] = 0usize;
        }

        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let mut dir: usize;
        let mut length: f64;
        let mut val: f64;
        let mut c: usize;
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;

            val = output[(row, col)];

            // find the downstream cell
            dir = pntr[(row, col)] as usize;
            if dir > 0 {
                if dir > 128 || pntr_matches[dir] == 999 {
                    return Err(Error::new(ErrorKind::InvalidInput,
                        "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                }

                c = pntr_matches[dir];
                row_n = row + d_y[c];
                col_n = col + d_x[c];

                length = trib_length[(row, col)] + grid_lengths[c];
                if trib_length[(row_n, col_n)] < length || trib_length[(row_n, col_n)] == nodata {
                    trib_length[(row_n, col_n)] = length;
                    output[(row_n, col_n)] = val;
                }

                num_inflowing.decrement(row_n, col_n, 1);
                if num_inflowing[(row_n, col_n)] == 0 {
                    stack.push((row_n, col_n));
                }
            } else {
                trib_is_main[val as usize] = true;
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 && streams[(row, col)] != nodata {
                    val = output[(row, col)];
                    if trib_is_main[val as usize] {
                        output[(row, col)] = streams[(row, col)];
                    } else {
                        output[(row, col)] = background_val;
                    }
                } else {
                    output[(row, col)] = background_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
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
        output.add_metadata_entry(format!("Input D8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
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
