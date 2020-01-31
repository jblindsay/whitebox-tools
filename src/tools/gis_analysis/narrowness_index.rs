/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 17/02/2019
Last Modified: 17/02/2019
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tools calculates a type of shape narrowness index (*NI*) for raster objects. The index is equal to:
///
/// > *NI* = *A* / (&#960;*MD*<sup>2</sup>)
///
/// where *A* is the patch area and *MD* is the maximum distance-to-edge of the patch. Circular-shaped patches
/// will have a narrowness index near 1.0, while more narrow patche shapes will have higher index values. The
/// index may be conceptualized as the ratio of the patch area to the area of the largest contained circle,
/// although in practice the circle defined by the radius of the maximum distance-to-edge will often fall
/// outside the patch boundaries.
///
/// Objects in the input raster (`--input`) are designated by their unique identifers. Identifer values must
/// be positive, non-zero whole numbers.
///
/// # See Also
/// `LinearityIndex`, `ElongationRatio`
pub struct NarrownessIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl NarrownessIndex {
    pub fn new() -> NarrownessIndex {
        // public constructor
        let name = "NarrownessIndex".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description = "Calculates the narrowness of raster polygons.".to_string();

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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=input.tif -o=output.tif --zero_back",
            short_exe, name
        )
        .replace("*", &sep);

        NarrownessIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for NarrownessIndex {
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
            println!("Reading data...");
        }

        // let input = Arc::new(Raster::new(&input_file, "r")?);
        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();

        // let nodata = input.configs.nodata;
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        let range = max_val - min_val + 0.00001f64; // otherwise the max value is outside the range
        let num_bins = range.ceil() as usize;
        let mut bin: usize;

        // calcuate the distance from an edge
        if verbose {
            println!("Calculating maximum distance from edge for each patch...");
        }
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let out_nodata = -999f64;
        output.reinitialize_values(out_nodata);
        output.configs.nodata = out_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
        output.configs.palette = String::from("spectrum_black_background.pal");

        let mut r_x: Array2D<f64> = Array2D::new(rows, columns, 0f64, out_nodata)?;
        let mut r_y: Array2D<f64> = Array2D::new(rows, columns, 0f64, out_nodata)?;

        let mut area_data = vec![0usize; num_bins];
        let mut max_width = vec![0f64; num_bins];

