/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/02/2019
Last Modified: 18/10/2019
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

/// This tools estimates the area of each category, polygon, or patch in an input raster. The input raster must be categorical 
/// in data scale. Rasters with floating-point cell values are not good candidates for an area analysis. The user must specify 
/// whether the output is given in `grid cells` or `map units` (`--units`). Map Units are physical units, e.g. if the rasters's 
/// scale is in metres, areas will report in square-metres. Notice that square-metres can be converted into hectares by dividing 
/// by 10,000 and into square-kilometres by dividing by 1,000,000. If the input raster is in geographic coordinates (i.e. 
/// latitude and longitude) a warning will be issued and areas will be estimated based on per-row calculated degree lengths.
/// 
/// The tool can be run with a raster output (`--output`), a text output (`--out_text`), or both. If niether outputs are specified,
/// the tool will automatically output a raster named `area.tif`. 
/// 
/// Zero values in the input raster may be excluded from the area analysis if the `--zero_back` flag is used.
/// 
/// To calculate the area of vector polygons, use the `PolygonArea` tool instead.
/// 
/// # See Also
/// `PolygonArea`, `RasterHistogram`
pub struct RasterArea {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RasterArea {
    pub fn new() -> RasterArea {
        // public constructor
        let name = "RasterArea".to_string();
        let toolbox = "GIS Analysis".to_string();
        let description =
            "Calculates the area of polygons or classes within a raster image."
                .to_string();

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

        RasterArea {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RasterArea {
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
                "Tool run with no paramters.",
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

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let min_val = input.configs.display_min;
        let max_val = input.configs.display_max;
        let range = max_val - min_val + 0.00001f64; // otherwise the max value is outside the range
        let num_bins = range.ceil() as usize;
        let back_val = if zero_back {
            0f64
        } else {
            nodata
        };
        
        if is_grid_cell_units {
            let num_procs = num_cpus::get() as isize;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut freq_data = vec![0usize; num_bins];
                    let mut val: f64;
                    let mut bin: usize;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        for col in 0..columns {
                            val = input.get_value(row, col);
                            if val != nodata && val != back_val && val >= min_val && val <= max_val {
                                bin = (val - min_val).floor() as usize;
                                freq_data[bin] += 1;
                            }
                        }
                    }
                    tx.send(freq_data).unwrap();
                });
            }

            let mut freq_data = vec![0usize; num_bins];
            for tid in 0..num_procs {
                let data = rx.recv().unwrap();
                for a in 0..num_bins {
                    freq_data[a] += data[a];
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
                output.configs.data_type = DataType::I32;
                for row in 0..rows {
                    for col in 0..columns {
                        val = input.get_value(row, col);
                        if val != nodata && val != back_val && val >= min_val && val <= max_val {
                            bin = (val - min_val).floor() as usize;
                            output.set_value(row, col, freq_data[bin] as f64);
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
                println!("Class,Cells");
                for a in 0..num_bins {
                    if freq_data[a] > 0 {
                        val = (a as f64 + min_val).floor();
                        println!("{},{}", val, freq_data[a]);
                    }
                }
            }
        } else {
            let is_geographic = input.is_in_geographic_coordinates();
            if is_geographic && verbose {
                println!("Warning: the input file does not appear to be in a projected coodinate system. Area values will only be estimates.");
            }

            let num_procs = num_cpus::get() as isize;
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut resx = input.configs.resolution_x;
                    let mut resy = input.configs.resolution_y;
                    let mut area_data = vec![0f64; num_bins];
                    let mut val: f64;
                    let mut bin: usize;
                    let mut mid_lat: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        if is_geographic {
                            mid_lat = input.get_y_from_row(row).to_radians();
                            resx = resx * 111_111.0 * mid_lat.cos();
                            resy = resy * 111_111.0;
                        }
                        for col in 0..columns {
                            val = input.get_value(row, col);
                            if val != nodata && val != back_val && val >= min_val && val <= max_val {
                                bin = (val - min_val).floor() as usize;
                                area_data[bin] += resx * resy;
                            }
                        }
                    }
                    tx.send(area_data).unwrap();
                });
            }

            // we could just multiply the num cells by the cell area to get the area,
            // but for the possibility of an input in geographic coordinates where the
            // cell size is not constant for the data.
            let mut area_data = vec![0f64; num_bins];
            for tid in 0..num_procs {
                let data = rx.recv().unwrap();
                for a in 0..num_bins {
                    area_data[a] += data[a];
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
                output.configs.data_type = DataType::I32;
                for row in 0..rows {
                    for col in 0..columns {
                        val = input.get_value(row, col);
                        if val != nodata && val != back_val && val >= min_val && val <= max_val {
                            bin = (val - min_val).floor() as usize;
                            output.set_value(row, col, area_data[bin]);
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
                println!("Class,Area");
                for a in 0..num_bins {
                    if area_data[a] > 0f64 {
                        val = (a as f64 + min_val).floor();
                        println!("{},{}", val, area_data[a]);
                    }
                }
            }
        }

        Ok(())
    }
}
