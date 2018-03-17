/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: December 31 2017
Last Modified: December 31, 2017
License: MIT

NOTES: This can be used to calculate the radius of gyration (RoG) for the polygon 
features within a raster image. RoG measures how far across the landscape a polygon 
extends its reach on average, given by the mean distance between cells in a patch 
(Mcgarigal et al. 2002). The radius of gyration can be considered a measure of the 
average distance an organism can move within a patch before encountering the patch 
boundary from a random starting point (Mcgarigal et al. 2002). The input raster grid 
should contain polygons with unique identifiers greater than zero. The user must also 
specify the name of the output raster file (where the radius of gyration will be 
assigned to each feature in the input file) and the specified option of outputting text 
data.

Should be updated to output to html table instead.
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

pub struct RadiusOfGyration {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RadiusOfGyration {
    pub fn new() -> RadiusOfGyration { // public constructor
        let name = "RadiusOfGyration".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description = "Calculates the distance of cells from their polygon's centroid.".to_string();
        
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
            name: "Output text?".to_owned(), 
            flags: vec!["--text_output".to_owned()], 
            description: "Optional text output.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: false
        });
         
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=polygons.dep -o=output.dep --text_output", short_exe, name).replace("*", &sep);
    
        RadiusOfGyration { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for RadiusOfGyration {
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
        let mut text_output = false;
        
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
            if flag_val == "-i" || flag_val == "-input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i+1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i+1].to_string();
                }
            } else if flag_val == "-text_output" {
                text_output = true;
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let start = time::now();
        
        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let resolution_x = input.configs.resolution_x;
        let resolution_y = input.configs.resolution_y;

        let min_val = input.configs.minimum.floor() as usize;
        let max_val = input.configs.maximum.ceil() as usize;
        let range = max_val - min_val;
        
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut a: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut total_columns = vec![0usize; range + 1];
                    let mut total_rows = vec![0usize; range + 1];
                    let mut total_n = vec![0usize; range + 1];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z > 0f64 && z != nodata {
                            a = (z - min_val as f64) as usize;
                            total_columns[a] += col as usize;
                            total_rows[a] += row as usize;
                            total_n[a] += 1usize;
                        }
                    }
                    tx.send((total_columns, total_rows, total_n)).unwrap();
                }
            });
        }

        let mut total_columns = vec![0usize; range + 1];
        let mut total_rows = vec![0usize; range + 1];
        let mut total_n = vec![0usize; range + 1];
        
        for row in 0..rows {
            let (tc, tr, n) = rx.recv().unwrap();

            for a in 0..range+1 {
                total_columns[a] += tc[a];
                total_rows[a] += tr[a];
                total_n[a] += n[a];
            }
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut centroid_x = vec![0f64; range + 1];
        let mut centroid_y = vec![0f64; range + 1];
        for a in 0..range+1 {
            if total_n[a] > 0 {
                centroid_x[a] = total_columns[a] as f64 / total_n[a] as f64;
                centroid_y[a] = total_rows[a] as f64 / total_n[a] as f64;
            }
        }

        let centroid_x = Arc::new(centroid_x);
        let centroid_y = Arc::new(centroid_y);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let centroid_x = centroid_x.clone();
            let centroid_y = centroid_y.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut a: usize;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut gyradius = vec![0f64; range + 1];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z > 0f64 && z != nodata {
                            a = (z - min_val as f64) as usize;
                            gyradius[a] = ((col as f64 - centroid_x[a]) * resolution_x) * ((col as f64 - centroid_x[a]) * resolution_x) +
                                ((row as f64 - centroid_y[a]) * resolution_y) * ((row as f64 - centroid_y[a]) * resolution_y)
                        }
                    }
                    tx.send(gyradius).unwrap();
                }
            });
        }

        let mut gyradius = vec![0f64; range + 1];
        for row in 0..rows {
            let g = rx.recv().unwrap();
            for a in 0..range+1 {
                gyradius[a] += g[a];
            }
            
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for a in 0..range+1 {
            if total_n[a] > 0 && gyradius[a] > 0f64 {
                gyradius[a] = (gyradius[a] / total_n[a] as f64).sqrt();
            }
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut z: f64;
        let mut a: usize;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z > 0f64 && z != nodata {
                    a = (z - min_val as f64) as usize;
                    output.set_value(row, col, gyradius[a]);
                } else {
                    output.set_value(row, col, z);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (Loop 3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if text_output {
            println!("Patch Radius of Gyration\nPatch ID\tValue");
            for a in 0..range+1 {
                if total_n[a] > 0 {
                    println!("{:.0}\t{:.4}", (a + min_val), gyradius[a]);
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.configs.palette = "spectrum_black_background.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };

        if verbose {
            println!("{}", &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}