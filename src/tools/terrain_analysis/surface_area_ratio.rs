/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 23/01/2019
Last Modified: 23/01/2019
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

/// This tool calculates the ratio between the surface area and planar area of grid cells within digital elevation models (DEMs).
/// The tool uses the method of Jenness (2004) to estimate the surface area of a DEM grid cell based on the elevations
/// contained within the 3 x 3 neighbourhood surrounding each cell. The surface area ratio has a lower bound of 1.0 for 
/// perfectly flat grid cells and is greater than 1.0 for other conditions. In particular, surface area ratio is a measure of
/// neighbourhood surface shape complexity (texture) and elevation variability (local slope).
/// 
/// # Reference
/// Jenness, J. S. (2004). Calculating landscape surface area from digital elevation models. Wildlife Society 
/// Bulletin, 32(3), 829-839.
/// 
/// # See Also
/// `RuggednessIndex`, `MultiscaleRoughness`, `CircularVarianceOfAspect`, `EdgeDensity`
pub struct SurfaceAreaRatio {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SurfaceAreaRatio {
    pub fn new() -> SurfaceAreaRatio {
        // public constructor
        let name = "SurfaceAreaRatio".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Calculates a the surface area ratio of each grid cell in an input DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif",
            short_exe, name
        )
        .replace("*", &sep);

        SurfaceAreaRatio {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SurfaceAreaRatio {
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
        let mut input_file = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
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

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F32;
        let rows = input.configs.rows as isize;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let columns = input.configs.columns as isize;
                /* ordering based on Jenness 2004

                    | 0 | 1 | 2 |
                    | 3 | 4 | 5 |
                    | 6 | 7 | 8 |
                */ 
                let dx = [-1, 0, 1, -1, 0, 1, -1, 0, 1];
                let dy = [-1, -1, -1, 0, 0, 0, 1, 1, 1];
                // let mut z: f64;
                let mut area: f64;
                let mut s: f64;
                let (mut p, mut q, mut r): (f64, f64, f64);
                let mut zvals: [f64; 9] = [0.0; 9];
                let mut zdiff: f64;
                let mut distances: [f64; 16] = [0.0; 16];
                let dist_pairs = [
                    [0, 1], // 0
                    [1, 2], // 1
                    [3, 4], // 2
                    [4, 5], // 3
                    [6, 7], // 4
                    [7, 8], // 5
                    [0, 3], // 6
                    [1, 4], // 7
                    [2, 5], // 8
                    [3, 6], // 9
                    [4, 7], // 10
                    [5, 8], // 11
                    [4, 0], // 12
                    [4, 2], // 13
                    [4, 6], // 14
                    [4, 8], // 15
                ];
                let triangle_sides = [
                    [0, 7, 12],
                    [1, 7, 13],
                    [2, 6, 12],
                    [3, 8, 13],
                    [2, 9, 14],
                    [3, 11, 15], 
                    [4, 10, 14],
                    [5, 10, 15]
                ];

                let mut resx = input.configs.resolution_x;
                let mut resy = input.configs.resolution_y;
                // let mut num_valid_facets: f64;
                let mut mid_lat: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    if input.is_in_geographic_coordinates() {
                        mid_lat = input.get_y_from_row(row).to_radians();
                        resx = resx * 111_111.0 * mid_lat.cos();
                        resy = resy * 111_111.0;
                    }
                    let res_diag = (resx * resx + resy * resy).sqrt();
                    let cell_area = resx * resy;
                    let eigth_area = cell_area / 8.0;

                    let dist_planar: [f64; 16] = [
                        resx, resx, resx, resx, 
                        resx, resx, resy, resy, 
                        resy, resy, resy, resy, 
                        res_diag, res_diag, res_diag, res_diag
                    ];
                
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if input.get_value(row, col) != nodata {
                            // get the elevation values
                            for c in 0..9 {
                                zvals[c] = input.get_value(row + dy[c], col + dx[c]);
                            }

                            // calculate the distances
                            for c in 0..16 {
                                if zvals[dist_pairs[c][0]] != nodata && zvals[dist_pairs[c][1]] != nodata {
                                    zdiff = (zvals[dist_pairs[c][0]] - zvals[dist_pairs[c][1]]).abs();
                                    distances[c] = (dist_planar[c] * dist_planar[c] + zdiff * zdiff).sqrt() / 2f64;
                                } else {
                                    distances[c] = 0f64;
                                }
                            }

                            // now calculate the area of each of the eight triangular facets,
                            // making sure to take note of how many, if any, facets have NoData vertices
                            // this will be used to adjust the planar area
                            area = 0f64;
                            let mut cell_area2 = cell_area;
                            for c in 0..8 {
                                p = distances[triangle_sides[c][0]];
                                q = distances[triangle_sides[c][1]];
                                r = distances[triangle_sides[c][2]];
                                if p * q * r != 0f64 {
                                    s = (p + q + r) / 2f64;
                                    let a = (s * (s - p) * (s - q) * (s - r)).sqrt();
                                    area += a;
                                } else {
                                    cell_area2 -= eigth_area;
                                }
                            }
                            if cell_area2 > 0f64 {
                                data[col as usize] = area / cell_area2;
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
                    println!("Performing analysis: {}%", progress);
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
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
