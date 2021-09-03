/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/06/2017
Last Modified: 18/10/2019
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to assign the Strahler stream order to each link in a stream network. Stream ordering is often used
/// in hydro-geomorphic and ecological studies to quantify the relative size and importance of a stream segment to the overall
/// river system. There are several competing stream ordering schemes. Based on to this common stream numbering system, headwater
/// stream links are assigned an order of one. Stream order only increases downstream when two links of equal order join, otherwise
/// the downstream link is assigned the larger of the two link orders.
///
/// Strahler order and Horton order are similar approaches to assigning stream network hierarchy. Horton stream
/// order essentially starts with the Strahler order scheme, but subsequently replaces each of the assigned stream order
/// value along the main trunk of the network with the order value of the outlet. The main channel is not treated
/// differently compared with other tributaries in the Strahler ordering scheme.
///
/// The user must specify the names of a streams raster image (`--streams`) and D8 pointer image (`--d8_pntr`). Stream cells
/// are designated in the streams image as all positive, nonzero values. Thus all non-stream or background grid cells are
/// commonly assigned either zeros or NoData values. The pointer image is used to traverse the stream network and should only
/// be created using the D8 algorithm. Background cells will be assigned the NoData value in the output image, unless the
/// `--zero_background` parameter is used, in which case non-stream cells will be assigned zero values in the output.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// # Reference
/// Strahler, A. N. (1957). Quantitative analysis of watershed geomorphology. Eos, Transactions American Geophysical Union,
/// 38(6), 913-920.
///
/// # See Also
/// `HortonStreamOrder`, `HackStreamOrder`, `ShreveStreamMagnitude`, `TopologicalStreamOrder`
pub struct StrahlerStreamOrder {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl StrahlerStreamOrder {
    pub fn new() -> StrahlerStreamOrder {
        // public constructor
        let name = "StrahlerStreamOrder".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Assigns the Strahler stream order to each link in a stream network.".to_string();

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

        StrahlerStreamOrder {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for StrahlerStreamOrder {
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
        if verbose {
            println!("Reading streams data...")
        };
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let pntr_nodata = pntr.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = streams_nodata;
        }

        // make sure the input files have the same size
        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
        let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        output.reinitialize_values(0.0);

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [999usize; 129];
        let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];

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

            inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
        }

        let mut num_neighbouring_stream_cells: i8;
        let mut current_value: f64;
        let mut current_order: f64;
        let mut max_stream_order = streams_nodata;
        let mut flag: bool;
        let (mut x, mut y): (isize, isize);
        let (mut x2, mut y2): (isize, isize);
        let mut dir: usize;
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 {
                    // see if it is a headwater location
                    num_neighbouring_stream_cells = 0i8;
                    for c in 0..8 {
                        x = col + d_x[c];
                        y = row + d_y[c];
                        if streams[(y, x)] > 0.0 && pntr[(y, x)] == inflowing_vals[c] {
                            num_neighbouring_stream_cells += 1;
                        }
                    }
                    if num_neighbouring_stream_cells == 0i8 {
                        // it's a headwater location so start a downstream flowpath
                        x = col;
                        y = row;
                        current_order = 1f64;
                        output[(y, x)] = current_order;
                        flag = true;
                        while flag {
                            // find the downslope neighbour
                            if pntr[(y, x)] > 0.0 {
                                dir = pntr[(y, x)] as usize;
                                if dir > 128 || pntr_matches[dir] == 999 {
                                    return Err(Error::new(ErrorKind::InvalidInput,
                                        "An unexpected value has been identified in the pointer image. This tool requires a pointer grid that has been created using either the D8 or Rho8 tools."));
                                }

                                x += d_x[pntr_matches[dir]];
                                y += d_y[pntr_matches[dir]];

                                if streams[(y, x)] <= 0.0 {
                                    //it's not a stream cell
                                    flag = false;
                                } else {
                                    current_value = output[(y, x)];
                                    if current_value > current_order {
                                        //flag = false; // run into a larger stream, end the downstream search
                                        break;
                                    }
                                    if current_value == current_order {
                                        num_neighbouring_stream_cells = 0;
                                        for d in 0..8 {
                                            x2 = x + d_x[d];
                                            y2 = y + d_y[d];
                                            if streams[(y2, x2)] > 0.0
                                                && pntr[(y2, x2)] == inflowing_vals[d]
                                                && output[(y2, x2)] == current_order
                                            {
                                                num_neighbouring_stream_cells += 1;
                                            }
                                        }
                                        if num_neighbouring_stream_cells >= 2 {
                                            current_order += 1.0;
                                            if current_order > max_stream_order {
                                                max_stream_order = current_order;
                                            }
                                        } else {
                                            //flag = false;
                                            break;
                                        }
                                    }
                                    if current_value < current_order {
                                        output[(y, x)] = current_order;
                                    }
                                }
                            } else {
                                if streams[(y, x)] > 0.0 {
                                    //it is a valid stream cell and probably just has no downslope neighbour (e.g. at the edge of the grid)
                                    output.increment(y, x, 1.0);
                                }
                                flag = false;
                            }
                        }
                    }
                } else {
                    if pntr[(row, col)] != pntr_nodata {
                        output[(row, col)] = background_val;
                    } else {
                        output[(row, col)] = streams_nodata;
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

        if verbose {
            println!("Max stream order: {}", max_stream_order);
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        if background_val == 0.0f64 {
            output.configs.palette = "spectrum_black_background.plt".to_string();
        } else {
            output.configs.palette = "spectrum.plt".to_string();
        }
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input d8 pointer file: {}", d8_file));
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
