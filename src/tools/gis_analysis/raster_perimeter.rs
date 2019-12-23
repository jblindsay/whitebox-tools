/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/12/2019
Last Modified: 18/12/2019
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

/// This tool can be used to measure the length of the perimeter of polygon features in a raster layer. The user must 
/// specify the name of the input raster file (`--input`) and optionally an output raster (`--output`), which is the 
/// raster layer containing the input features assigned the perimeter length. The user may also optionally choose to output text 
/// data (`--out_text`). Raster-based perimeter estimation uses the accurate, anti-aliasing algorithm of 
/// Prashker (2009).
/// 
/// The input file must be of a categorical data type, containing discrete polygon features that have been assigned unique identifiers.
/// Such rasters are often created by region-grouping (`Clump`) a classified raster.
/// 
/// # Reference
/// 
/// Prashker, S. (2009) An anti-aliasing algorithm for calculating the perimeter of raster polygons. Geotec, Ottawa and 
/// Geomtics Atlantic, Wolfville, NS.
/// 
/// # See Also
/// `RasterArea`, `Clump`
pub struct RasterPerimeter {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterPerimeter {
    pub fn new() -> RasterPerimeter {
        // public constructor
        let name = "RasterPerimeter".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Calculates the perimeters of polygons or classes within a raster image.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
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
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output text?".to_owned(),
            flags: vec!["--out_text".to_owned()],
            description: "Would you like to output polygon areas to text?"
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Units".to_owned(), 
            flags: vec!["--units".to_owned()], 
            description: "Area units; options include 'grid cells' and 'map units'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["grid cells".to_owned(), "map units".to_owned()]),
            default_value: Some("grid cells".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter {
            name: "Treat zero values as background?".to_owned(),
            flags: vec!["--zero_back".to_owned()],
            description: "Flag indicating whether zero values should be treated as a background."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: false,
        });

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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --out_text --units='grid cells' --zero_back",
            short_exe, name
        )
        .replace("*", &sep);

        RasterPerimeter {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterPerimeter {
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
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut output_raster = false;
        let mut zero_back = false;
        let mut is_grid_cell_units = false;
        let mut output_text = false;

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
                output_raster = true;
            } else if flag_val == "-units" {
                is_grid_cell_units = if keyval {
                    vec[1].to_string().to_lowercase().contains("cells")
                } else {
                    args[i + 1].to_string().to_lowercase().contains("cells")
                };
            } else if flag_val == "-zero_back" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    zero_back = true;
                }
            } else if flag_val == "-out_text" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    output_text = true;
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_raster && !output_text {
            println!("Warning: Niether a raster nor text outputs were selected. An area raster will be generated.");
            output_file = String::from("area.tif");
            output_raster = true;
        }

