/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 26/06/2017
Last Modified: 24/02/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to calculate a normalized difference index (NDI) from two bands of multispectral image data.
/// A NDI of two band images (`image1` and `image2`) takes the general form:
///
/// > NDI = (image1 - image2) / (image1 + image2 + *c*)
///
/// Where *c* is a correction factor sometimes used to avoid division by zero. It is, however, often set to 0.0. In fact,
/// the `NormalizedDifferenceIndex` tool will set all pixels where `image1 + image2 = 0` to 0.0 in the output image. While
/// this is not strictly mathematically correct (0 / 0 = infinity), it is often the intended output in these cases.
///
/// NDIs generally takes the value range -1.0 to 1.0, although in practice the range of values for a particular image scene
/// may be more restricted than this.
///
/// NDIs have two important properties that make them particularly useful for remote sensing applications. First, they
/// emphasize certain aspects of the shape of the spectral signatures of different land covers. Secondly, they can be
/// used to de-emphasize the effects of variable illumination within a scene. NDIs are therefore frequently used in the
/// field of remote sensing to create vegetation indices and other indices for emphasizing various land-covers and as inputs
/// to analytical operations like image classification. For example, the normalized difference vegetation index (NDVI),
/// one of the most common image-derived products in remote sensing, is calculated as:
///
/// > NDVI = (NIR - RED) / (NIR + RED)
///
/// The optimal soil adjusted vegetation index (OSAVI) is:
///
/// > OSAVI = (NIR - RED) / (NIR + RED + 0.16)
///
/// The normalized difference water index (NDWI), or normalized difference moisture index (NDMI), is:
///
/// > NDWI = (NIR - SWIR) / (NIR + SWIR)
///
/// The normalized burn ratio 1 (NBR1) and normalized burn ration 2 (NBR2) are:
///
/// > NBR1 = (NIR - SWIR2) / (NIR + SWIR2)
/// >
/// > NBR2 = (SWIR1 - SWIR2) / (SWIR1 + SWIR2)
///
/// In addition to NDIs, *Simple Ratios* of image bands, are also commonly used as inputs to other remote sensing
/// applications like image classification. Simple ratios can be calculated using the `Divide` tool. Division by zero,
/// in this case, will result in an output NoData value.
///
/// # See Also
/// `Divide`
pub struct NormalizedDifferenceIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NormalizedDifferenceIndex {
    pub fn new() -> NormalizedDifferenceIndex {
        // public constructor
        let name = "NormalizedDifferenceIndex".to_string();
        let toolbox = "Image Processing Tools".to_string();
        let description = "Calculate a normalized-difference index (NDI) from two bands of multispectral image data.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input 1 File".to_owned(),
            flags: vec!["--input1".to_owned()],
            description: "Input image 1 (e.g. near-infrared band).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input 2 File".to_owned(),
            flags: vec!["--input2".to_owned()],
            description: "Input image 2 (e.g. red band).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Distribution Tail Clip Amount (%)".to_owned(),
            flags: vec!["--clip".to_owned()],
            description: "Optional amount to clip the distribution tails by, in percent."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Correction value".to_owned(),
            flags: vec!["--correction".to_owned()],
            description: "Optional adjustment value (e.g. 1, or 0.16 for the optimal soil adjusted vegetation index, OSAVI)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        // parameters.push(ToolParameter{
        //     name: "Use the optimized soil-adjusted veg index (OSAVI)?".to_owned(),
        //     flags: vec!["--osavi".to_owned()],
        //     description: "Optional flag indicating whether the optimized soil-adjusted veg index (OSAVI) should be used.".to_owned(),
        //     parameter_type: ParameterType::Boolean,
        //     default_value: None,
        //     optional: true
        // });

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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1=band4.tif --input2=band3.tif -o=output.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --input1=band4.tif --input2=band3.tif -o=output.tif --clip=1.0 --adjustment=0.16", short_exe, name).replace("*", &sep);

        NormalizedDifferenceIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NormalizedDifferenceIndex {
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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input1_file = String::new();
        let mut input2_file = String::new();
        let mut output_file = String::new();
        let mut clip_amount = 0.0;
        // let mut osavi_mode = false;
        let mut correction_factor = 0.0;
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
            if flag_val == "-input1" {
                input1_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-input2" {
                input2_file = if keyval {
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
            } else if flag_val == "-clip" {
                clip_amount = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
                if clip_amount < 0.0 {
                    clip_amount = 0.0;
                } else if clip_amount > 30.0 {
                    clip_amount = 30.0;
                }
            } else if flag_val == "-correction" {
                correction_factor = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            }
            // } else if flag_val == "-osavi" {
            // if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
            //     osavi_mode = true;
            //     correction_factor = 0.16;
            // }
            // }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input1_file.contains(&sep) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let nir = Arc::new(Raster::new(&input1_file, "r")?);
        let rows = nir.configs.rows as isize;
        let columns = nir.configs.columns as isize;
        let nir_nodata = nir.configs.nodata;

        let red = Arc::new(Raster::new(&input2_file, "r")?);
        let red_nodata = red.configs.nodata;

        // make sure the input files have the same size
        if nir.configs.rows != red.configs.rows || nir.configs.columns != red.configs.columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &nir);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let nir = nir.clone();
            let red = red.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut z_nir, mut z_red): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nir_nodata; columns as usize];
                    for col in 0..columns {
                        z_nir = nir[(row, col)];
                        z_red = red[(row, col)];
                        if z_nir != nir_nodata && z_red != red_nodata {
                            if z_nir + z_red != 0.0 || correction_factor > 0f64 {
                                data[col as usize] =
                                    (z_nir - z_red) / (z_nir + z_red + correction_factor);
                            } else {
                                data[col as usize] = 0f64;
                            }
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
            if verbose {
                println!("Clipping output...");
            }
            output.clip_min_and_max_by_percent(clip_amount);
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.data_type = DataType::F32;
        // output.configs.display_max = 1f64;
        // output.configs.display_min = -1f64;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("NIR file: {}", input1_file));
        output.add_metadata_entry(format!("Red file: {}", input2_file));
        output.add_metadata_entry(format!("Adjustment value: {}", correction_factor));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
