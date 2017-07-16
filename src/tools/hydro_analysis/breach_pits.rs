extern crate time;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::WhiteboxTool;

pub struct BreachSingleCellPits {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl BreachSingleCellPits {
    pub fn new() -> BreachSingleCellPits { // public constructor
        let name = "BreachSingleCellPits".to_string();
        
        let description = "Removes single-cell pits from an input DEM by breaching.".to_string();
        
        let mut parameters = "-i, --dem     Input raster DEM file.".to_owned();
        parameters.push_str("-o, --output  Output raster file.\n");
        
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} --wd=\"*path*to*data*\" --dem=DEM.dep -o=output.dep", short_exe, name).replace("*", &sep);
    
        BreachSingleCellPits { name: name, description: description, parameters: parameters, example_usage: usage }
    }
}

impl WhiteboxTool for BreachSingleCellPits {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" || vec[0].to_lowercase() == "--dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
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

        if !input_file.contains(&sep) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let input = Raster::new(&input_file, "r")?;

        let start = time::now();
        
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        match output.set_data_from_raster(&input) {
            Err(e) => return Err(e),
            Ok(()) => (),
        }

        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let dx2 = [ 2, 2, 2, 2, 2, 1, 0, -1, -2, -2, -2, -2, -2, -1, 0, 1 ];
        let dy2 = [ -2, -1, 0, 1, 2, 2, 2, 2, 2, 1, 0, -1, -2, -2, -2, -2 ];
        let breachcell = [ 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 0 ];
        let (mut z, mut zn): (f64, f64);
        let mut flag: bool;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    flag = true;
                    for i in 0..8 {
                        zn = input[(row + dy[i], col + dx[i])];
                        if zn < z && zn != nodata {
                            flag = false;
                            break;
                        }
                    }
                    if flag { // it's a pit cell
                        for i in 0..16 {
                            zn = input[(row + dy2[i], col + dx2[i])];
                            if zn < z && zn != nodata {
                                output[(row + dy[breachcell[i]], col + dx[breachcell[i]])] = (z + zn) / 2f64;
                            }
                        }
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

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}