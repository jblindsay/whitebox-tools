/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 04/07/2017
Last Modified: 18/10/2019
License: MIT

NOTES: Add support for vector seed points.
*/

use whitebox_raster::*;
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to mark the flowpath initiated from user-specified locations downslope and
/// terminating at either the grid's edge or a grid cell with undefined flow direction. The user must
/// input the name of a D8 flow pointer grid (`--d8_pntr`) and an input vector file indicating the location
/// of one or more initiation points, i.e. 'seed points' (`--seed_pts`). The seed point file must be a
/// vector of the POINT ShapeType. Note that the flow pointer should be generated from a DEM that has
/// been processed to remove all topographic depression (see `BreachDepressions` and `FillDepressions`) and
/// created using the D8 flow algorithm (`D8Pointer`).
///
/// # See Also
/// `D8Pointer`, `BreachDepressions`, `FillDepressions`, `DownslopeFlowpathLength`, `DownslopeDistanceToStream`
pub struct TraceDownslopeFlowpaths {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TraceDownslopeFlowpaths {
    pub fn new() -> TraceDownslopeFlowpaths {
        // public constructor
        let name = "TraceDownslopeFlowpaths".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Traces downslope flowpaths from one or more target sites (i.e. seed points)."
                .to_string();

        let mut parameters = vec![];
        // parameters.push(ToolParameter{
        //     name: "Input Seed Points File".to_owned(),
        //     flags: vec!["--seed_pts".to_owned()],
        //     description: "Input raster seed points file.".to_owned(),
        //     parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
        //     default_value: None,
        //     optional: false
        // });

        parameters.push(ToolParameter {
            name: "Input Vector Seed Points File".to_owned(),
            flags: vec!["--seed_pts".to_owned()],
            description: "Input vector seed points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input D8 pointer raster file.".to_owned(),
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
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Should a background value of zero be used?".to_owned(),
            flags: vec!["--zero_background".to_owned()],
            description: "Flag indicating whether a background value of zero should be used."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --seed_pts=seeds.shp --flow_dir=flow_directions.tif --output=flow_paths.tif", short_exe, name).replace("*", &sep);

        TraceDownslopeFlowpaths {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TraceDownslopeFlowpaths {
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
        let mut seed_file = String::new();
        let mut flowdir_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut background_val = f64::NEG_INFINITY;

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
            if vec[0].to_lowercase() == "-seed_pts" || vec[0].to_lowercase() == "--seed_pts" {
                if keyval {
                    seed_file = vec[1].to_string();
                } else {
                    seed_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "--d8_pntr"
                || vec[0].to_lowercase() == "-flow_dir"
                || vec[0].to_lowercase() == "--flow_dir"
            {
                if keyval {
                    flowdir_file = vec[1].to_string();
                } else {
                    flowdir_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-esri_pntr"
                || vec[0].to_lowercase() == "--esri_pntr"
                || vec[0].to_lowercase() == "--esri_style"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
                }
            } else if vec[0].to_lowercase() == "-zero_background"
                || vec[0].to_lowercase() == "--zero_background"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    background_val = 0f64;
                }
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

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !seed_file.contains(&sep) && !seed_file.contains("/") {
            seed_file = format!("{}{}", working_directory, seed_file);
        }
        if !flowdir_file.contains(&sep) && !flowdir_file.contains("/") {
            flowdir_file = format!("{}{}", working_directory, flowdir_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // if verbose { println!("Reading destination data...") };
        // let seeds = Raster::new(&seed_file, "r")?;

        if verbose {
            println!("Reading flow direction data...")
        };
        let flowdir = Raster::new(&flowdir_file, "r")?;

        // make sure the input files have the same size
        // if seeds.configs.rows != flowdir.configs.rows || seeds.configs.columns != flowdir.configs.columns {
        //     return Err(Error::new(ErrorKind::InvalidInput,
        //                         "The input files must have the same number of rows and columns and spatial extent."));
        // }

        let start = Instant::now();
        let rows = flowdir.configs.rows as isize;
        let columns = flowdir.configs.columns as isize;
        let nodata = flowdir.configs.nodata;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }

        let mut output = Raster::initialize_using_file(&output_file, &flowdir);
        output.reinitialize_values(background_val);

        let seeds = Shapefile::read(&seed_file)?;

        // make sure the input vector file is of points type
        if seeds.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let mut seed_rows = vec![];
        let mut seed_cols = vec![];
        for record_num in 0..seeds.num_records {
            let record = seeds.get_record(record_num);
            seed_rows.push(flowdir.get_row_from_y(record.points[0].y));
            seed_cols.push(flowdir.get_column_from_x(record.points[0].x));

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (seeds.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Locating seed points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut pntr_matches: [usize; 129] = [0usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 0usize;
            pntr_matches[2] = 1usize;
            pntr_matches[4] = 2usize;
            pntr_matches[8] = 3usize;
            pntr_matches[16] = 4usize;
            pntr_matches[32] = 5usize;
            pntr_matches[64] = 6usize;
            pntr_matches[128] = 7usize;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in d_x and d_y.
            pntr_matches[1] = 1usize;
            pntr_matches[2] = 2usize;
            pntr_matches[4] = 3usize;
            pntr_matches[8] = 4usize;
            pntr_matches[16] = 5usize;
            pntr_matches[32] = 6usize;
            pntr_matches[64] = 7usize;
            pntr_matches[128] = 0usize;
        }
        let (mut x, mut y): (isize, isize);
        let mut flag: bool;
        let mut dir: f64;
        // for row in 0..rows {
        //     for col in 0..columns {
        //     if seeds[(row, col)] > 0.0 && flowdir[(row, col)] != nodata {
        //         flag = false;
        //         x = col;
        //         y = row;
        //         while !flag {
        //             if output[(y, x)] == background_val {
        //                 output[(y, x)] = 1.0;
        //             } else {
        //                 output.increment(y, x, 1.0);
        //             }
        //             // find its downslope neighbour
        //             dir = flowdir[(y, x)];
        //             if dir != nodata && dir > 0.0 {
        //                 // move x and y accordingly
        //                 x += dx[pntr_matches[dir as usize]];
        //                 y += dy[pntr_matches[dir as usize]];
        //             } else {
        //                 flag = true;
        //             }
        //         }
        //     } else if flowdir[(row, col)] == nodata {
        //         output[(row, col)] = nodata;
        //     }
        // }
        // if verbose {
        //     progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //     if progress != old_progress {
        //         println!("Progress: {}%", progress);
        //         old_progress = progress;
        //     }
        // }
        // }

        for i in 0..seed_cols.len() {
            let row = seed_rows[i];
            let col = seed_cols[i];
            if flowdir.get_value(row, col) != nodata {
                flag = false;
                x = col;
                y = row;
                while !flag {
                    if output.get_value(y, x) == background_val {
                        output.set_value(y, x, 1f64);
                    } else {
                        output.increment(y, x, 1f64);
                    }
                    // find its downslope neighbour
                    dir = flowdir.get_value(y, x);
                    if dir != nodata && dir > 0.0 {
                        // move x and y accordingly
                        x += dx[pntr_matches[dir as usize]];
                        y += dy[pntr_matches[dir as usize]];
                    } else {
                        flag = true;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / (seed_cols.len() - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if flowdir.get_value(row, col) == nodata {
                    output.set_value(row, col, nodata);
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

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "spectrum.plt".to_string();
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Seed points raster file: {}", seed_file));
        output.add_metadata_entry(format!(
            "D8 flow direction (pointer) raster: {}",
            flowdir_file
        ));
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
