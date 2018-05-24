/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: January 2, 2018
Last Modified: January 2, 2018
License: MIT

Notes: This tool can be used to create a random sample of grid cells. The user specifies 
the base raster file, which is used to determine the grid dimensions and georeference 
information for the output raster, and the number of sample random samples (n). The 
output grid will contain n non-zero grid cells, randomly distributed throughout the 
raster grid, and a background value of zero. This tool is useful when performing 
statistical analyses on raster images when you wish to obtain a random sample of data.

Only valid, non-nodata, cells in the base raster will be sampled.
*/

use time;
use rand::prelude::*;
use std::env;
use std::path;
use std::f64;
use std::io::{Error, ErrorKind};
use raster::*;
use tools::*;

pub struct RandomSample {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RandomSample {
    pub fn new() -> RandomSample {
        // public constructor
        let name = "RandomSample".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Creates an image containing randomly located sample grid cells with unique IDs."
            .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input Base File".to_owned(), 
            flags: vec!["-i".to_owned(), "--base".to_owned()], 
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
            name: "Num. Samples".to_owned(), 
            flags: vec!["--num_samples".to_owned()], 
            description: "Number of samples".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1000".to_string()),
            optional: false
        });
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --base=in.tif -o=out.tif --num_samples=1000", short_exe, name).replace("*", &sep);

        RandomSample {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RandomSample {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut num_samples = 1000usize;
        
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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-base" {
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
            } else if flag_val == "-num_samples" {
                num_samples = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap() as usize
                };
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

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        if num_samples > (rows * columns) as usize {
            return Err(Error::new(ErrorKind::InvalidInput,
                "num_samples is too large for the size of grid."));
            // This could still be a problem because of nodata in the input grid.
            // Only valid grid cells in the input will have samples.
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.reinitialize_values(0f64);

        let mut rng = thread_rng();
        // let row_rng = Range::new(0, rows as isize);
        // let col_rng = Range::new(0, columns as isize);
        let mut sample_num = 0usize;
        let mut num_tries = 0usize;
        while sample_num < num_samples {
            let row = rng.gen_range(0, rows as isize); //row_rng.ind_sample(&mut rng);
            let col = rng.gen_range(0, columns as isize); //col_rng.ind_sample(&mut rng);
            if output.get_value(row, col) == 0f64 && input.get_value(row, col) != nodata {
                sample_num += 1;
                output.set_value(row, col, sample_num as f64);

                if verbose {
                    progress = (100.0_f64 * sample_num as f64 / num_samples as f64) as i32;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
            num_tries += 1;
            if num_tries >= 20 * num_samples {
                println!("Warning: num_samples may be too large for the size of grid.");
                break;
            }
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.configs.data_type = DataType::F32;
        output.configs.palette = "qual.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input base raster file: {}", input_file));
        output.add_metadata_entry(format!("Num. samples: {}", num_samples));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            },
            Err(e) => return Err(e),
        };
        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}