        let mut h: f64;
        let mut which_cell: usize;
        let inf_val = f64::INFINITY;
        let d_x = [-1, -1, 0, 1, 1, 1, 0, -1];
        let d_y = [0, -1, -1, -1, 0, 1, 1, 1];
        let g_x = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
        let g_y = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let (mut x, mut y): (isize, isize);
        let (mut z, mut z2, mut z_min): (f64, f64, f64);
        let cell_size = (input.configs.resolution_x + input.configs.resolution_y) / 2.0;
        let mut is_edge: bool;

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z == 0.0 || z == nodata {
                    output[(row, col)] = 0.0;
                } else {
                    bin = (z - min_val).floor() as usize;
                    area_data[bin] += 1;

                    // is it an edge cell?
                    is_edge = false;
                    for a in 0..8 {
                        z2 = input[(row + d_y[a], col + d_x[a])];
                        if z2 != z {
                            is_edge = true;
                            break;
                        }
                    }
                    if !is_edge {
                        output[(row, col)] = inf_val;
                    } else {
                        output[(row, col)] = cell_size;
                        max_width[bin] = cell_size;
                    }
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
                        if z2 != out_nodata {
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
                    println!("Progress (1 of 4): {}%", progress);
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
                        if z2 != out_nodata {
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
                progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (2 of 4): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata {
                    if z != 0f64 {
                        output[(row, col)] = output[(row, col)].sqrt() * cell_size;
                        bin = (z - min_val).floor() as usize;
                        if output[(row, col)] > max_width[bin] {
                            max_width[bin] = output[(row, col)];
                        }
                    } else {
                        output[(row, col)] = 0f64;
                    }
                } else {
                    output[(row, col)] = out_nodata;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (3 of 4): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let cell_area = cell_size * cell_size;
        for bin in 1..num_bins {
            max_width[bin] = (area_data[bin] as f64 * cell_area)
                / (f64::consts::PI * max_width[bin] * max_width[bin]);
        }

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata && z != 0f64 {
                    bin = (z - min_val).floor() as usize;
                    output[(row, col)] = max_width[bin];
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress (4 of 4): {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let num_procs = num_cpus::get() as isize;
        // let (tx, rx) = mpsc::channel();
        // for tid in 0..num_procs {
        //     let input = input.clone();
        //     let tx = tx.clone();
        //     thread::spawn(move || {
        //         let mut freq_data = vec![0usize; num_bins];
        //         let mut min_row = vec![isize::max_value(); num_bins];
        //         let mut max_row = vec![isize::min_value(); num_bins];
        //         let mut min_col = vec![isize::max_value(); num_bins];
        //         let mut max_col = vec![isize::min_value(); num_bins];
        //         let mut val: f64;
        //         let mut n1: f64;
        //         let mut bin: usize;
        //         for row in (0..rows).filter(|r| r % num_procs == tid) {
        //             for col in 0..columns {
        //                 val = input.get_value(row, col);
        //                 if val > 0f64 && val >= min_val && val <= max_val {
        //                     n1 = input.get_value(row, col - 1);
        //                     // n2 = input.get_value(row, col + 1);

        //                     bin = (val - min_val).floor() as usize;

        //                     if val != n1 {
        //                         freq_data[bin] += 1;
        //                     }

        //                     if row < min_row[bin] { min_row[bin] = row; }
        //                     if row > max_row[bin] { max_row[bin] = row; }
        //                     if col < min_col[bin] { min_col[bin] = col; }
        //                     if col > max_col[bin] { max_col[bin] = col; }
        //                 }
        //             }
        //         }

        //         for col in (0..columns).filter(|c| c % num_procs == tid) {
        //             for row in 0..rows {
        //                 val = input.get_value(row, col);
        //                 if val > 0f64 && val >= min_val && val <= max_val {
        //                     n1 = input.get_value(row - 1, col);

        //                     if val != n1 {
        //                         bin = (val - min_val).floor() as usize;
        //                         freq_data[bin] += 1;
        //                     }
        //                 }
        //             }
        //         }

        //         tx.send((freq_data, min_row, max_row, min_col, max_col)).unwrap();
        //     });
        // }

        // let mut freq_data = vec![0usize; num_bins];
        // let mut min_row = vec![isize::max_value(); num_bins];
        // let mut max_row = vec![isize::min_value(); num_bins];
        // let mut min_col = vec![isize::max_value(); num_bins];
        // let mut max_col = vec![isize::min_value(); num_bins];
        // for tid in 0..num_procs {
        //     let (data1, data2, data3, data4, data5) = rx.recv().expect("Error receiving data from thread.");
        //     for bin in 0..num_bins {
        //         freq_data[bin] += data1[bin];
        //         if data2[bin] < min_row[bin] { min_row[bin] = data2[bin]; }
        //         if data3[bin] > max_row[bin] { max_row[bin] = data3[bin]; }
        //         if data4[bin] < min_col[bin] { min_col[bin] = data4[bin]; }
        //         if data5[bin] > max_col[bin] { max_col[bin] = data5[bin]; }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * (tid + 1) as f64 / num_procs as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        // let mut bin: usize;
        // let mut index_values = vec![0f64; num_bins];
        // for bin in 1..num_bins {
        //     if freq_data[bin] > 0 {
        //         index_values[bin] = freq_data[bin] as f64 / ((max_row[bin] - min_row[bin] + 1) + (max_col[bin] - min_col[bin] + 1)) as f64;
        //     }
        // }

        // let mut val: f64;
        // for row in 0..rows {
        //     for col in 0..columns {
        //         val = input.get_value(row, col);
        //         if val > 0f64 && val >= min_val && val <= max_val {
        //             bin = (val - min_val).floor() as usize;
        //             output.set_value(row, col, index_values[bin]);
        //         } else if val == 0f64 {
        //             output.set_value(row, col, 0f64);
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Progress: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

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
