/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/01/2018
Last Modified: 03/09/2020
License: MIT
*/

use crate::raster::*;
use crate::structures::RectangleWithData;
use crate::tools::*;
use num_cpus;
use rstar::RTree;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool will create an image mosaic from one or more input image files using
/// one of three resampling methods including, nearest neighbour, bilinear interpolation,
/// and cubic convolution. The order of the input source image files is important. Grid
/// cells in the output image will be assigned the corresponding value determined from the
/// last image found in the list to possess an overlapping coordinate.
///
/// Note that when the `--inputs` parameter is left unspecified, the tool will use
/// all of the *.tif*, *.tiff*, *.rdc*, *.flt*, *.sdat*, and *.dep* files located in the working directory.
/// This can be a useful way of mosaicing large number of tiles, particularly when
/// the text string that would be required to specify all of the input tiles is
/// longer than the allowable limit.
///
/// This is the preferred mosaicing tool to use when appending multiple images with
/// little to no overlapping areas, e.g. tiled data. When images have significant overlap
/// areas, users are advised to use the `MosaicWithFeathering` tool instead.
///
/// Resample is very similar in operation to the Mosaic tool. The Resample tool should be
/// used when there is an existing image into which you would like to dump information from
/// one or more source images. If the source images are more extensive than the destination
/// image, i.e. there are areas that extend beyond the destination image boundaries, these
/// areas will not be represented in the updated image. Grid cells in the destination image
/// that are not overlapping with any of the input source images will not be updated, i.e.
/// they will possess the same value as before the resampling operation. The Mosaic tool is
/// used when there is no existing destination image. In this case, a new image is created
/// that represents the bounding rectangle of each of the two or more input images. Grid
/// cells in the output image that do not overlap with any of the input images will be
/// assigned the NoData value.
///
/// # See Also
/// `MosaicWithFeathering`
pub struct Mosaic {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Mosaic {
    pub fn new() -> Mosaic {
        // public constructor
        let name = "Mosaic".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Mosaics two or more images together.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Files".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input raster files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Resampling Method".to_owned(), 
            flags: vec!["--method".to_owned()], 
            description: "Resampling method; options include 'nn' (nearest neighbour), 'bilinear', and 'cc' (cubic convolution)".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["nn".to_owned(), "bilinear".to_owned(), "cc".to_owned()]),
            default_value: Some("nn".to_owned()),
            optional: true
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
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' -o=dest.tif --method='cc'", short_exe, name).replace("*", &sep);

