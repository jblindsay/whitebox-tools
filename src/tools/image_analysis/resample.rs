/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: January 1 2018
Last Modified: January 1, 2018
License: MIT

Note: Resample is very similar in operation to the Mosaic tool. The Resample tool should 
be used when there is an existing image into which you would like to dump information 
from one or more source images. If the source images are more extensive than the 
destination image, i.e. there are areas that extend beyond the destination image 
boundaries, these areas will not be represented in the updated image. Grid cells in the 
destination image that are not overlapping with any of the input source images will not 
be updated, i.e. they will possess the same value as before the resampling operation. The 
Mosaic tool is used when there is no existing destination image. In this case, a new 
image is created that represents the bounding rectangle of each of the two or more input 
images. Grid cells in the output image that do not overlap with any of the input images 
will be assigned the NoData value.
*/

use time;
use num_cpus;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct Resample {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Resample {
    pub fn new() -> Resample { // public constructor
        let name = "Resample".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Resamples one or more input images into a destination image.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input Files".to_owned(), 
            flags: vec!["-i".to_owned(), "--inputs".to_owned()], 
            description: "Input raster files.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Destination File".to_owned(), 
            flags: vec!["--destination".to_owned()], 
            description: "Destination raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Resampling Method".to_owned(), 
            flags: vec!["--method".to_owned()], 
            description: "Resampling method".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["nn".to_owned(), "bilinear".to_owned(), "cc".to_owned()]),
            default_value: Some("cc".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' -i='image1.tif;image2.tif;image3.tif' --destination=dest.tif --method='cc", short_exe, name).replace("*", &sep);
    
        Resample { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for Resample {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_files = String::new();
        let mut destination_file = String::new();
        let mut method = String::from("cc");
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Tool run with no paramters."));
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
                    args[i+1].to_string()
                };
            } else if flag_val == "-destination" {
                destination_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-method" {
                method = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
                if method.to_lowercase().contains("nn") || method.to_lowercase().contains("nearest") {
                    method = "nn".to_string();
                } else if method.to_lowercase().contains("bilinear") || method.to_lowercase().contains("bi") {
                    method = "bilinear".to_string();
                } else if method.to_lowercase().contains("cc") || method.to_lowercase().contains("cubic") {
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

        if !destination_file.contains(&sep) && !destination_file.contains("/") {
            destination_file = format!("{}{}", working_directory, destination_file);
        }

        // see if the destination file exists.
        if !path::Path::new(&destination_file).exists() {
            return Err(Error::new(ErrorKind::InvalidInput,
                "The destination raster file does not exist. If you want to create a new file, try the Mosaic tool rather than Resample."));
        }

        let mut cmd = input_files.split(";");
        let mut input_vec = cmd.collect::<Vec<&str>>();
        if input_vec.len() == 1 {
            cmd = input_files.split(",");
            input_vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = input_vec.len();
        if num_files < 1 {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect about the input files. At least one input is required to operate this tool."));
        }

        let start = time::now();

        // Open the destination raster.
        let mut destination = Raster::new(&destination_file, "rw")?;
        let rows = destination.configs.rows as isize;
        let columns = destination.configs.columns as isize;
        let nodata = destination.configs.nodata;

        // read the input files
        if verbose { println!("Reading data...") };
        let mut inputs: Vec<Raster> = Vec::with_capacity(num_files);
        let mut nodata_vals: Vec<f64> = Vec::with_capacity(num_files);
        for i in 0..num_files {
            let value = input_vec[i];
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                inputs.push(Raster::new(&input_file, "r")?);
                nodata_vals.push(inputs[i].configs.nodata);
            } else {
                return Err(Error::new(ErrorKind::InvalidInput,
                    "There is a problem with the list of input files. At least one specified input is empty."));
            }
        }

        // create the x and y arrays
        let mut x: Vec<f64> = Vec::with_capacity(columns as usize);
        for col in 0..columns {
            x.push(destination.get_x_from_column(col));
        }

        let mut y: Vec<f64> = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            y.push(destination.get_y_from_row(row));
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
                thread::spawn(move || {
                    let mut z: f64;
                    let (mut col_src, mut row_src): (isize, isize);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            for i in 0..num_files {
                                row_src = inputs[i].get_row_from_y(y[row as usize]);
                                col_src = inputs[i].get_column_from_x(x[col as usize]);
                                // row_src = ((inputs[i].configs.north - y[row as usize]) / inputs[i].configs.resolution_y).round() as isize;
                                // col_src = ((x[col as usize] - inputs[i].configs.west) / inputs[i].configs.resolution_x).round() as isize;
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
                let (row, data) = rx.recv().unwrap();
                for col in 0..columns {
                    if data[col as usize] != nodata {
                        destination.set_value(row, col, data[col as usize]);
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
            destination.configs.photometric_interp = PhotometricInterpretation::Continuous;
            destination.configs.data_type = DataType::F32;

            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
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
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            let mut flag = true;
                            for i in 0..num_files {
                                if !flag { break; }
                                // row_src = inputs[i].get_row_from_y(y[row as usize]);
                                // col_src = inputs[i].get_column_from_x(x[col as usize]);
                                row_src = (inputs[i].configs.north - y[row as usize]) / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west) / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);;
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
                let (row, data) = rx.recv().unwrap();
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        destination.set_value(row, col as isize, data[col]);
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

        } else { // bilinear
            destination.configs.photometric_interp = PhotometricInterpretation::Continuous;
            destination.configs.data_type = DataType::F32;
            for tid in 0..num_procs {
                let inputs = inputs.clone();
                let nodata_vals = nodata_vals.clone();
                let x = x.clone();
                let y = y.clone();
                let tx = tx.clone();
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
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            let mut flag = true;
                            for i in 0..num_files {
                                if !flag { break; }
                                row_src = (inputs[i].configs.north - y[row as usize]) / inputs[i].configs.resolution_y;
                                col_src = (x[col as usize] - inputs[i].configs.west) / inputs[i].configs.resolution_x;
                                origin_row = row_src.floor() as isize;
                                origin_col = col_src.floor() as isize;
                                sum_dist = 0f64;
                                for n in 0..num_neighbours {
                                    row_n = origin_row + shift_y[n];
                                    col_n = origin_col + shift_x[n];
                                    neighbour[n][0] = inputs[i].get_value(row_n, col_n);;
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
                let (row, data) = rx.recv().unwrap();
                for col in 0..columns as usize {
                    if data[col] != nodata {
                        destination.set_value(row, col as isize, data[col]);
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
        
        let end = time::now();
        let elapsed_time = end - start;
        destination.add_metadata_entry(format!("Modified by whitebox_tools\' {} tool", self.get_tool_name()));
        
        if verbose { println!("Saving data...") };
        let _ = match destination.write() {
            Ok(_) => if verbose { println!("Destination file written") },
            Err(e) => return Err(e),
        };
        if verbose {
            println!("{}", &format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}