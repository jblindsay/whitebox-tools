/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 06/07/2017
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

/// This tool can be used to measure the slope gradient, in degrees, each grid cell in a raster stream network. To
/// estimate the average slope for each link in a stream network, use the
/// `StreamLinkSlope` tool instead. The user must specify the names of a stream raster image (`--streams`), a D8
/// pointer image (`--d8_pntr`), and a digital elevation model (`--dem`). The pointer image is used to traverse the
/// stream network and must only be created using the D8 algorithm (`D8Pointer`).
/// Stream cells are designated in the streams image as all values greater than zero. Thus, all non-stream or background
/// grid cells are commonly assigned either zeros or NoData values. Background cells will be assigned the NoData value
/// in the output image, unless the `--zero_background` parameter is used, in which case non-stream cells will be assigned
/// zero values in the output.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// # See Also
/// `StreamLinkSlope`, `D8Pointer`
pub struct StreamSlopeContinuous {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl StreamSlopeContinuous {
    pub fn new() -> StreamSlopeContinuous {
        // public constructor
        let name = "StreamSlopeContinuous".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Estimates the slope of each grid cell in a stream network.".to_string();

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --linkid=streamsID.tif --dem=dem.tif -o=output.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streamsID.tif --dem=dem.tif -o=output.tif --esri_pntr --zero_background", short_exe, name).replace("*", &sep);

        StreamSlopeContinuous {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for StreamSlopeContinuous {
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
        let mut dem_file = String::new();
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
            } else if vec[0].to_lowercase() == "-dem" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    dem_file = vec[1].to_string();
                } else {
                    dem_file = args[i + 1].to_string();
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
        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading pointer data...")
        };
        let pntr = Arc::new(Raster::new(&d8_file, "r")?);
        let pntr_nodata = pntr.configs.nodata;
        if verbose {
            println!("Reading link ID data...")
        };
        let streams = Arc::new(Raster::new(&streams_file, "r")?);
        if verbose {
            println!("Reading DEM data...")
        };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = streams.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }
        let mut cell_size_x = streams.configs.resolution_x;
        let mut cell_size_y = streams.configs.resolution_y;
        let mut diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let dem_nodata = dem.configs.nodata;

        if streams.is_in_geographic_coordinates()
            || pntr.is_in_geographic_coordinates()
            || dem.is_in_geographic_coordinates()
        {
            let mut mid_lat = (streams.configs.north - streams.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                cell_size_x = cell_size_x * (111320.0 * mid_lat.cos());
                cell_size_y = cell_size_y * (111320.0 * mid_lat.cos());
                diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
            }
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
        if streams.configs.rows != dem.configs.rows
            || streams.configs.columns != dem.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let pntr = pntr.clone();
            let streams = streams.clone();
            let dem = dem.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
                let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
                if esri_style {
                    inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
                }
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
                let mut dir: usize;
                let mut z_inflowing: f64;
                let mut dist: f64;
                let mut n_inflowing: f64;
                let mut z_dn: f64;
                let mut c: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if streams[(row, col)] > 0f64
                            && streams[(row, col)] != nodata
                            && dem[(row, col)] != dem_nodata
                            && pntr[(row, col)] != pntr_nodata
                        {
                            // first get the average elevation and distance of the inflowing stream cells
                            z_inflowing = 0f64;
                            dist = 0f64;
                            n_inflowing = 0f64;
                            for n in 0..8 {
                                if streams[(row + dy[n], col + dx[n])] > 0.0
                                    && streams[(row + dy[n], col + dx[n])] != nodata
                                    && pntr[(row + dy[n], col + dx[n])] == inflowing_vals[n]
                                {
                                    z_inflowing += dem[(row + dy[n], col + dx[n])];
                                    dist += grid_lengths[n];
                                    n_inflowing += 1f64;
                                }
                            }
                            if n_inflowing > 0f64 {
                                z_inflowing = z_inflowing / n_inflowing;
                                dist = dist / n_inflowing;
                            } else {
                                z_inflowing = dem[(row, col)];
                            }

                            // now find the elevation of the downslope stream cell
                            dir = pntr[(row, col)] as usize;
                            if dir > 0 {
                                c = pntr_matches[dir];
                                z_dn = dem[(row + dy[c], col + dx[c])];
                                if z_dn != dem_nodata {
                                    dist += grid_lengths[c];
                                } else {
                                    z_dn = dem[(row, col)];
                                }
                            } else {
                                z_dn = dem[(row, col)];
                            }
                            if dist > 0f64 {
                                data[col as usize] =
                                    ((z_inflowing - z_dn) / dist).atan().to_degrees();
                            } else {
                                data[col as usize] = 0f64;
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        output.configs.data_type = DataType::F32;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
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
        output.add_metadata_entry(format!("Input streams ID file: {}", streams_file));
        output.add_metadata_entry(format!("Input DEM file: {}", dem_file));
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
