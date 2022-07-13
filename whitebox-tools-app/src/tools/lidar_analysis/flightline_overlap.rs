/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/06/2017
Last Modified: 24/03/2022
License: MIT
*/

use kd_tree::{KdPoint, KdTree};
use whitebox_common::structures::Point3D;
use whitebox_lidar::*;
use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to map areas of overlapping flightlines in an input LiDAR (LAS) file (`--input`). 
/// The output raster file (`--output`) will contain the number of different flightlines that are contained
/// within each grid cell. The user must specify the desired cell size (`--resolution`). The flightline 
/// associated with a LiDAR point is assumed to be contained within the point's `Point Source ID` property.
/// Thus, the tool essentially counts the number of different Point Source ID values among the points contained
/// within each grid cell. If the Point Source ID property is not set, or has been lost, users may with to
/// apply the `RecoverFlightlineInfo` tool prior to running `FlightlineOverlap`.
/// 
/// It is important to set the `--resolution` parameter appropriately, as setting this value too high will
/// yield the mis-characterization of non-overlap areas, and setting the resolution to low will result in
/// fewer than expected overlap areas. An appropriate resolution size value may require experimentation,
/// however a value that is 2-3 times the nominal point spacing has been previously recommended. The nominal
/// point spacing can be determined using the `LidarInfo` tool.
/// 
/// Note that this tool is intended to be applied to LiDAR tile data containing points that have been merged
/// from multiple overlapping flightlines. It is commonly the case that airborne LiDAR data from each of the
/// flightlines from a survey are merged and then tiled into 1 km<sup>2</sup> tiles, which are the target
/// dataset for this tool.
/// 
/// Like many of the LiDAR related tools, the input and output file parameters are optional. If left unspecified,
/// the tool will locate all valid LiDAR files within the current Whitebox working directory and use these
/// for calculation (specifying the output raster file name based on the associated input LiDAR file). This can
/// be a helpful way to run the tool on a batch of user inputs within a specific directory.
///
/// # See Also
/// `ClassifyOverlapPoints`, `RecoverFlightlineInfo`, `LidarInfo`
pub struct FlightlineOverlap {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FlightlineOverlap {
    pub fn new() -> FlightlineOverlap {
        // public constructor
        let name = "FlightlineOverlap".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Reads a LiDAR (LAS) point file and outputs a raster containing the number of overlapping flight-lines in each grid cell.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input LiDAR file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Lidar),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=2.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.tif --resolution=5.0 --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        FlightlineOverlap {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FlightlineOverlap {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut grid_res: f64 = 1.0;
        let mut palette = "default".to_string();

        // read the arguments
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
            } else if flag_val == "-resolution" {
                grid_res = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-palette" {
                palette = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        let mut inputs = vec![];
        let mut outputs = vec![];
        if input_file.is_empty() {
            if working_directory.is_empty() {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "This tool must be run by specifying either an individual input file or a working directory."));
            }
            if std::path::Path::new(&working_directory).is_dir() {
                for entry in fs::read_dir(working_directory.clone())? {
                    let s = entry?
                        .path()
                        .into_os_string()
                        .to_str()
                        .expect("Error reading path string")
                        .to_string();
                    if s.to_lowercase().ends_with(".las") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".las", ".tif")
                                .replace(".LAS", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zlidar") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zlidar", ".tif")
                                .replace(".ZLIDAR", ".tif"),
                        )
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
                        outputs.push(
                            inputs[inputs.len() - 1]
                                .replace(".zip", ".tif")
                                .replace(".ZIP", ".tif"),
                        )
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("The input directory ({}) is incorrect.", working_directory),
                ));
            }
        } else {
            inputs.push(input_file.clone());
            if output_file.is_empty() {
                output_file = input_file
                    .clone()
                    .replace(".las", ".tif")
                    .replace(".LAS", ".tif")
                    .replace(".zlidar", ".tif");
            }
            outputs.push(output_file);
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

        for k in 0..inputs.len() {
            input_file = inputs[k].replace("\"", "").clone();
            output_file = outputs[k].replace("\"", "").clone();

            if verbose && inputs.len() > 1 {
                println!(
                    "Gridding {} of {} ({})",
                    k + 1,
                    inputs.len(),
                    input_file.clone()
                );
            }

            if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
                input_file = format!("{}{}", working_directory, input_file);
            }
            if !output_file.contains(path::MAIN_SEPARATOR) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }

            if verbose && inputs.len() == 1 {
                println!("reading input LiDAR file...");
            }
            let input = match LasFile::new(&input_file, "r") {
                Ok(lf) => lf,
                Err(err) => panic!("Error reading file {}: {}", input_file, err),
            };

            let start_run = Instant::now();

            if verbose && inputs.len() == 1 {
                println!("Performing analysis...");
            }

            // Make sure that the input LAS file have GPS time data?
            if input.header.point_format == 0u8 || input.header.point_format == 2u8 {
                panic!("The input file has a Point Format that does not include GPS time, which is required for the operation of this tool.");
            }

            let n_points = input.header.number_of_points as usize;

            let mut points: Vec<TreeItem> = Vec::with_capacity(n_points);
            let mut p: Point3D;
            for i in 0..n_points {
                if !input[i].withheld() {
                    p = input.get_transformed_coords(i);
                    points.push( TreeItem { point: [p.x, p.y ], id: i } );
                }
            }
            // build the tree
            if verbose {
                println!("Building kd-tree...");
            }
            let kdtree: KdTree<TreeItem> = KdTree::build_by_ordered_float(points);


            let west: f64 = input.header.min_x; // - 0.5 * grid_res;
            let north: f64 = input.header.max_y; // + 0.5 * grid_res;
            let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
            let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;
            let south: f64 = north - rows as f64 * grid_res;
            let east = west + columns as f64 * grid_res;
            let nodata = -32768.0f64;

            let mut configs = RasterConfigs {
                ..Default::default()
            };
            configs.rows = rows;
            configs.columns = columns;
            configs.north = north;
            configs.south = south;
            configs.east = east;
            configs.west = west;
            configs.resolution_x = grid_res;
            configs.resolution_y = grid_res;
            configs.nodata = nodata;
            configs.data_type = DataType::F64;
            configs.photometric_interp = PhotometricInterpretation::Continuous;
            configs.palette = palette.clone();
            let mut output = Raster::initialize_using_config(&output_file, &configs);
            let (mut x, mut y): (f64, f64);
            let (mut x_n, mut y_n): (f64, f64);
            let mut index_n: usize;
            let half_res_sqrd = grid_res / 2.0 * grid_res / 2.0;
            let search_dist = grid_res * 2.0_f64.sqrt();
            for row in 0..rows as isize {
                for col in 0..columns as isize {
                    x = west + col as f64 * grid_res + 0.5;
                    y = north - row as f64 * grid_res - 0.5;

                    let ret = kdtree.within_radius(&[x, y], search_dist);

                    if ret.len() > 0 {
                        let mut pt_src_ids = Vec::with_capacity(ret.len());
                        for i in 0..ret.len() {
                            x_n = ret[i].point[0];
                            y_n = ret[i].point[1];
                            if (x_n - x).powi(2) <= half_res_sqrd
                                && (y_n - y).powi(2) <= half_res_sqrd
                            {
                                // it falls within the grid cell
                                index_n = ret[i].id;
                                pt_src_ids.push(input[index_n].point_source_id);
                            }
                        }

                        if pt_src_ids.len() > 0 {
                            pt_src_ids.sort();
                            let mut num_flightlines = 1.0;
                            for j in 1..pt_src_ids.len() {
                                if pt_src_ids[j] != pt_src_ids[j-1] {
                                    num_flightlines += 1.0;
                                }
                            }
                            output.set_value(row, col, num_flightlines);
                        } else {
                            output.set_value(row, col, nodata);
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

            let elapsed_time_run = get_formatted_elapsed_time(start_run);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("Input file: {}", input_file));
            output.add_metadata_entry(format!(
                "Elapsed Time (excluding I/O): {}",
                elapsed_time_run
            ));

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
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

struct TreeItem {
    point: [f64; 2],
    id: usize,
}

impl KdPoint for TreeItem {
    type Scalar = f64;
    type Dim = typenum::U2; // 3 dimensional tree.
    fn at(&self, k: usize) -> f64 { self.point[k] }
}