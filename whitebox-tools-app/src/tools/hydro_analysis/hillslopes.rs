/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 16/07/2017
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

/// This tool will identify the hillslopes associated with a user-specified stream network. Hillslopes
/// include the catchment areas draining to the left and right sides of each stream link in the network as well
/// as the catchment areas draining to all channel heads. `Hillslopes` are conceptually similar to `Subbasins`,
/// except that sub-basins do not distinguish between the right-bank and left-bank catchment areas of stream links.
/// The `Subbasins` tool simply assigns a unique identifier to each stream link in a stream network. Each hillslope
/// output by this tool is assigned a unique, positive identifier  value. All grid cells in the output raster that
/// coincide with a stream cell are assigned an idenifiter of zero, i.e. stream cells do not belong to any hillslope.
///
/// The user must specify the name of a flow pointer
/// (flow direction) raster (`--d8_pntr`), a streams raster (`--streams`), and the output raster (`--output`).
/// The flow pointer and streams rasters should be generated using the `D8Pointer` algorithm. This will require
/// a depressionless DEM, processed using either the `BreachDepressions` or `FillDepressions` tool.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// NoData values in the input flow pointer raster are assigned NoData values in the output image.
///
/// # See Also
/// `StreamLinkIdentifier`, `Watershed`, `Subbasins`, `D8Pointer`, `BreachDepressions`, `FillDepressions`
pub struct Hillslopes {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Hillslopes {
    pub fn new() -> Hillslopes {
        // public constructor
        let name = "Hillslopes".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Identifies the individual hillslopes draining to each link in a stream network."
                .to_string();

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr='d8pntr.tif' --streams='streams.tif' -o='output.tif'", short_exe, name).replace("*", &sep);

        Hillslopes {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Hillslopes {
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
        let mut d8_file = String::new();
        let mut streams_file = String::new();
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
            println!("Reading data...")
        };

        let pntr = Raster::new(&d8_file, "r")?;
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let num_cells = pntr.num_cells();
        let nodata = streams.configs.nodata;
        let pntr_nodata = pntr.configs.nodata;

        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        // First assign each stream link a unique identifier
        let mut pourpts: Array2D<f64> = Array2D::new(rows, columns, nodata, nodata)?;
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut heads = vec![];

        // Calculate the number of inflowing cells
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
        if esri_style {
            inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
        }
        let mut num_solved_cells = 0;
        let mut count: i8;
        let mut current_id = 1f64;
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 && streams[(row, col)] != nodata {
                    count = 0i8;
                    for i in 0..8 {
                        if streams[(row + dy[i], col + dx[i])] > 0.0
                            && pntr[(row + dy[i], col + dx[i])] == inflowing_vals[i]
                        {
                            count += 1;
                        }
                    }
                    num_inflowing[(row, col)] = count;
                    if count == 0 {
                        // It's a headwater; add it to the stack
                        stack.push((row, col));
                        heads.push((row, col));
                        pourpts[(row, col)] = current_id;
                        current_id += 1f64;
                    }
                } else {
                    if pntr[(row, col)] != pntr_nodata {
                        pourpts[(row, col)] = 0.0;
                    } else {
                        pourpts[(row, col)] = nodata;
                    }
                    num_solved_cells += 1;
                }
            }
            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Processing streams: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [999usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in dx and dy.
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
            // onto the cell offsets in dx and dy.
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
        let mut val: f64;
        let mut c: usize;
        while !stack.is_empty() {
            let cell = stack.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;

            val = pourpts[(row, col)];

            // find the downstream cell
            dir = pntr[(row, col)] as usize;
            if dir > 0 {
                if dir > 128 || pntr_matches[dir] == 999 {
                    return Err(Error::new(ErrorKind::InvalidInput,
                        "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                }
                c = pntr_matches[dir];
                row_n = row + dy[c];
                col_n = col + dx[c];
                if num_inflowing[(row_n, col_n)] > 1 {
                    current_id += 1f64;
                    pourpts[(row_n, col_n)] = current_id;
                } else if pourpts[(row_n, col_n)] == nodata {
                    pourpts[(row_n, col_n)] = val;
                }

                num_inflowing.decrement(row_n, col_n, 1);
                if num_inflowing[(row_n, col_n)] == 0 {
                    stack.push((row_n, col_n));
                }
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Processing streams: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Assign a new unique id to each channel head
        while !heads.is_empty() {
            let cell = heads.pop().expect("Error during pop operation.");
            row = cell.0;
            col = cell.1;
            current_id += 1f64;
            pourpts[(row, col)] = current_id;
        }

        // Now perform the watershedding operation
        let mut output = Raster::initialize_using_file(&output_file, &streams);
        output.configs.data_type = DataType::F32;
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        let low_value = f64::MIN;
        output.reinitialize_values(low_value);

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                if pntr[(row, col)] == pntr_nodata {
                    output[(row, col)] = nodata;
                }
                z = pourpts[(row, col)];
                if z != nodata && z > 0.0 {
                    output[(row, col)] = z;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Watershedding (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut flag: bool;
        let (mut x, mut y): (isize, isize);
        let mut outlet_id: f64;
        for row in 0..rows {
            for col in 0..columns {
                if output[(row, col)] == low_value {
                    flag = false;
                    x = col;
                    y = row;
                    outlet_id = nodata;
                    while !flag {
                        dir = pntr[(y, x)] as usize;
                        if dir > 0 {
                            c = pntr_matches[dir];
                            y += dy[c];
                            x += dx[c];

                            // if the new cell already has a value in the output, use that as the outletID
                            z = output[(y, x)];
                            if z != low_value {
                                outlet_id = z;
                                flag = true;
                            }
                        } else {
                            flag = true;
                        }
                    }

                    flag = false;
                    x = col;
                    y = row;
                    output[(y, x)] = outlet_id;
                    while !flag {
                        // find its downslope neighbour
                        dir = pntr[(y, x)] as usize;
                        if dir > 0 {
                            c = pntr_matches[dir];
                            y += dy[c];
                            x += dx[c];

                            // if the new cell already has a value in the output, use that as the outletID
                            if output[(y, x)] != low_value {
                                flag = true;
                            }
                        } else {
                            flag = true;
                        }
                        output[(y, x)] = outlet_id;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Watershedding (Loop 2 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Replace all stream cells with 0's
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0f64 && streams[(row, col)] != nodata {
                    output[(row, col)] = 0f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Watershedding (Loop 1 of 2): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        /////////////////////
        // Clump the basins /
        /////////////////////

        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 1, -1)?;

        // 6 7 0
        // 5 x 1
        // 4 3 2
        let card1 = [0, 8, 1, 8, 2, 8, 3, 8]; // used to ensure that clumping doesn occur across stream lines at diagonals
        let card2 = [7, 1, 3, 5];
        let card3 = [1, 3, 5, 7];
        let (mut row2, mut col2): (isize, isize);
        current_id = 0f64;
        let mut old_id: f64;
        for row in 0..rows {
            for col in 0..columns {
                if visited[(row, col)] > 0
                    && pntr[(row, col)] != pntr_nodata
                    && output[(row, col)] > 0f64
                {
                    current_id += 1f64;
                    old_id = output[(row, col)];
                    stack.push((row, col));
                    while !stack.is_empty() {
                        let cell = stack.pop().expect("Error during pop operation.");
                        row2 = cell.0;
                        col2 = cell.1;
                        output[(row2, col2)] = current_id;
                        visited[(row2, col2)] = 0;

                        for n in 0..8 {
                            y = row2 + dy[n];
                            x = col2 + dx[n];
                            if output[(y, x)] == old_id && visited[(y, x)] > 0 {
                                let diag = card1[n];
                                if diag == 8 {
                                    // its a cardinal direction
                                    stack.push((y, x));
                                } else {
                                    // clumping can't cross a stream via a diagonal
                                    if streams[(row2 + dy[card2[diag]], col2 + dx[card2[diag]])]
                                        == 0f64
                                        || streams[(row2 + dy[card3[diag]], col2 + dx[card3[diag]])]
                                            == 0f64
                                    {
                                        stack.push((y, x));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Grouping hillslopes: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("D8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Streams file: {}", streams_file));
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
