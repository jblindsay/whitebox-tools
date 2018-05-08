/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 07/05/2018
Last Modified: 07/05/2018
License: MIT

HELP:

This tool performs a weighted overlay on multiple input images. It can be used to 
combine multiple factors with varying levels of weight or relative importance. The 
WeightedOverlay tool is similar to the WeightedSum tool but is more powerful because 
it automatically converts the input factors to a common user-defined scale and allows 
the user to specify benefit factors and cost factors. A benefit factor is a factor 
for which higher values are more suitable. A cost factor is a factor for which higher 
values are less suitable. By default, WeightedOverlay assumes that input images are 
benefit factors, unless a cost value of 'true' is entered in the cost array. 
Constraints are absolute restriction with values of 0 (unsuitable) and 1 (suitable). 
This tool is particularly useful for performing multi-criteria evaluations (MCE).

Notice that the algorithm will convert the user-defined factor weights internally such 
that the sum of the weights is always equal to one. As such, the user can specify the 
relative weights as decimals, percentages, or relative weightings (e.g. slope is 2 times 
more important than elevation, in which case the weights may not sum to 1 or 100).

NoData valued grid cells in any of the input images will be assigned NoData values in 
the output image. The output raster is of the float data type and continuous data scale.

*/

use time;
use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;

pub struct WeightedOverlay {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl WeightedOverlay {
    pub fn new() -> WeightedOverlay { // public constructor
        let name = "WeightedOverlay".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Performs a weighted sum on multiple input rasters after converting each image to a common scale. The tool performs a multi-criteria evaluation (MCE).".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input Factor Files".to_string(), 
            flags: vec!["--factors".to_string()], 
            description: "Input factor raster files.".to_string(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Weight Values (e.g. 1.7;3.5;1.2)".to_string(), 
            flags: vec!["-w".to_owned(), "--weights".to_string()], 
            description: "Weight values, contained in quotes and separated by commas or semicolons. Must have the same number as factors.".to_string(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Cost Factor? (e.g. false;true;true)".to_string(), 
            flags: vec!["--cost".to_string()], 
            description: "Weight values, contained in quotes and separated by commas or semicolons. Must have the same number as factors.".to_string(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Input Constraints Files".to_string(), 
            flags: vec!["--constraints".to_string()], 
            description: "Input constraints raster files.".to_string(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Output File".to_string(), 
            flags: vec!["-o".to_string(), "--output".to_string()], 
            description: "Output raster file.".to_string(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Suitability Scale Maximum".to_owned(), 
            flags: vec!["--scale_max".to_owned()], 
            description: "Suitability scale maximum value (common values are 1.0, 100.0, and 255.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd='*path*to*data*' --factors='image1.tif;image2.tif;image3.tif' --weights='0.3;0.2;0.5' --cost='false;false;true' -o=output.tif --scale_max=100.0", short_exe, name).replace("*", &sep);
    
        WeightedOverlay { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for WeightedOverlay {
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
        let mut weights_list = String::new();
        let mut cost_list = String::new();
        let mut constraint_files = String::new();
        let mut output_file = String::new();
        let mut scale_max = 1f64;
        
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
            if flag_val == "-factors" {
                input_files = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-w" || flag_val == "-weights" {
                weights_list = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-cost" {
                cost_list = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-constraints" {
                constraint_files = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-scale_max" {
                scale_max = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
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

        if !output_file.contains(&sep) && !output_file.contains("/") {
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

        // Parse the weights list and convert it into numbers
        if weights_list.is_empty() { // assume they are all benefit factors
            for _ in 0..num_files-1 {
                weights_list.push_str("false;");
            }
            weights_list.push_str("false");
        }
        cmd = weights_list.split(";");
        let mut weights_str = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = weights_list.split(",");
            weights_str = cmd.collect::<Vec<&str>>();
        }
        let num_weights = weights_str.len();
        if num_weights != num_files {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The number of weights specified must equal the number of factors."));
        }
        let mut weights = vec![];
        for w in weights_str {
            weights.push(w.to_string().parse::<f64>().unwrap());
        }

        // make sure that the weights sum to 1.0
        let mut weight_sum = 0.0f64;
        for i in 0..num_weights {
            weight_sum += weights[i];
        }
        for i in 0..num_weights {
            weights[i] /= weight_sum;
        }

        // Parse the cost list and convert it into booleans
        cmd = cost_list.split(";");
        let mut cost_str = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = cost_list.split(",");
            cost_str = cmd.collect::<Vec<&str>>();
        }
        let num_costs = cost_str.len();
        if num_costs != num_files {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "The number of cost values specified must equal the number of factors."));
        }
        let mut cost = vec![];
        for c in cost_str {
            if c.to_lowercase().contains("t") {
                cost.push(true);
            } else {
                cost.push(false);
            }
        }

        // We need to initialize output here, but in reality this can't be done
        // until we know the size of rows and columns, which occurs during the first loop.
        let mut output: Raster = Raster::new(&output_file, "w")?;
        let mut rows = 0isize;
        let mut columns = 0isize;
        let mut in_nodata: f64;
        let mut out_nodata: f64 = -32768.0f64;
        let mut in_val: f64;
        let mut min_val: f64;
        let mut range: f64;
        let mut read_first_file = false;
        let mut i = 1;
        let mut j = 0usize;
        for value in vec {
            if !value.trim().is_empty() {
                if verbose { println!("Reading data...") };

                let mut input_file = value.trim().to_string();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;
                in_nodata = input.configs.nodata;
                if !read_first_file {
                    read_first_file = true;
                    rows = input.configs.rows as isize;
                    columns = input.configs.columns as isize;
                    out_nodata = in_nodata;
                    
                    // initialize the output file and low_val
                    output = Raster::initialize_using_file(&output_file, &input);
                    output.reinitialize_values(0.0);
                }
                // check to ensure that all inputs have the same rows and columns
                if input.configs.rows as isize != rows || input.configs.columns as isize != columns {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
                }

                min_val = input.configs.minimum;
                range = input.configs.maximum - min_val;

                for row in 0..rows {
                    for col in 0..columns {
                        if output.get_value(row, col) != out_nodata {
                            in_val = input.get_value(row, col);
                            if in_val != in_nodata {
                                in_val = (in_val - min_val) / range;
                                if cost[j] { in_val = 1.0 - in_val; }
                                in_val *= scale_max;
                                output.increment(row, col, in_val * weights[j]);
                            } else {
                                output.set_value(row, col, out_nodata);
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Processing Factors (loop {} of {}): {}%", i, num_files, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
            j += 1;
        }

        // now deal with the constraints
        cmd = constraint_files.split(";");
        let mut constraint_file_names = cmd.collect::<Vec<&str>>();
        if constraint_file_names.len() == 1 {
            cmd = constraint_files.split(",");
            constraint_file_names = cmd.collect::<Vec<&str>>();
        }
        let num_constraints = constraint_file_names.len();
        i = 1;
        j = 0usize;
        for value in constraint_file_names {
            if !value.trim().is_empty() {
                if verbose { println!("Reading data...") };

                let mut input_file = value.trim().to_string();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }
                let input = Raster::new(&input_file, "r")?;
                in_nodata = input.configs.nodata;
                // check to ensure that all inputs have the same rows and columns
                if input.configs.rows as isize != rows || input.configs.columns as isize != columns {
                    return Err(Error::new(ErrorKind::InvalidInput,
                                "The input files must have the same number of rows and columns and spatial extent."));
                }

                for row in 0..rows {
                    for col in 0..columns {
                        in_val = input.get_value(row, col);
                        if in_val != in_nodata && in_val <= 0f64 {
                            if output.get_value(row, col) != out_nodata {
                                output.set_value(row, col, 0f64);
                            }
                        } else if in_val == in_nodata {
                            output.set_value(row, col, out_nodata);
                        } // else it stays unaltered
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Processing Constraints (loop {} of {}): {}%", i, num_constraints, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
            j += 1;
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

        if verbose {
            println!("{}", &format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}