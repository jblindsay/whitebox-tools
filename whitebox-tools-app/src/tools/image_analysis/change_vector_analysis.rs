/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/04/2018
Last Modified: 29/04/2018
License: MIT
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// Change Vector Analysis (CVA) is a change detection method that characterizes the
/// magnitude and change direction in spectral space between two times. A change vector
/// is the difference vector between two vectors in n-dimensional feature space defined
/// for two observations of the same geographical location (i.e. corresponding pixels)
/// during two dates. The CVA inputs include the set of raster images corresponding to
/// the multispectral data for each date. Note that there must be the same number of
/// image files (bands) for the two dates and they must be entered in the same order,
/// i.e. if three bands, red, green, and blue are entered for date one, these same
/// bands must be entered in the same order for date two.
///
/// CVA outputs two image files. The first image contains the change vector length,
/// i.e. magnitude, for each pixel in the multi-spectral dataset. The second image
/// contains information about the direction of the change event in spectral feature
/// space, which is related to the type of change event, e.g. deforestation will likely
/// have a different change direction than say crop growth. The vector magnitude is a
/// continuous numerical variable. The change vector direction is presented in the form
/// of a code, referring to the multi-dimensional sector in which the change vector
/// occurs. A text output will be produced to provide a key describing sector codes,
/// relating the change vector to positive or negative shifts in n-dimensional feature
/// space.
///
/// It is common to apply a simple thresholding operation on the magnitude data to
/// determine 'actual' change (i.e. change above some assumed level of error). The type
/// of change (qualitatively) is then defined according to the corresponding sector code.
/// Jensen (2015) provides a useful description of this approach to change detection.
///
/// # Reference
/// Jensen, J. R. (2015). Introductory Digital Image Processing: A Remote Sensing Perspective.
///
/// # See Also
/// `WriteFunctionMemoryInsertion`
pub struct ChangeVectorAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ChangeVectorAnalysis {
    pub fn new() -> ChangeVectorAnalysis {
        // public constructor
        let name = "ChangeVectorAnalysis".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description =
            "Performs a change vector analysis on a two-date multi-spectral dataset.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Earlier Date Input Files".to_owned(),
            flags: vec!["--date1".to_owned()],
            description: "Input raster files for the earlier date.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Later Date Input Files".to_owned(),
            flags: vec!["--date2".to_owned()],
            description: "Input raster files for the later date.".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector Magnitude File".to_owned(),
            flags: vec!["--magnitude".to_owned()],
            description: "Output vector magnitude raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector Direction File".to_owned(),
            flags: vec!["--direction".to_owned()],
            description: "Output vector Direction raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --date1='d1_band1.tif;d1_band2.tif;d1_band3.tif' --date2='d2_band1.tif;d2_band2.tif;d2_band3.tif' --magnitude=mag_out.tif --direction=dir_out.tif", short_exe, name).replace("*", &sep);

        ChangeVectorAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ChangeVectorAnalysis {
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
        let mut input1_files_str = String::new();
        let mut input2_files_str = String::new();
        let mut magnitude_file = String::new();
        let mut direction_file = String::new();

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
            if flag_val == "-date1" {
                input1_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-date2" {
                input2_files_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-magnitude" {
                magnitude_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-direction" {
                direction_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
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

        let start = Instant::now();

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !magnitude_file.contains(&sep) && !magnitude_file.contains("/") {
            magnitude_file = format!("{}{}", working_directory, magnitude_file);
        }
        if !direction_file.contains(&sep) && !direction_file.contains("/") {
            direction_file = format!("{}{}", working_directory, direction_file);
        }

        let mut cmd = input1_files_str.split(";");
        let mut input1_files = cmd.collect::<Vec<&str>>();
        if input1_files.len() == 1 {
            cmd = input1_files_str.split(",");
            input1_files = cmd.collect::<Vec<&str>>();
        }
        let num_files = input1_files.len();
        if num_files == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "At least one input for each date are required to operate this tool.",
            ));
        }

        cmd = input2_files_str.split(";");
        let mut input2_files = cmd.collect::<Vec<&str>>();
        if input2_files.len() == 1 {
            cmd = input2_files_str.split(",");
            input2_files = cmd.collect::<Vec<&str>>();
        }
        if input2_files.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "At least one input for each date are required to operate this tool.",
            ));
        }

        if num_files != input2_files.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "There must be the same number of input files for each date.",
            ));
        }

        let mut direction_array: Vec<f64> = Vec::with_capacity(num_files);
        for i in 0..num_files {
            direction_array.push(2f64.powf(i as f64));
        }

        // We will need to read one of the files in to get the rows, columns, and nodata values
        // in order to create the output files.
        let mut input1_file = input1_files[0].trim().to_owned();
        if !input1_file.contains(&sep) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        let input1 = Raster::new(&input1_file, "r")?;
        let rows = input1.configs.rows as isize;
        let columns = input1.configs.columns as isize;
        let nodata = input1.configs.nodata;

        // Create the output files
        let mut out_magnitude = Raster::initialize_using_file(&magnitude_file, &input1);
        out_magnitude.configs.data_type = DataType::F32;
        let mut out_direction = Raster::initialize_using_file(&direction_file, &input1);
        out_direction.reinitialize_values(0f64);

        let mut nodata_detected: Array2D<i8> = Array2D::new(rows, columns, -1i8, -1i8)?;
        // let mut num_procs = num_cpus::get() as isize;
        // let configs = whitebox_common::configs::get_configs()?;
        // let max_procs = configs.max_procs;
        // if max_procs > 0 && max_procs < num_procs {
        //     num_procs = max_procs;
        // }

        for i in 0..num_files {
            if verbose {
                println!("Reading file {} of {}", i + 1, num_files);
            }
            if !input1_files[i].trim().is_empty() && !input2_files[i].trim().is_empty() {
                let mut input1_file = input1_files[i].trim().to_owned();
                if !input1_file.contains(&sep) && !input1_file.contains("/") {
                    input1_file = format!("{}{}", working_directory, input1_file);
                }

                let mut input2_file = input2_files[i].trim().to_owned();
                if !input2_file.contains(&sep) && !input2_file.contains("/") {
                    input2_file = format!("{}{}", working_directory, input2_file);
                }

                let input1 = Raster::new(&input1_file, "r")?;
                let input2 = Raster::new(&input2_file, "r")?;

                // make sure the images have the right rows and columns
                if input1.configs.rows as isize != rows
                    || input1.configs.columns as isize != columns
                    || input2.configs.rows as isize != rows
                    || input1.configs.columns as isize != columns
                {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "All of the input files must share the same extent (rows and columns).",
                    ));
                }

                let nodata1 = input1.configs.nodata;
                let nodata2 = input2.configs.nodata;

                let (mut z1, mut z2): (f64, f64);
                let mut z: f64;
                for row in 0..rows {
                    for col in 0..columns {
                        z1 = input1.get_value(row, col);
                        z2 = input2.get_value(row, col);
                        if z1 != nodata1 && z2 != nodata2 {
                            z = z2 - z1;
                            out_magnitude.increment(row, col, z * z);
                            if z >= 0f64 {
                                out_direction.increment(row, col, direction_array[i]);
                            }
                        } else {
                            nodata_detected.set_value(row, col, 1i8);
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
            }
        }

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                if nodata_detected.get_value(row, col) < 0 {
                    z = out_magnitude.get_value(row, col);
                    out_magnitude.set_value(row, col, z.sqrt());
                } else {
                    out_magnitude.set_value(row, col, nodata);
                    out_direction.set_value(row, col, nodata);
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

        if verbose {
            println!("Saving data...")
        };

        let elapsed_time = get_formatted_elapsed_time(start);
        out_magnitude.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        out_magnitude.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

        let _ = match out_magnitude.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        out_direction.configs.photometric_interp = PhotometricInterpretation::Categorical;
        out_direction.configs.palette = String::from("qual.plt");
        out_direction.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        out_direction.add_metadata_entry(format!("Elapsed Time (including I/O): {}", elapsed_time));

        let _ = match out_direction.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        // print out a key for interpreting the direction image
        let mut s = "Key For Interpreting The CVA Direction Image:\n\n\tDirection of Change (+ or -)\nValue".to_string();
        for i in 0..num_files {
            s.push_str(&format!("\tBand{}", i + 1));
        }
        s.push_str("\n");
        let mut line: String;
        for a in 0..(2u32 * 2u32.pow(num_files as u32 - 1u32)) as usize {
            line = format!("{}\t", a);
            for i in 0..num_files {
                if a >> i & 1usize == 1usize {
                    line.push_str("+\t");
                } else {
                    line.push_str("-\t");
                }
            }
            s.push_str(&format!("{}\n", line));
        }

        println!("{}", s);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
