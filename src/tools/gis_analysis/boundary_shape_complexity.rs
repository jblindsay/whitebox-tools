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
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tools calculates a type of shape complexity index for raster objects, focused on the complexity of the
/// boundary of polygons. The index uses the `LineThinning` tool to estimate a skeletonized network for each 
/// input raster polygon. The Boundary Shape Complexity (BSC) index is then calculated as the percentage of the 
/// skeletonized network belonging to exterior links. Polygons with more complex boundaries will possess
/// more branching skeletonized networks, with each spur in the boundary possessing a short exterior branch. The 
/// two longest exterior links in the network are considered to be part of the main network.  Therefore, 
/// polygons of complex shaped boundaries will have a higher percentage of their skeleton networks consisting 
/// of exterior links. It is expected that simple convex hulls should have relatively low BSC index values.
/// 
/// Objects in the input raster (`--input`) are designated by their unique identifers. Identifer values should be 
/// positive, non-zero whole numbers.
/// 
/// # See Also
/// `ShapeComplexityIndexRaster`, `LineThinning`
pub struct BoundaryShapeComplexity {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl BoundaryShapeComplexity {
    pub fn new() -> BoundaryShapeComplexity {
        // public constructor
        let name = "BoundaryShapeComplexity".to_string();
        let toolbox = "GIS Analysis/Patch Shape Tools".to_string();
        let description =
            "Calculates the complexity of the boundaries of raster polygons."
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

        BoundaryShapeComplexity {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for BoundaryShapeComplexity {
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

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let min_val = input.configs.minimum;
        let max_val = input.configs.maximum;
        let range = max_val - min_val + 0.00001f64; // otherwise the max value is outside the range
        let num_bins = range.ceil() as usize;
        let mut bin: usize;

        let start = Instant::now();

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata; columns as usize];
                    for col in 0..columns {
                        if input[(row, col)] > 0.0 && input[(row, col)] != nodata {
                            data[col as usize] = 1.0;
                        } else if input[(row, col)] == 0.0 {
                            data[col as usize] = 0.0;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        if verbose {
            println!("Performing line-thinning...");
        }

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let out_nodata = -999f64;
        output.reinitialize_values(out_nodata);
        output.configs.nodata = out_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.data_type = DataType::F32;
        output.configs.palette = String::from("spectrum_black_background.pal");
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
            output.set_row_data(row, data);

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing output: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        let mut did_something = true;
        let mut loop_num = 0;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        
        let elements1 = [
            [6, 7, 0, 4, 3, 2],
            [0, 1, 2, 4, 5, 6],
            [2, 3, 4, 6, 7, 0],
            [4, 5, 6, 0, 1, 2],
        ];

        let elements2 = [
            [7, 0, 1, 3, 5],
            [1, 2, 3, 5, 7],
            [3, 4, 5, 7, 1],
            [5, 6, 7, 1, 3],
        ];

        let vals1 = [0f64, 0f64, 0f64, 1f64, 1f64, 1f64];
        let vals2 = [0f64, 0f64, 0f64, 1f64, 1f64];

        let mut neighbours = [0.0; 8];
        let mut pattern_match: bool;
        let mut z: f64;
        while did_something {
            loop_num += 1;
            did_something = false;
            for a in 0..4 {
                for row in 0..rows {
                    for col in 0..columns {
                        z = output[(row, col)];
                        if z > 0.0 && z != nodata {
                            // fill the neighbours array
                            for i in 0..8 {
                                neighbours[i] = output[(row + dy[i], col + dx[i])];
                            }

                            // scan through element
                            pattern_match = true;
                            for i in 0..6 {
                                if neighbours[elements1[a][i]] != vals1[i] {
                                    pattern_match = false;
                                }
                            }

                            if pattern_match {
                                output[(row, col)] = 0.0;
                                did_something = true;
                            } else {
                                pattern_match = true;
                                for i in 0..5 {
                                    if neighbours[elements2[a][i]] != vals2[i] {
                                        pattern_match = false;
                                    }
                                }

                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * (a + 1) as f64 / 4.0) as usize;
                    if progress != old_progress {
                        println!("Loop Number {}: {}%", loop_num, progress);
                        old_progress = progress;
                    }
                }
            }
        }

        // let mut did_something = true;
        // let mut loop_num = 0;
        // let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        // let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        // let elements = vec![
        //     vec![6, 7, 0, 4, 3, 2],
        //     vec![7, 0, 1, 3, 5],
        //     vec![0, 1, 2, 4, 5, 6],
        //     vec![1, 2, 3, 5, 7],
        //     vec![2, 3, 4, 6, 7, 0],
        //     vec![3, 4, 5, 7, 1],
        //     vec![4, 5, 6, 0, 1, 2],
        //     vec![5, 6, 7, 1, 3],
        // ];

        // let vals = vec![
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
        //     vec![0f64, 0f64, 0f64, 1f64, 1f64],
        // ];

        // let mut neighbours = [0.0; 8];
        // let mut pattern_match: bool;
        // let mut z: f64;
        // while did_something {
        //     loop_num += 1;
        //     did_something = false;
        //     for a in 0..8 {
        //         for row in 0..rows {
        //             for col in 0..columns {
        //                 z = output[(row, col)];
        //                 if z > 0.0 && z != nodata {
        //                     // fill the neighbours array
        //                     for i in 0..8 {
        //                         neighbours[i] = output[(row + dy[i], col + dx[i])];
        //                     }

        //                     // scan through element
        //                     pattern_match = true;
        //                     for i in 0..elements[a].len() {
        //                         if neighbours[elements[a][i]] != vals[a][i] {
        //                             pattern_match = false;
        //                         }
        //                     }
        //                     if pattern_match {
        //                         output[(row, col)] = 0.0;
        //                         did_something = true;
        //                     }
        //                 }
        //             }
        //         }
        //         if verbose {
        //             progress = (100.0_f64 * a as f64 / 7.0) as usize;
        //             if progress != old_progress {
        //                 println!("Loop Number {}: {}%", loop_num, progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }
        // }


        let mut visited: Array2D<i8> = Array2D::new(rows, columns, 0, -1)?;
        let dx = [-1, -1, 0, 1, 1, 1, 0, -1];
        let dy = [0, -1, -1, -1, 0, 1, 1, 1];
        let mut zn: f64;
        // let mut is_edge: bool;
        // let mut num_edge_cells = vec![0usize; num_bins];
        let mut num_cells = vec![0usize; num_bins];
        let mut longest_exterior_link = vec![0usize; num_bins];
        let mut second_longest_exterior_link = vec![0usize; num_bins];
        let mut num_end_nodes = vec![0f64; num_bins];
        let mut num_line_thinned_neighbours: usize;
        let mut polyid: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = output[(row, col)];
                if z > 0f64 {
                    polyid = input[(row, col)];
                    num_line_thinned_neighbours = 0;
                    for a in 0..8 {
                        zn = output[(row + dy[a], col + dx[a])];
                        if zn == 1f64 && input[(row + dy[a], col + dx[a])] == polyid {
                            num_line_thinned_neighbours += 1
                        }
                    }

                    bin = (input[(row, col)] - min_val).floor() as usize;
                    num_cells[bin] += 1;
                    if num_line_thinned_neighbours == 1 {
                        num_end_nodes[bin] += 1f64;
                        let mut row_n = row;
                        let mut col_n = col;
                        let mut next_n: usize;
                        let mut link_length = 1;
                        did_something = true;
                        while did_something {
                            visited.set_value(row_n, col_n, 1);
                            num_line_thinned_neighbours = 0;
                            next_n = 8;
                            for a in 0..8 {
                                zn = output[(row_n + dy[a], col_n + dx[a])];
                                if zn == 1f64 && input[(row_n + dy[a], col_n + dx[a])] == polyid {
                                    num_line_thinned_neighbours += 1;
                                    if visited.get_value(row_n + dy[a], col_n + dx[a]) == 0 {
                                        next_n = a;
                                    }
                                }
                            }
                            if num_line_thinned_neighbours < 3 && next_n < 8 {
                                // num_end_nodes[bin] += 1f64;
                                link_length += 1;
                                row_n += dy[next_n];
                                col_n += dx[next_n];
                            } else {
                                did_something = false;
                            }
                        } 

                        num_end_nodes[bin] += link_length as f64;
                        if longest_exterior_link[bin] < link_length {
                            second_longest_exterior_link[bin] = longest_exterior_link[bin];
                            longest_exterior_link[bin] = link_length;
                        } else if second_longest_exterior_link[bin] < link_length {
                            second_longest_exterior_link[bin] = link_length;
                        }
                    }
                }
                // z = input[(row, col)];
                // if z != nodata && z != 0f64 {
                //     bin = (z - min_val).floor() as usize;

                //     // is it an edge cell?
                //     is_edge = false;
                //     num_line_thinned_neighbours = 0;
                //     for a in 0..8 {
                //         zn = input[(row + dy[a], col + dx[a])];
                //         if zn != z {
                //             is_edge = true;
                //         }
                //         if output[(row + dy[a], col + dx[a])] > 0f64 && zn == z {
                //             num_line_thinned_neighbours += 1
                //         }
                //     }

                //     num_cells[bin] += 1;

                //     if is_edge {
                //         // num_edge_cells[bin] += 1;
                //         if output[(row, col)] > 0f64 && num_line_thinned_neighbours < 2 {
                //             num_end_nodes[bin] += 1f64
                //         }
                //     }
                // }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating Index: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for bin in 1..num_bins {
            // if num_end_nodes[bin] >= 2f64 {
            //     num_end_nodes[bin] -= 2f64; // you get 2 end nodes for free; i.e. they don't count against the complexity. 
            //     // Elongated convex hulls and single-cell wide lines will have two end nodes.
            // } else {
            //     num_end_nodes[bin] = 0f64;
            // }
            // let perimeter = (num_cells[bin] as f64 / f64::consts::PI).sqrt() * 2f64 * f64::consts::PI;
            // num_end_nodes[bin] = 100f64 * num_end_nodes[bin] / perimeter; // num_edge_cells[bin] as f64;
            num_end_nodes[bin] = 100f64 * (num_end_nodes[bin] - longest_exterior_link[bin] as f64 - second_longest_exterior_link[bin] as f64) / num_cells[bin] as f64;
            // if num_end_nodes[bin] == 100f64 {
            //     // This is the case of a simple shape with no actual exterior links in the skeleton. That is
            //     // the measured 'exterior' is the full skeleton length.
            //     num_end_nodes[bin] = 0f64;
            // }
        }

        for row in 0..rows {
            for col in 0..columns {
                z = input[(row, col)];
                if z != nodata && z != 0f64 {
                    bin = (z - min_val).floor() as usize;
                    output[(row, col)] = num_end_nodes[bin];
                } else if z == 0f64 {
                    output[(row, col)] = 0f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Calculating Index: {}%", progress);
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
