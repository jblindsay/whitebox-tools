extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct PickFromList {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl PickFromList {
    pub fn new() -> PickFromList { // public constructor
        let name = "PickFromList".to_string();
        
        let description = "Outputs the value from a raster stack specified by a position raster.".to_string();
        
        let mut parameters = "-i, --inputs     Input raster files, contained in quotes and separated by commas or semicolons.\n".to_owned();
        parameters.push_str("--pos_input      Input position raster file.\n");
        parameters.push_str("-o, --output     Output raster file.\n");
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd='*path*to*data*' --pos_input=position.dep -i='image1.dep;image2.dep;image3.dep' -o=output.dep", short_exe, name).replace("*", &sep);
    
        PickFromList { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for PickFromList {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_files = String::new();
        let mut output_file = String::new();
        let mut pos_file = String::new();
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--inputs" {
                if keyval {
                    input_files = vec[1].to_string();
                } else {
                    input_files = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-pos_input" || vec[0].to_lowercase() == "--pos_input" {
                if keyval {
                    pos_file = vec[1].to_string();
                } else {
                    pos_file = args[i+1].to_string();
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

        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        let start = time::now();

        if !pos_file.contains(&sep) {
            pos_file = format!("{}{}", working_directory, pos_file);
        }

        // read in the position file
        let position = Raster::new(&pos_file, "r")?;
        let rows = position.configs.rows as isize;
        let columns = position.configs.columns as isize;
        
        // initialize the output file
        let mut output = Raster::initialize_using_file(&output_file, &position);

        let mut in_val: f64;
        let mut i = 1;
        let mut j = 0f64;
        for value in vec {
            if !value.trim().is_empty() {
                if verbose { println!("Reading data...") };

                let mut input_file = value.trim().to_owned();
                if !input_file.contains(&sep) {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;
                let in_nodata = input.configs.nodata;
                
                // check to ensure that all inputs have the same rows and columns
                if input.configs.rows as isize != rows || input.configs.columns as isize != columns {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
                }

                for row in 0..rows {
                    for col in 0..columns {
                        if position[(row, col)] == j {
                            in_val = input[(row, col)];
                            if in_val != in_nodata {
                                output[(row, col)] = in_val;
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress (loop {} of {}): {}%", i, num_files, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
            j += 1f64;
        }
        
        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}