        Mosaic {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Mosaic {
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
        let mut input_files = String::new();
        let mut output_file = String::new();
        let mut method = String::from("nn");

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
            if flag_val == "-i" || flag_val == "-inputs" {
                input_files = if keyval {
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
            } else if flag_val == "-method" {
                method = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                if method.to_lowercase().contains("nn") || method.to_lowercase().contains("nearest")
                {
                    method = "nn".to_string();
                } else if method.to_lowercase().contains("bilinear")
                    || method.to_lowercase().contains("bi")
                {
                    method = "bilinear".to_string();
                } else if method.to_lowercase().contains("cc")
                    || method.to_lowercase().contains("cubic")
                {
                    method = "cc".to_string();
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

        let mut input_vec: Vec<String> = vec![];

        let supported_raster_extensions = [".tif", ".tiff", ".dep", ".rdc", ".flt", ".sdat"];

        if input_files.is_empty() {
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

                    for extension in supported_raster_extensions.iter() {
                        if s.to_lowercase().ends_with(extension) {
                            input_vec.push(s);
                            break;
                        }
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("The input directory ({}) is incorrect.", working_directory),
                ));
            }
        } else {
            let mut cmd = input_files.split(";");
            input_vec = cmd
                .collect::<Vec<&str>>()
                .iter()
                .map(|x| String::from(*x))
                .collect();
            if input_vec.len() == 1 {
                cmd = input_files.split(",");
                input_vec = cmd
                    .collect::<Vec<&str>>()
                    .iter()
                    .map(|x| String::from(*x))
                    .collect();
            }
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        /*
        let mut cmd = input_files.split(";");
        let mut input_vec = cmd.collect::<Vec<&str>>();
        if input_vec.len() == 1 {
            cmd = input_files.split(",");
            input_vec = cmd.collect::<Vec<&str>>();
        }
        */

        let num_files = input_vec.len();
        println!("Number of tiles: {}", num_files);
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        let start = Instant::now();

        // read the input files
        if verbose {
            println!("Reading data...")
        };
        let mut inputs: Vec<Raster> = Vec::with_capacity(num_files);
        let mut nodata_vals: Vec<f64> = Vec::with_capacity(num_files);
        let mut north = f64::NEG_INFINITY;
        let mut south = f64::INFINITY;
        let mut east = f64::NEG_INFINITY;
        let mut west = f64::INFINITY;
        let mut resolution_x = f64::INFINITY;
        let mut resolution_y = f64::INFINITY;
        let mut north_greater_than_south = true;
        let mut east_greater_than_west = true;

        let mut tile_aabb = vec![];

        let mut i = 0;
        for a in 0..num_files {
            let value = &(input_vec[a]);
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let res = Raster::new(&input_file, "r");
                if res.is_ok() {
                    inputs.push(res.unwrap()); //Raster::new(&input_file, "r")?).expect(&format!("Error reading file: {}", value)));
                    nodata_vals.push(inputs[i].configs.nodata);

                    if i == 0 {
                        if inputs[i].configs.north < inputs[i].configs.south {
                            north_greater_than_south = false;
                            north = f64::INFINITY;
                            south = f64::NEG_INFINITY;
                        }
                        if inputs[i].configs.east < inputs[i].configs.west {
                            east_greater_than_west = false;
                            east = f64::INFINITY;
                            west = f64::NEG_INFINITY;
                        }
                    }

                    if north_greater_than_south {
                        if inputs[i].configs.north > north {
                            north = inputs[i].configs.north;
                        }
                        if inputs[i].configs.south < south {
                            south = inputs[i].configs.south;
                        }
                    } else {
                        if inputs[i].configs.north < north {
                            north = inputs[i].configs.north;
                        }
                        if inputs[i].configs.south > south {
                            south = inputs[i].configs.south;
                        }
                    }

                    if east_greater_than_west {
                        if inputs[i].configs.east > east {
                            east = inputs[i].configs.east;
                        }
                        if inputs[i].configs.west < west {
                            west = inputs[i].configs.west;
                        }
                    } else {
                        if inputs[i].configs.east < east {
                            east = inputs[i].configs.east;
                        }
                        if inputs[i].configs.west > west {
                            west = inputs[i].configs.west;
                        }
                    }

                    tile_aabb.push(RectangleWithData::new(
                        i,
                        [
                            inputs[i].configs.west - inputs[i].configs.resolution_x,
                            inputs[i].configs.south - inputs[i].configs.resolution_y,
                        ],
                        [
                            inputs[i].configs.east + inputs[i].configs.resolution_x,
                            inputs[i].configs.north + inputs[i].configs.resolution_y,
                        ],
                    ));

                    if inputs[i].configs.resolution_x < resolution_x {
                        resolution_x = inputs[i].configs.resolution_x;
                    }
                    if inputs[i].configs.resolution_y < resolution_y {
                        resolution_y = inputs[i].configs.resolution_y;
                    }

                    i += 1;
                } else {
                    println!("Warning: Error reading file {}", value);
                }
            } else {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "There is a problem with the list of input files. At least one specified input is empty."));
            }

            if verbose {
                progress = (100.0_f64 * a as f64 / (num_files - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let tree = Arc::new(RTree::bulk_load(tile_aabb));

        // num_files = inputs.len();

        // create the output image
        let rows = ((north - south).abs() / resolution_y).ceil() as isize;
        let columns = ((east - west).abs() / resolution_x).ceil() as isize;
        let south: f64 = north - rows as f64 * resolution_y;
        let east = west + columns as f64 * resolution_x;
        let nodata = -32768.0f64;

        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = resolution_x;
        configs.resolution_y = resolution_y;
        configs.nodata = nodata;
        configs.data_type = inputs[0].configs.data_type;
        configs.photometric_interp = inputs[0].configs.photometric_interp;
        configs.palette = inputs[0].configs.palette.clone();

        if verbose {
            println!(
                "Output image size: ({} x {})",
                configs.rows, configs.columns
            );
        }

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        // create the x and y arrays
        let mut x: Vec<f64> = Vec::with_capacity(columns as usize);
        for col in 0..columns {
            x.push(output.get_x_from_column(col));
        }

        let mut y: Vec<f64> = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            y.push(output.get_y_from_row(row));
        }

        let x = Arc::new(x);
        let y = Arc::new(y);
        let inputs = Arc::new(inputs);
        let nodata_vals = Arc::new(nodata_vals);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        if method == "nn" {
            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                let tree = tree.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let (mut col_src, mut row_src): (isize, isize);
                    let mut i: usize;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            // for i in 0..num_files {
                            let ret = tree
                                .locate_all_at_point(&[x[col as usize], y[row as usize]])
                                .collect::<Vec<_>>();

                            for a in 0..ret.len() {
                                i = ret[a].data;
                                row_src = inputs[i].get_row_from_y(y[row as usize]);
                                col_src = inputs[i].get_column_from_x(x[col as usize]);
                                z = inputs[i].get_value(row_src, col_src);

                                if z != nodata_vals[i] {
                                    data[col as usize] = z;
                                    break;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns {
                    if data[col as usize] != nodata {
                        output.set_value(row, col, data[col as usize]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else if method == "cc" {
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;

            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                let tree = tree.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let shift_x = [-1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2, -1, 0, 1, 2];
                    let shift_y = [-1, -1, -1, -1, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2];
                    let num_neighbours = 16;
                    let mut neighbour = [[0f64; 2]; 16];
                    let (mut col_src, mut row_src): (f64, f64);
                    let (mut col_n, mut row_n): (isize, isize);
                    let (mut origin_row, mut origin_col): (isize, isize);
                    let (mut dx, mut dy): (f64, f64);
                    let mut sum_dist: f64;
                    let mut i: usize;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            // let mut flag = true;
                            // for i in 0..num_files {
                            //     if !flag {
                            //         break;
                            //     }
                            let ret = tree
                                .locate_all_at_point(&[x[col as usize], y[row as usize]])
                                .collect::<Vec<_>>();

                            let mut flag = true;
                            for a in 0..ret.len() {
                                i = ret[a].data;
                                if !flag {
                                    break;
                                }

                                row_src = (inputs[i].configs.north - y[row as usize])
                                    / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west)
                                    / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);
                                    dy = row_n as f64 - row_src;
                                    dx = col_n as f64 - col_src;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata_vals[i] {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata_vals[i] {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        data[col as usize] = neighbour[n][0];
                                        flag = false;
                                    }
                                }

                                if sum_dist > 0f64 {
                                    z = 0f64;
                                    for n in 0..num_neighbours {
                                        z += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                    data[col as usize] = z;
                                    flag = false;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        output.set_value(row, col as isize, data[col]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        } else {
            // bilinear
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;
            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
                let tree = tree.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let shift_x = [0, 1, 0, 1];
                    let shift_y = [0, 0, 1, 1];
                    let num_neighbours = 4;
                    let mut neighbour = [[0f64; 2]; 4];
                    let (mut col_src, mut row_src): (f64, f64);
                    let (mut col_n, mut row_n): (isize, isize);
                    let (mut origin_col, mut origin_row): (isize, isize);
                    let (mut dx, mut dy): (f64, f64);
                    let mut sum_dist: f64;
                    let mut i: usize;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            // let mut flag = true;
                            // for i in 0..num_files {
                            let ret = tree
                                .locate_all_at_point(&[x[col as usize], y[row as usize]])
                                .collect::<Vec<_>>();
                            let mut flag = true;
                            for a in 0..ret.len() {
                                i = ret[a].data;
                                if !flag {
                                    break;
                                }
                                row_src = (inputs[i].configs.north - y[row as usize])
                                    / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west)
                                    / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);
                                    dy = row_n as f64 - row_src;
                                    dx = col_n as f64 - col_src;

                                    if (dx + dy) != 0f64 && neighbour[n][0] != nodata_vals[i] {
                                        neighbour[n][1] = 1f64 / (dx * dx + dy * dy);
                                        sum_dist += neighbour[n][1];
                                    } else if neighbour[n][0] == nodata_vals[i] {
                                        neighbour[n][1] = 0f64;
                                    } else {
                                        data[col as usize] = neighbour[n][0];
                                        flag = false;
                                    }
                                }

                                if sum_dist > 0f64 {
                                    z = 0f64;
                                    for n in 0..num_neighbours {
                                        z += (neighbour[n][0] * neighbour[n][1]) / sum_dist;
                                    }
                                    data[col as usize] = z;
                                    flag = false;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for r in 0..rows {
                let (row, data) = rx.recv().expect("Error receiving data from thread.");
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        output.set_value(row, col as isize, data[col]);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Modified by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Resampling method: {}", method));

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
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
