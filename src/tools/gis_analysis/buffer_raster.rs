/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 22/06/2017
Last Modified: 18/10/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to identify an area of interest within a specified distance of 
/// features of interest in a raster data set. 
/// 
/// The Euclidean distance (i.e. straight-line distance) is calculated between each grid 
/// cell and the nearest 'target cell' in the input image. Distance is calcualted using the
/// efficient method of Shih and Wu (2004). Target cells are all non-zero, 
/// non-NoData grid cells. Because NoData values in the input image are assigned the NoData 
/// value in the output image, the only valid background value in the input image is zero.
/// 
/// The user must specify the input and output image names, the desired buffer size (`--size`), and, 
/// optionally, whether the distance units are measured in grid cells (i.e. `--gridcells` flag). 
/// If the `--gridcells` flag is not specified, the linear units of the raster's coordinate 
/// reference system will be used.
/// 
/// # Reference
/// Shih FY and Wu Y-T (2004), Fast Euclidean distance transformation in two scans using a 3 x 3
/// neighborhood, *Computer Vision and Image Understanding*, 93: 195-205.
///
/// # See Also
/// `EuclideanDistance`
pub struct BufferRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BufferRaster {
    pub fn new() -> BufferRaster {
        // public constructor
        let name = "BufferRaster".to_string();
        let toolbox = "GIS Analysis/Distance Tools".to_string();
        let description = "Maps a distance-based buffer around each non-background (non-zero/non-nodata) grid cell in an input image.".to_string();

        // let mut parameters = "-i, --input   Input raster file.\n".to_owned();
        // parameters.push_str("-o, --output  Output raster file.\n");
        // parameters.push_str("--size        Buffer size.\n");
        // parameters.push_str("--gridcells   Optional flag to indicate that the 'size' threshold should be measured in grid cells instead of the default.\n");

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
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Buffer Size".to_owned(),
            flags: vec!["--size".to_owned()],
            description: "Buffer size.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter{
            name: "Buffer size measured in grid cells?".to_owned(), 
            flags: vec!["--gridcells".to_owned()], 
            description: "Optional flag to indicate that the 'size' threshold should be measured in grid cells instead of the default map units.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: None,
            optional: true
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        BufferRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BufferRaster {
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
        let mut buffer_size: f64 = 10.0;
        let mut grid_cell_units = false;

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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-size" || vec[0].to_lowercase() == "--size" {
                if keyval {
                    buffer_size = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    buffer_size = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-gridcells"
                || vec[0].to_lowercase() == "--gridcells"
            {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    grid_cell_units = true;
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
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Raster::new(&input_file, "r")?;

        let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let mut r_x: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut r_y: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);

        let mut h: f64;
        let mut which_cell: usize;
        let inf_val = f64::INFINITY;
        let d_x = [-1, -1, 0, 1, 1, 1, 0, -1];
        let d_y = [0, -1, -1, -1, 0, 1, 1, 1];
        let g_x = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let g_y = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let (mut x, mut y): (isize, isize);
        let (mut z, mut z2, mut z_min): (f64, f64, f64);

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != 0.0 {
                    output[(row, col)] = 0.0;
                } else {
                    output[(row, col)] = inf_val;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing Rasters: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = output[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 0..4 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = output[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                0 => 2.0 * r_x[(y, x)] + 1.0,
                                1 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                2 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 3
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (1 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in (0..rows).rev() {
            for col in (0..columns).rev() {
                z = output[(row, col)];
                if z != 0.0 {
                    z_min = inf_val;
                    which_cell = 0;
                    for i in 4..8 {
                        x = col + d_x[i];
                        y = row + d_y[i];
                        z2 = output[(y, x)];
                        if z2 != nodata {
                            h = match i {
                                5 => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0),
                                4 => 2.0 * r_x[(y, x)] + 1.0,
                                6 => 2.0 * r_y[(y, x)] + 1.0,
                                _ => 2.0 * (r_x[(y, x)] + r_y[(y, x)] + 1.0), // 7
                            };
                            z2 += h;
                            if z2 < z_min {
                                z_min = z2;
                                which_cell = i;
                            }
                        }
                    }
                    if z_min < z {
                        output[(row, col)] = z_min;
                        x = col + d_x[which_cell];
                        y = row + d_y[which_cell];
                        r_x[(row, col)] = r_x[(y, x)] + g_x[which_cell];
                        r_y[(row, col)] = r_y[(y, x)] + g_y[which_cell];
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (2 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        if grid_cell_units {
            cell_size = 1.0;
        }

        let mut dist: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    dist = output[(row, col)].sqrt() * cell_size;
                    if dist <= buffer_size {
                        output[(row, col)] = 1.0;
                    } else {
                        output[(row, col)] = 0.0;
                    }
                } else {
                    output[(row, col)] = nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (3 of 3): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Buffer size: {}", buffer_size));
        output.add_metadata_entry(format!("Grid cells as units: {}", grid_cell_units));
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
