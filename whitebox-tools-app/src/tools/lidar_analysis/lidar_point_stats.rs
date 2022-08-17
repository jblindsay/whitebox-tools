/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 18/02/2018
Last Modified: 20/05/2020
License: MIT
*/

use whitebox_lidar::*;
use whitebox_raster::*;
use whitebox_common::structures::{Array2D, Point3D};
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// This tool creates several rasters summarizing the distribution of LiDAR points in a LAS data file.
/// The user must specify the name of an input LAS file (`--input`) and the output raster grid
/// resolution (`--resolution`). Additionally, the user must specify one or more of the possible
/// output rasters to create using the various available flags, which include:
///
/// | Flag                     | Meaning                                               |
/// | :----------------------- | :-----------------------------------------------------|
/// | `--num_points`           | Number of points (returns) in each grid cell          |
/// | `--num_pulses`           | Number of pulses in each grid cell                    |
/// | `--avg_points_per_pulse` | Average number of points per pulse in each grid cells |
/// | `--z_range`              | Elevation range within each grid cell                 |
/// | `--intensity_range`      | Intensity range within each grid cell                 |
/// | `--predom_class`         | Predominant class value within each grid cell         |
///
/// If no output raster flags are specified, all of the output rasters will be created. All output
/// rasters will have the same base name as the input LAS file but will have a suffix that
/// reflects the statistic type (e.g. _num_pnts, _num_pulses, _avg_points_per_pulse, etc.). Output
/// files will be in the GeoTIFF (*.tif) file format.
///
/// When the input/output parameters are not specified, the tool works on all LAS files contained within
/// the working directory.
///
/// **Notes**:
/// 1. The num_pulses output is actually the number of pulses with at lease one return; specifically it is
///    the sum of the early returns (first and only) in a grid cell. In areas of low reflectance, such as
///    over water surfaces, the system may have emitted a significantly higher pulse rate but far fewer
///    returns are observed.
/// 2. The memory requirement of this tool is high, particulalry if the grid resolution is fine and
///    the spatial extent is large.
///
///
/// # See Also
/// `LidarBlockMinimum`, `LidarBlockMaximum`
pub struct LidarPointStats {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LidarPointStats {
    pub fn new() -> LidarPointStats {
        // public constructor
        let name = "LidarPointStats".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description = "Creates several rasters summarizing the distribution of LAS point data. When the input/output parameters are not specified, the tool works on all LAS files contained within the working directory.".to_string();

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
            name: "Grid Resolution".to_owned(),
            flags: vec!["--resolution".to_owned()],
            description: "Output raster's grid resolution.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output number of points?".to_owned(),
            flags: vec!["--num_points".to_owned()],
            description:
                "Flag indicating whether or not to output the number of points (returns) raster."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("True".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output number of pulses?".to_owned(),
            flags: vec!["--num_pulses".to_owned()],
            description: "Flag indicating whether or not to output the number of pulses raster."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output average number of points per pulse?".to_owned(),
            flags: vec!["--avg_points_per_pulse".to_owned()],
            description: "Flag indicating whether or not to output the average number of points (returns) per pulse raster."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("True".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output elevation range?".to_owned(),
            flags: vec!["--z_range".to_owned()],
            description: "Flag indicating whether or not to output the elevation range raster."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output intensity range?".to_owned(),
            flags: vec!["--intensity_range".to_owned()],
            description: "Flag indicating whether or not to output the intensity range raster."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output predominant class?".to_owned(),
            flags: vec!["--predom_class".to_owned()],
            description:
                "Flag indicating whether or not to output the predominant classification raster."
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las --resolution=1.0 --num_points",
            short_exe, name
        )
        .replace("*", &sep);

        LidarPointStats {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarPointStats {
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
        let mut grid_res: f64 = 1.0;
        let mut num_points = false;
        let mut num_pulses = false;
        let mut avg_points_per_pulse = false;
        let mut z_range = false;
        let mut intensity_range = false;
        let mut predominant_class = false;

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
            } else if flag_val == "-num_points" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    num_points = true;
                }
            } else if flag_val == "-num_pulses" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    num_pulses = true;
                }
            } else if flag_val == "-z_range" || flag_val == "elev_range" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    z_range = true;
                }
            } else if flag_val == "-intensity_range" || flag_val == "i_range" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    intensity_range = true;
                }
            } else if flag_val == "-predom_class" || flag_val == "-predominant_class" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    predominant_class = true;
                }
            } else if flag_val == "-avg_points_per_pulse" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    avg_points_per_pulse = true;
                }
            }
        }

        let start = Instant::now();

        // check to see if all of the outputs are false and if so, set them all the true
        if !num_points
            && !num_pulses
            && !avg_points_per_pulse
            && !z_range
            && !intensity_range
            && !predominant_class
        {
            num_points = true;
            num_pulses = true;
            avg_points_per_pulse = true;
            z_range = true;
            intensity_range = true;
            predominant_class = true;
        }

        let mut inputs = vec![];
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
                    if s.to_lowercase().ends_with(".las") || s.to_lowercase().ends_with(".zlidar") 
                    || s.to_lowercase().ends_with(".laz") {
                        inputs.push(s);
                    } else if s.to_lowercase().ends_with(".zip") {
                        inputs.push(s);
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

        let num_tiles = inputs.len();
        let tile_list = Arc::new(Mutex::new(0..num_tiles));
        let inputs = Arc::new(inputs);
        let num_procs2 = num_cpus::get() as isize;
        let (tx2, rx2) = mpsc::channel();
        for _ in 0..num_procs2 {
            let inputs = inputs.clone();
            let tile_list = tile_list.clone();
            // copy over the string parameters
            let tool_name = self.get_tool_name();
            let tx2 = tx2.clone();
            thread::spawn(move || {
                let mut tile = 0;
                while tile < num_tiles {
                    // Get the next tile up for interpolation
                    {
                        tile = match tile_list.lock().unwrap().next() {
                            Some(val) => val,
                            None => break, // There are no more tiles to interpolate
                        };
                    }

                    let input_file = inputs[tile].replace("\"", "").clone();
                    if verbose && inputs.len() == 1 {
                        println!("reading input LiDAR file...");
                    }
                    let input = match LasFile::new(&input_file, "r") {
                        Ok(lf) => lf,
                        Err(err) => panic!("Error reading file {}: {}", input_file, err),
                    };

                    let mut progress: i32;
                    let mut old_progress: i32 = -1;

                    let west: f64 = input.header.min_x; // - 0.5 * grid_res;
                    let north: f64 = input.header.max_y; // + 0.5 * grid_res;
                    let rows: usize = (((north - input.header.min_y) / grid_res).ceil()) as usize;
                    let columns: usize = (((input.header.max_x - west) / grid_res).ceil()) as usize;
                    let south: f64 = north - rows as f64 * grid_res;
                    let east = west + columns as f64 * grid_res;
                    let nodata = -32768.0f64;
                    let half_grid_res = grid_res / 2.0;
                    let ns_range = north - south;
                    let ew_range = east - west;

                    let mut configs = RasterConfigs {
                        ..Default::default()
                    };
                    configs.rows = rows as usize;
                    configs.columns = columns as usize;
                    configs.north = north;
                    configs.south = south;
                    configs.east = east;
                    configs.west = west;
                    configs.resolution_x = grid_res;
                    configs.resolution_y = grid_res;
                    configs.nodata = nodata;
                    configs.data_type = DataType::F64;
                    configs.photometric_interp = PhotometricInterpretation::Continuous;

                    let n_points = input.header.number_of_points as usize;
                    let num_points_float: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

                    if num_points || num_pulses || avg_points_per_pulse {
                        let out_file_num_pnts = input_file
                            .replace(".las", "_num_pnts.tif")
                            .replace(".laz", "_num_pnts.tif")
                            .replace(".zlidar", "_num_pnts.tif")
                            .clone();
                        let mut out_num_pnts =
                            Raster::initialize_using_config(&out_file_num_pnts, &configs);
                        out_num_pnts.reinitialize_values(0f64);

                        let out_file_num_pulses = input_file
                            .replace(".las", "_num_pulses.tif")
                            .replace(".laz", "_num_pulses.tif")
                            .replace(".zlidar", "_num_pulses.tif")
                            .clone();
                        let mut out_num_pulses =
                            Raster::initialize_using_config(&out_file_num_pulses, &configs);
                        out_num_pulses.reinitialize_values(0f64);

                        let start_run = Instant::now();

                        let (mut row, mut col): (isize, isize);
                        let mut p: Point3D;
                        let mut pd: PointData;
                        for i in 0..n_points {
                            pd = input[i];
                            p = input.get_transformed_coords(i);
                            col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range)
                                .round()) as isize;
                            row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range)
                                .round()) as isize;

                            out_num_pnts.increment(row, col, 1f64);

                            if pd.is_early_return() {
                                out_num_pulses.increment(row, col, 1f64);
                            }

                            if verbose && inputs.len() == 1 {
                                progress = (100.0_f64 * i as f64 / num_points_float) as i32;
                                if progress != old_progress {
                                    println!("Progress: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }

                        let elapsed_time_run = get_formatted_elapsed_time(start_run);

                        if verbose && inputs.len() == 1 {
                            println!("Saving data...")
                        };

                        if avg_points_per_pulse {
                            let out_file_avg_points_per_pulse = input_file
                                .replace(".las", "_avg_points_per_pulse.tif")
                                .replace(".laz", "_avg_points_per_pulse.tif")
                                .replace(".zlidar", "_avg_points_per_pulse.tif")
                                .clone();
                            let mut out_avg_points_per_pulse = Raster::initialize_using_config(
                                &out_file_avg_points_per_pulse,
                                &configs,
                            );

                            for row in 0..rows as isize {
                                for col in 0..columns as isize {
                                    if out_num_pulses.get_value(row, col) > 0f64 {
                                        out_avg_points_per_pulse.set_value(
                                            row,
                                            col,
                                            out_num_pnts.get_value(row, col)
                                                / out_num_pulses.get_value(row, col),
                                        );
                                    } else {
                                        out_avg_points_per_pulse.set_value(row, col, 0f64);
                                    }
                                }
                            }

                            out_avg_points_per_pulse.add_metadata_entry(format!(
                                "Created by whitebox_tools\' {} tool",
                                tool_name
                            ));
                            out_avg_points_per_pulse
                                .add_metadata_entry(format!("Input file: {}", input_file));
                            out_avg_points_per_pulse
                                .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                            out_avg_points_per_pulse.add_metadata_entry(
                                format!("Elapsed Time (excluding I/O): {}", elapsed_time_run)
                                    .replace("PT", ""),
                            );
                            let _ = out_avg_points_per_pulse.write().unwrap();
                        }

                        if num_points {
                            out_num_pnts.add_metadata_entry(format!(
                                "Created by whitebox_tools\' {} tool",
                                tool_name
                            ));
                            out_num_pnts.add_metadata_entry(format!("Input file: {}", input_file));
                            out_num_pnts
                                .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                            out_num_pnts.add_metadata_entry(format!(
                                "Elapsed Time (excluding I/O): {}",
                                elapsed_time_run
                            ));
                            let _ = out_num_pnts.write().unwrap();
                        }
                        drop(out_num_pnts);

                        if num_pulses {
                            out_num_pulses.add_metadata_entry(format!(
                                "Created by whitebox_tools\' {} tool",
                                tool_name
                            ));
                            out_num_pulses
                                .add_metadata_entry(format!("Input file: {}", input_file));
                            out_num_pulses
                                .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                            out_num_pulses.add_metadata_entry(
                                format!("Elapsed Time (excluding I/O): {}", elapsed_time_run)
                                    .replace("PT", ""),
                            );
                            let _ = out_num_pulses.write().unwrap();
                        }
                        drop(out_num_pulses);
                    }

                    if z_range || intensity_range {
                        let mut min_z: Array2D<f64> =
                            Array2D::new(rows as isize, columns as isize, f64::INFINITY, nodata)
                                .unwrap();
                        let mut max_z: Array2D<f64> = Array2D::new(
                            rows as isize,
                            columns as isize,
                            f64::NEG_INFINITY,
                            nodata,
                        )
                        .unwrap();
                        let out_file_elev_range = input_file
                            .replace(".las", "_elev_range.tif")
                            .replace(".laz", "_elev_range.tif")
                            .replace(".zlidar", "_elev_range.tif")
                            .clone();
                        let mut out_elev_range =
                            Raster::initialize_using_config(&out_file_elev_range, &configs);

                        let mut min_i: Array2D<u16> =
                            Array2D::new(rows as isize, columns as isize, u16::max_value(), 0u16)
                                .unwrap();
                        let mut max_i: Array2D<u16> =
                            Array2D::new(rows as isize, columns as isize, u16::min_value(), 0u16)
                                .unwrap();
                        let out_file_intensity_range = input_file
                            .replace(".las", "_intensity_range.tif")
                            .replace(".laz", "_intensity_range.tif")
                            .replace(".zlidar", "_intensity_range.tif")
                            .clone();
                        let mut out_intensity_range =
                            Raster::initialize_using_config(&out_file_intensity_range, &configs);

                        let start_run = Instant::now();

                        let mut new_min_max_z: bool;
                        let mut new_min_max_i: bool;
                        let (mut row, mut col): (isize, isize);
                        let mut p: Point3D;
                        let mut pd: PointData;
                        for i in 0..n_points {
                            pd = input[i];
                            p = input.get_transformed_coords(i);
                            col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range)
                                .round()) as isize;
                            row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range)
                                .round()) as isize;

                            new_min_max_z = false;
                            if p.z < min_z.get_value(row, col) {
                                min_z.set_value(row, col, p.z);
                                new_min_max_z = true;
                            }

                            if p.z > max_z.get_value(row, col) {
                                max_z.set_value(row, col, p.z);
                                new_min_max_z = true;
                            }

                            if new_min_max_z {
                                out_elev_range.set_value(
                                    row,
                                    col,
                                    max_z.get_value(row, col) - min_z.get_value(row, col),
                                );
                            }

                            new_min_max_i = false;
                            if pd.intensity < min_i.get_value(row, col) {
                                min_i.set_value(row, col, pd.intensity);
                                new_min_max_i = true;
                            }

                            if pd.intensity > max_i.get_value(row, col) {
                                max_i.set_value(row, col, pd.intensity);
                                new_min_max_i = true;
                            }

                            if new_min_max_i {
                                out_intensity_range.set_value(
                                    row,
                                    col,
                                    (max_i.get_value(row, col) - min_i.get_value(row, col)) as f64,
                                );
                            }

                            if verbose && inputs.len() == 1 {
                                progress = (100.0_f64 * i as f64 / num_points_float) as i32;
                                if progress != old_progress {
                                    println!("Progress: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }

                        let elapsed_time_run = get_formatted_elapsed_time(start_run);

                        if verbose && inputs.len() == 1 {
                            println!("Saving data...")
                        };

                        if z_range {
                            out_elev_range.add_metadata_entry(format!(
                                "Created by whitebox_tools\' {} tool",
                                tool_name
                            ));
                            out_elev_range
                                .add_metadata_entry(format!("Input file: {}", input_file));
                            out_elev_range
                                .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                            out_elev_range.add_metadata_entry(format!(
                                "Elapsed Time (excluding I/O): {}",
                                elapsed_time_run
                            ));
                            let _ = out_elev_range.write().unwrap();
                        }
                        drop(out_elev_range);

                        if intensity_range {
                            out_intensity_range.add_metadata_entry(format!(
                                "Created by whitebox_tools\' {} tool",
                                tool_name
                            ));
                            out_intensity_range
                                .add_metadata_entry(format!("Input file: {}", input_file));
                            out_intensity_range
                                .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                            out_intensity_range.add_metadata_entry(format!(
                                "Elapsed Time (excluding I/O): {}",
                                elapsed_time_run
                            ));
                            let _ = out_intensity_range.write().unwrap();
                        }
                        drop(out_intensity_range);
                    }

                    if predominant_class {
                        let mut max_class: Array2D<u16> =
                            Array2D::new(rows as isize, columns as isize, u16::min_value(), 0u16)
                                .unwrap();
                        let mut class_histo: Vec<Array2D<u16>> = vec![];
                        for _ in 0..19 {
                            class_histo.push(
                                Array2D::new(rows as isize, columns as isize, 0u16, 0u16).unwrap(),
                            );
                        }
                        let out_file_predominant_class = input_file
                            .replace(".las", "_predominant_class.tif")
                            .replace(".laz", "_predominant_class.tif")
                            .replace(".zlidar", "_predominant_class.tif")
                            .clone();
                        let mut out_predominant_class =
                            Raster::initialize_using_config(&out_file_predominant_class, &configs);

                        let start_run = Instant::now();

                        let mut class: u8;
                        let mut freq: u16;
                        let (mut row, mut col): (isize, isize);
                        let mut p: Point3D;
                        let mut pd: PointData;
                        for i in 0..n_points {
                            pd = input[i];
                            p = input.get_transformed_coords(i);
                            col = (((columns - 1) as f64 * (p.x - west - half_grid_res) / ew_range)
                                .round()) as isize;
                            row = (((rows - 1) as f64 * (north - half_grid_res - p.y) / ns_range)
                                .round()) as isize;

                            class = pd.classification();
                            class_histo[class as usize].increment(row, col, 1u16);
                            freq = class_histo[class as usize].get_value(row, col);
                            if freq > max_class.get_value(row, col) {
                                max_class.set_value(row, col, freq);
                                out_predominant_class.set_value(row, col, class as f64);
                            }

                            if verbose && inputs.len() == 1 {
                                progress = (100.0_f64 * i as f64 / num_points_float) as i32;
                                if progress != old_progress {
                                    println!("Progress: {}%", progress);
                                    old_progress = progress;
                                }
                            }
                        }

                        let elapsed_time_run = get_formatted_elapsed_time(start_run);

                        if verbose && inputs.len() == 1 {
                            println!("Saving data...")
                        };

                        out_predominant_class.add_metadata_entry(format!(
                            "Created by whitebox_tools\' {} tool",
                            tool_name
                        ));
                        out_predominant_class
                            .add_metadata_entry(format!("Input file: {}", input_file));
                        out_predominant_class
                            .add_metadata_entry(format!("Grid resolution: {}", grid_res));
                        out_predominant_class.add_metadata_entry(format!(
                            "Elapsed Time (excluding I/O): {}",
                            elapsed_time_run
                        ));
                        let _ = out_predominant_class.write().unwrap();
                        drop(out_predominant_class);
                    }

                    tx2.send(tile).unwrap();
                }
            });
        }

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        for tile in 0..inputs.len() {
            let tile_completed = rx2.recv().unwrap();
            if verbose {
                println!(
                    "Finished {} ({} of {})",
                    inputs[tile_completed]
                        .replace("\"", "")
                        .replace(working_directory, "")
                        .replace(".las", "")
                        .replace(".laz", "")
                        .replace(".zlidar", ""),
                    tile + 1,
                    inputs.len()
                );
            }
            if verbose {
                progress = (100.0_f64 * tile as f64 / (inputs.len() - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