        if output_raster {
            if !output_file.contains(&sep) && !output_file.contains("/") {
                output_file = format!("{}{}", working_directory, output_file);
            }
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let lut = [4.000000000f64, 2.828427125, 2.236067977, 2.414213562, 2.828427125, 3.000000000,
            2.414213562, 2.236067977, 2.236067977, 2.414213562, 2.000000000,
            2.000000000, 2.828427125, 1.414213562, 1.414213562, 1.414213562,
            2.236067977, 2.828427125, 2.000000000, 1.414213562, 2.414213562,
            1.414213562, 2.000000000, 1.414213562, 2.000000000, 2.000000000,
            1.000000000, 2.000000000, 2.000000000, 2.000000000, 2.000000000,
            1.000000000, 2.828427125, 3.000000000, 2.828427125, 1.414213562,
            2.000000000, 4.000000000, 2.236067977, 2.236067977, 2.414213562,
            2.236067977, 1.414213562, 1.414213562, 2.236067977, 2.236067977,
            1.414213562, 1.414213562, 2.828427125, 2.236067977, 1.414213562,
            1.414213562, 2.236067977, 2.414213562, 2.000000000, 1.414213562, 2.000000000, 2.000000000, 1.000000000,
            1.414213562, 2.000000000, 2.000000000, 1.000000000, 1.000000000, 2.236067977, 2.828427125, 2.000000000,
            2.000000000, 2.828427125, 2.236067977, 2.000000000, 2.000000000, 2.000000000, 1.414213562, 1.000000000,
            2.000000000, 1.414213562, 1.414213562, 1.000000000, 1.414213562, 2.000000000, 1.414213562,
            1.000000000, 1.000000000, 1.414213562, 1.414213562, 2.000000000, 1.414213562, 1.000000000, 1.000000000,
            0.000000000, 0.000000000, 1.000000000, 1.000000000, 0.000000000, 0.000000000, 2.414213562, 1.414213562,
            2.000000000, 2.000000000, 2.236067977, 2.414213562, 2.000000000, 2.000000000, 2.000000000, 1.414213562,
            2.000000000, 1.000000000, 2.000000000, 1.414213562, 1.000000000, 1.000000000, 1.414213562, 1.414213562,
            1.000000000, 1.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000, 2.000000000, 1.414213562,
            0.000000000, 0.000000000, 1.000000000, 1.000000000, 0.000000000, 0.000000000, 2.828427125, 2.000000000,
            2.828427125, 2.236067977, 3.000000000, 4.000000000, 1.414213562, 2.236067977,
            2.828427125, 2.236067977, 1.414213562, 2.000000000, 2.236067977, 2.414213562, 1.414213562, 1.414213562,
            2.414213562, 2.236067977, 1.414213562, 1.414213562, 2.236067977, 2.236067977, 1.414213562, 1.414213562,
            2.000000000, 2.000000000, 1.000000000, 1.000000000, 2.000000000, 2.000000000, 1.414213562, 1.000000000,
            3.000000000, 4.000000000, 2.236067977, 2.414213562, 4.000000000, 4.000000000, 2.414213562, 2.236067977,
            1.414213562, 2.236067977, 1.414213562, 1.414213562, 2.414213562, 2.236067977, 1.414213562, 1.414213562,
            1.414213562, 2.414213562, 1.414213562, 1.414213562, 2.236067977, 2.236067977,
            1.414213562, 1.414213562, 2.000000000, 2.000000000, 1.000000000, 1.000000000, 2.000000000, 2.000000000,
            1.000000000, 1.000000000, 2.414213562, 2.000000000, 2.236067977, 2.000000000, 1.414213562, 2.414213562,
            2.000000000, 2.000000000, 1.414213562, 1.414213562, 1.000000000, 1.000000000, 1.414213562, 1.414213562,
            1.000000000, 1.000000000, 2.000000000, 2.000000000, 2.000000000, 1.000000000, 1.414213562, 1.414213562,
            1.000000000, 1.000000000, 2.000000000, 1.000000000, 0.000000000, 0.000000000, 1.414213562, 1.000000000,
            0.000000000, 0.000000000, 2.236067977, 2.236067977, 2.000000000, 2.000000000, 2.236067977, 2.236067977,
            2.000000000, 2.000000000, 1.414213562, 1.414213562, 1.414213562, 1.000000000, 1.414213562, 1.414213562,
            1.000000000, 1.000000000, 1.414213562, 1.414213562, 1.414213562, 1.000000000, 1.414213562, 1.414213562,
            1.000000000, 1.000000000, 1.000000000, 1.000000000, 0.000000000, 0.000000000, 1.000000000, 1.000000000,
            0.000000000, 0.000000000];
        
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let v = [1usize, 2, 4, 8, 16, 32, 64, 128];
        // let (mut i, mut j): (isize, isize);
        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let resx = input.configs.resolution_x;
        let resy = input.configs.resolution_y;
        let avg_res = (resx + resy) / 2f64;
        let min_val = input.configs.display_min;
        let max_val = input.configs.display_max;
        let range = max_val - min_val + 0.00001f64; // otherwise the max value is outside the range
        let num_bins = range.ceil() as usize;
        let back_val = if zero_back {
            0f64
        } else {
            nodata
        };

        let is_geographic = input.is_in_geographic_coordinates();
        if is_geographic && verbose {
            println!("Warning: the input file does not appear to be in a projected coordinate system. Perimeter values will only be estimates.");
        }

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut resx = input.configs.resolution_x;
                let mut resy = input.configs.resolution_y;
                let mut data = vec![0f64; num_bins];
                let mut val: f64;
                let mut val2: usize;
                let mut bin: usize;
                let (mut i, mut j): (isize, isize);
                let mut mid_lat: f64;
                let mut res: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    if is_geographic {
                        mid_lat = input.get_y_from_row(row).to_radians();
                        resx = resx * 111_111.0 * mid_lat.cos();
                        resy = resy * 111_111.0;
                    }
                    res = (resx + resy) / 2f64;
                    for col in 0..columns {
                        val = input.get_value(row, col);
                        if val != nodata && val != back_val && val >= min_val && val <= max_val {
                            bin = (val - min_val).floor() as usize;
                            val2 = 0;
                            for n in 0..8 {
                                i = col + dx[n];
                                j = row + dy[n];
                                if input.get_value(j, i) == val {
                                    val2 += v[n];
                                }
                            }
                            data[bin] += if !is_grid_cell_units { 
                                lut[val2]
                            } else {
                                lut[val2] * res
                            };
                        }
                    }
                }
                tx.send(data).unwrap();
            });
        }

        let mut data = vec![0f64; num_bins];
        for tid in 0..num_procs {
            let data_rx = rx.recv().unwrap();
            for a in 0..num_bins {
                data[a] += data_rx[a] * avg_res;
            }

            if verbose {
                progress = (100.0_f64 * (tid + 1) as f64 / num_procs as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut val: f64;
        let mut bin: usize;  
        if output_raster {
            let mut output = Raster::initialize_using_file(&output_file, &input);
            let out_nodata = -999f64;
            output.reinitialize_values(out_nodata);
            output.configs.nodata = out_nodata;
            output.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output.configs.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..columns {
                    val = input.get_value(row, col);
                    if val != nodata && val != back_val && val >= min_val && val <= max_val {
                        bin = (val - min_val).floor() as usize;
                        output.set_value(row, col, data[bin] as f64);
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Outputting raster: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time = get_formatted_elapsed_time(start);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("Input file: {}", input_file));
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
        }
        if output_text {
            println!("Class,Perimeter");
            for a in 0..num_bins {
                if data[a] > 0f64 {
                    val = (a as f64 + min_val).floor();
                    println!("{},{}", val, data[a]);
                }
            }
        }
        
        Ok(())
    }
}
