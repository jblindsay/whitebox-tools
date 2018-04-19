/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: June 27, 2017
Last Modified: 19/04/2018
License: MIT
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

pub struct EmbossFilter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EmbossFilter {
    pub fn new() -> EmbossFilter { // public constructor
        let name = "EmbossFilter".to_string();
        let toolbox = "Image Processing Tools/Filters".to_string();
        let description = "Performs an emboss filter on an image, similar to a hillshade operation.".to_string();
        
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
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });
        
        parameters.push(ToolParameter{
            name: "Direction".to_owned(), 
            flags: vec!["--direction".to_owned()], 
            description: "Direction of reflection; options include 'n', 's', 'e', 'w', 'ne', 'se', 'nw', 'sw'".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["n".to_owned(), "s".to_owned(), "e".to_owned(), "w".to_owned(), "ne".to_owned(), "se".to_owned(), "nw".to_owned(), "sw".to_owned()]),
            default_value: Some("n".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Percent to clip the distribution tails".to_owned(), 
            flags: vec!["--clip".to_owned()], 
            description: "Optional amount to clip the distribution tails by, in percent.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=image.tif -o=output.tif --direction='s' --clip=1.0", short_exe, name).replace("*", &sep);
    
        EmbossFilter { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for EmbossFilter {
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
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters."));
        }
        
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut direction = "n".to_string();
        let mut clip_amount = 0.0;
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
            } else if flag_val == "-direction" {
                direction = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                direction = direction.to_lowercase();
            } else if flag_val == "-clip" {
                clip_amount = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
                if clip_amount < 0.0 { clip_amount == 0.0; }
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
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        
        let start = time::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
    
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let direction = direction.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let weights = match &direction as &str {
                    "n" => [ 0f64, -1f64, 0f64, 0f64, 0f64, 0f64, 0f64, 1f64, 0f64 ],
                    "s" => [ 0f64, 1f64, 0f64, 0f64, 0f64, 0f64, 0f64, -1f64, 0f64 ],
                    "e" => [ 0f64, 0f64, 0f64, 1f64, 0f64, -1f64, 0f64, 0f64, 0f64 ],
                    "w" => [ 0f64, 0f64, 0f64, -1f64, 0f64, 1f64, 0f64, 0f64, 0f64 ],
                    "ne" => [ 0f64, 0f64, -1f64, 0f64, 0f64, 0f64, 1f64, 0f64, 0f64 ],
                    "nw" => [ -1f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 1f64 ],
                    "se" => [ 1f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, -1f64 ],
                    _ => [ 0f64, 0f64, 1f64, 0f64, 0f64, 0f64, -1f64, 0f64, 0f64 ], // sw
                };
                let dx = [ -1, 0, 1, -1, 0, 1, -1, 0, 1 ];
                let dy = [ -1, -1, -1, 0, 0, 0, 1, 1, 1 ];
                let num_pixels_in_filter = dx.len();
                let mut sum: f64;
                let mut z: f64;
                let mut zn: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            sum = 0.0;
                            for n in 0..num_pixels_in_filter {
                                zn = input[(row + dy[n], col + dx[n])];
                                if zn == nodata { zn = z; }
                                sum += zn * weights[n];
                            }
                            data[col as usize] = sum;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

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

        if clip_amount > 0.0 {
            println!("Clipping output...");
            output.clip_min_and_max_by_percent(clip_amount);
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Direction: {}", direction));
        output.add_metadata_entry(format!("Clip amount: {}", clip_amount));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

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
            println!("{}",
                    &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}