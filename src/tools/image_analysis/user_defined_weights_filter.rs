/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/04/2018
Last Modified: 27/04/2018
License: MIT

NOTES: NoData values in the input image are ignored during the convolution operation. 
This can lead to unexpected behavior at the edges of images (since the default behavior 
is to return NoData when addressing cells beyond the grid edge) and where the grid 
contains interior areas of NoData values. Normalization of kernel weights can be useful 
for handling the edge effects associated with interior areas of NoData values. When the 
normalization option is selected, the sum of the cell value-weight product is divided 
by the sum of the weights on a cell-by-cell basis. Therefore, if the kernel at a 
particular grid cell contains neighboring cells of NoData values, normalization 
effectively re-adjusts the weighting to account for the missing data values. Normalization 
also ensures that the output image will possess values within the range of the input 
image and allows the user to specify integer value weights in the kernel. However, note
that this implies that the sum of weights should equal one. In some cases, alternative
sums (e.g. zero) are more appropriate, and as such normalization should not be applied
in these cases.
*/

use time;
use num_cpus;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct UserDefinedWeightsFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl UserDefinedWeightsFilter {
    pub fn new() -> UserDefinedWeightsFilter { // public constructor
        let name = "UserDefinedWeightsFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs a user-defined weights filter on an image.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Input Weights File".to_owned(), 
            flags: vec!["--weights".to_owned()], 
            description: "Input weights file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Csv),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Kernel Center".to_owned(), 
            flags: vec!["--center".to_owned()], 
            description: "Kernel center cell; options include 'center', 'upper-left', 'upper-right', 'lower-left', 'lower-right'".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["center".to_owned(), "upper-left".to_owned(), "upper-right".to_owned(), "lower-left".to_owned(), "lower-right".to_owned()]),
            default_value: Some("center".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Normalize kernel weights?".to_owned(), 
            flags: vec!["--normalize".to_owned()], 
            description: "Normalize kernel weights? This can reduce edge effects and lessen the impact of data gaps (nodata) but is not suited when the kernel weights sum to zero.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true
        });
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif --weights=weights.txt -o=output.tif --center=center --normalize", short_exe, name).replace("*", &sep);
    
        UserDefinedWeightsFilter { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for UserDefinedWeightsFilter {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut weights_file = String::new();
        let mut kernel_center = "center".to_string();
        let mut normalize = false;
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters."));
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
            } else if flag_val == "-weights" {
                weights_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-center" || flag_val == "-centre" {
                kernel_center = vec[1].to_string().to_lowercase();
            } else if flag_val == "-normalize" {
                normalize = true;
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !weights_file.contains(&sep) && !weights_file.contains("/") {
            weights_file = format!("{}{}", working_directory, weights_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // read in the filter weights
        let f = File::open(weights_file.clone())?;
        let f = BufReader::new(f);
        let mut weight: f64;
        let mut num_pixels_in_filter = 0;
        let mut weights = vec![];
        let mut kernel_rows = 0;
        let mut kernel_columns = 0;
        for line in f.lines() {
            let line_unwrapped = line.unwrap();
            let mut line_split = line_unwrapped.split(",");
            let mut vec = line_split.collect::<Vec<&str>>();
            kernel_rows += 1;
            if vec.len() == 1 {
                line_split = line_unwrapped.split(" ");
                vec = line_split.collect::<Vec<&str>>();
            }
            if kernel_rows == 1 {
                kernel_columns = vec.len();
            }
            for i in 0..vec.len() {
                weight = vec[i].trim().parse::<f64>().unwrap();
                weights.push(weight);
                num_pixels_in_filter += 1;
            }
        }

        // calculate the filter offsets
        let mut d_x: Vec<isize> = Vec::with_capacity(num_pixels_in_filter);
        let mut d_y: Vec<isize> = Vec::with_capacity(num_pixels_in_filter);

        let (kernel_center_x, kernel_center_y) = match &kernel_center as &str {
            "upper-left" => (0isize, 0isize),
            "upper-right" => (0isize, kernel_columns as isize),
            "lower-left" => (kernel_rows as isize, 0isize),
            "lower-right" => (kernel_rows as isize, kernel_columns as isize),
            _ => {
                // assume 'center'

                // First make sure the filter dimensions are odd.
                // The filter dimensions must be odd numbers such that there is a middle pixel
                if (kernel_columns as f64 / 2f64).floor() == (kernel_columns as f64 / 2f64) || 
                    (kernel_rows as f64 / 2f64).floor() == (kernel_rows as f64 / 2f64) {
                    return Err(Error::new(ErrorKind::InvalidInput,
                        "The filter kernel is not an odd number of rows and columns yet the 'center' 
                        option for the kernel centre has been selected. Please modify the input 
                        kernel file."));
                }

                let midpoint_x: isize = (kernel_columns as f64 / 2f64).floor() as isize;
                let midpoint_y: isize = (kernel_rows as f64 / 2f64).floor() as isize;
                (midpoint_x, midpoint_y)
            },
        };

        // fill the filter d_x and d_y values and the distance-weights
        for row in 0..kernel_rows {
            for col in 0..kernel_columns {
                d_x.push(col as isize - kernel_center_x);
                d_y.push(row as isize - kernel_center_y);
            }
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose { println!("Reading data...") };
        let input = Arc::new(Raster::new(&input_file, "r")?);
        
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let d_x = Arc::new(d_x);
        let d_y = Arc::new(d_y);
        let weights = Arc::new(weights);
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let d_x = d_x.clone();
            let d_y = d_y.clone();
            let weights = weights.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut sum_weights, mut z_final): (f64, f64);
                let mut z: f64;
                let mut zn: f64;
                let (mut x, mut y): (isize, isize);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    if normalize {
                        for col in 0..columns {
                            z = input.get_value(row, col);
                            if z != nodata {
                                sum_weights = 0.0;
                                z_final = 0.0;
                                for a in 0..num_pixels_in_filter {
                                    x = col + d_x[a];
                                    y = row + d_y[a];
                                    zn = input.get_value(y, x);
                                    if zn != nodata {
                                        sum_weights += weights[a];
                                        z_final += weights[a] * zn;
                                    }
                                }
                                if sum_weights > 0f64 {
                                    data[col as usize] = z_final / sum_weights;
                                }
                            }
                        }
                    } else {
                        for col in 0..columns {
                            z = input.get_value(row, col);
                            if z != nodata {
                                z_final = 0.0;
                                for a in 0..num_pixels_in_filter {
                                    x = col + d_x[a];
                                    y = row + d_y[a];
                                    zn = input.get_value(y, x);
                                    if zn != nodata {
                                        z_final += weights[a] * zn;
                                    }
                                }
                                data[col as usize] = z_final;
                            }
                        }
                    }

                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Weights file: {}", weights_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => {
                if verbose { println!("Output file written") }
            },
            Err(e) => return Err(e),
        };

        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}