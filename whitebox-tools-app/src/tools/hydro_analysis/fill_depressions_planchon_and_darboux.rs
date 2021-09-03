/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 02/02/2020
Last Modified: 02/02/2020
License: MIT

NOTE: This tool exists for legacy reasons only and for algorithm performance comparison. Ultimately, users should
prefer the far more efficient FillDepressions tool instead.
*/

use whitebox_raster::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::i32;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to fill all of the depressions in a digital elevation model (DEM) and to remove the
/// flat areas using the Planchon and Darboux (2002) method. This is a common pre-processing step required by
/// many flow-path analysis tools to ensure continuous flow from each grid cell to an outlet located along
/// the grid edge. **This tool is currently not the most efficient depression-removal algorithm available in
/// WhiteboxTools**; `FillDepressions` and `BreachDepressionsLeastCost` are both more efficient and often
/// produce better, lower-impact results.
///
/// The user may optionally specify the size of the elevation increment used to solve flats (`--flat_increment`), although
/// **it is best not to specify this optional value and to let the algorithm determine the most suitable value itself**.
/// If a flat increment value isn't specified, the output DEM will use 64-bit floating point values in order
/// to make sure that the very small elevation increment value determined will be accurately stored. Consequently,
/// it may double the storage requirements as DEMs are often stored with 32-bit precision. However, if a flat increment
/// value is specified, the output DEM will keep the same data type as the input assuming the user chose its value wisely.
///
/// # Reference
/// Planchon, O. and Darboux, F., 2002. A fast, simple and versatile algorithm to fill the depressions of digital
/// elevation models. Catena, 46(2-3), pp.159-176.
///
/// # See Also
/// `FillDepressions`, `BreachDepressionsLeastCost`
pub struct FillDepressionsPlanchonAndDarboux {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FillDepressionsPlanchonAndDarboux {
    pub fn new() -> FillDepressionsPlanchonAndDarboux {
        // public constructor
        let name = "FillDepressionsPlanchonAndDarboux".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Fills all of the depressions in a DEM using the Planchon and Darboux (2002) method."
                .to_string();

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

        parameters.push(ToolParameter {
            name: "Fix flat areas?".to_owned(),
            flags: vec!["--fix_flats".to_owned()],
            description:
                "Optional flag indicating whether flat areas should have a small gradient applied."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Flat increment value (z units)".to_owned(),
            flags: vec!["--flat_increment".to_owned()],
            description: "Optional elevation increment applied to flat areas.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --fix_flats",
            short_exe, name
        )
        .replace("*", &sep);

        FillDepressionsPlanchonAndDarboux {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FillDepressionsPlanchonAndDarboux {
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
        let mut fix_flats = false;
        let mut flat_increment = f64::NAN;

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
            } else if flag_val == "-fix_flats" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    fix_flats = true;
                }
            } else if flag_val == "-flat_increment" {
                flat_increment = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
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

        let start_inclusive = Instant::now();

        let input = Raster::new(&input_file, "r")?;

        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let nodata_output = -32768.0f64;
        let large_value = f64::INFINITY;
        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F64;
        output.configs.nodata = nodata_output;
        output.reinitialize_values(large_value);

        let small_num = if fix_flats && !flat_increment.is_nan() {
            output.configs.data_type = input.configs.data_type; // Assume the user knows what he's doing
            flat_increment
        } else if fix_flats {
            output.configs.data_type = DataType::F64; // Don't take any chances and promote to 64-bit
            let resx = input.configs.resolution_x;
            let resy = input.configs.resolution_y;
            let diagres = (resx * resx + resy * resy).sqrt();
            let elev_digits = (input.configs.maximum as i64).to_string().len();
            let elev_multiplier = 10.0_f64.powi((15 - elev_digits) as i32);
            1.0_f64 / elev_multiplier as f64 * diagres.ceil()
        } else {
            output.configs.data_type = input.configs.data_type;
            0f64
        };


        /*
        Find the data edges. This is complicated by the fact that DEMs frequently
        have nodata edges, whereby the DEM does not occupy the full extent of
        the raster. One approach to doing this would be simply to scan the
        raster, looking for cells that neighbour nodata values. However, this
        assumes that there are no interior nodata holes in the data set. Instead,
        the approach used here is to perform a region-growing operation, looking
        for nodata values along the raster's edges.
        */
        let mut z: f64;
        let mut w: f64;
        let mut wn: f64;
        let (mut cn, mut rn): (isize, isize);
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut stack = vec![];
        for row in 0..rows {
            z = input.get_value(row, 0);
            w = output.get_value(row, 0);
            if z != nodata {
                output.set_value(row, 0, z);
            } else if w == large_value {
                output.set_value(row, 0, nodata_output);
                stack.push((row, 0));
                while let Some(cell) = stack.pop() {
                    for n in 0..8 {
                        rn = cell.0 + dy[n];
                        cn = cell.1 + dx[n];
                        w = output.get_value(rn, cn);
                        if w == large_value {
                            z = input.get_value(rn, cn);
                            if z == nodata {
                                output.set_value(rn, cn, nodata_output);
                                stack.push((rn, cn));
                            } else {
                                output.set_value(rn, cn, z);
                            }
                        }
                    }
                }
            }

            z = input.get_value(row, columns - 1);
            w = output.get_value(row, columns - 1);
            if z != nodata {
                output.set_value(row, columns - 1, z);
            } else if w == large_value {
                output.set_value(row, columns - 1, nodata_output);
                stack.push((row, columns - 1));
                while let Some(cell) = stack.pop() {
                    for n in 0..8 {
                        rn = cell.0 + dy[n];
                        cn = cell.1 + dx[n];
                        w = output.get_value(rn, cn);
                        if w == large_value {
                            z = input.get_value(rn, cn);
                            if z == nodata {
                                output.set_value(rn, cn, nodata_output);
                                stack.push((rn, cn));
                            } else {
                                output.set_value(rn, cn, z);
                            }
                        }
                    }
                }
            }
        }

        for col in 0..columns {
            z = input.get_value(0, col);
            w = output.get_value(0, col);
            if z != nodata {
                output.set_value(0, col, z);
            } else if w == large_value {
                output.set_value(0, col, nodata_output);
                stack.push((0, col));
                while let Some(cell) = stack.pop() {
                    for n in 0..8 {
                        rn = cell.0 + dy[n];
                        cn = cell.1 + dx[n];
                        w = output.get_value(rn, cn);
                        if w == large_value {
                            z = input.get_value(rn, cn);
                            if z == nodata {
                                output.set_value(rn, cn, nodata_output);
                                stack.push((rn, cn));
                            } else {
                                output.set_value(rn, cn, z);
                            }
                        }
                    }
                }
            }

            z = input.get_value(rows - 1, col);
            w = output.get_value(rows - 1, col);
            if z != nodata {
                output.set_value(rows - 1, col, z);
            } else if w == large_value {
                output.set_value(rows - 1, col, nodata_output);
                stack.push((rows - 1, col));
                while let Some(cell) = stack.pop() {
                    for n in 0..8 {
                        rn = cell.0 + dy[n];
                        cn = cell.1 + dx[n];
                        w = output.get_value(rn, cn);
                        if w == large_value {
                            z = input.get_value(rn, cn);
                            if z == nodata {
                                output.set_value(rn, cn, nodata_output);
                                stack.push((rn, cn));
                            } else {
                                output.set_value(rn, cn, z);
                            }
                        }
                    }
                }
            }
        }

        let mut i = 0; // 'i' will control the order of the scan directions.
        let mut something_done = true;
        let mut loop_num = 0;
        while something_done {
            loop_num += 1;
            something_done = false;
            let mut num_modified = 0u64;
            if i == 0 {
                for row in 1..rows - 1 {
                    for col in 1..columns - 1 {
                        z = input.get_value(row, col);
                        w = output.get_value(row, col);
                        if w != nodata_output {
                            if w > z {
                                for n in 0..8 {
                                    rn = row + dy[n];
                                    cn = col + dx[n];
                                    wn = output.get_value(rn, cn);
                                    if wn != nodata_output {
                                        wn += small_num;
                                        if z >= wn {
                                            // operation 1
                                            output.set_value(row, col, z);
                                            something_done = true;
                                            num_modified += 1;
                                            break;
                                        } else if w > wn && wn > z {
                                            // operation 2
                                            output.set_value(row, col, wn);
                                            w = wn;
                                            something_done = true;
                                            num_modified += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("progress (Loop {}): {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            } else if i == 1 {
                for row in (1..rows - 1).rev() {
                    for col in (1..columns - 1).rev() {
                        z = input.get_value(row, col);
                        w = output.get_value(row, col);
                        if w != nodata_output {
                            if w > z {
                                for n in 0..8 {
                                    rn = row + dy[n];
                                    cn = col + dx[n];
                                    wn = output.get_value(rn, cn);
                                    if wn != nodata_output {
                                        wn += small_num;
                                        if z >= wn {
                                            // operation 1
                                            output.set_value(row, col, z);
                                            something_done = true;
                                            num_modified += 1;
                                            break;
                                        } else if w > wn && wn > z {
                                            // operation 2
                                            output.set_value(row, col, wn);
                                            w = wn;
                                            something_done = true;
                                            num_modified += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("progress (Loop {}): {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            } else if i == 2 {
                for row in 1..rows - 1 {
                    for col in (1..columns - 1).rev() {
                        z = input.get_value(row, col);
                        w = output.get_value(row, col);
                        if w != nodata_output {
                            if w > z {
                                for n in 0..8 {
                                    rn = row + dy[n];
                                    cn = col + dx[n];
                                    wn = output.get_value(rn, cn);
                                    if wn != nodata_output {
                                        wn += small_num;
                                        if z >= wn {
                                            // operation 1
                                            output.set_value(row, col, z);
                                            something_done = true;
                                            num_modified += 1;
                                            break;
                                        } else if w > wn && wn > z {
                                            // operation 2
                                            output.set_value(row, col, wn);
                                            w = wn;
                                            something_done = true;
                                            num_modified += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("progress (Loop {}): {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            } else {
                // i == 3
                for row in (1..rows - 1).rev() {
                    for col in 1..columns - 1 {
                        z = input.get_value(row, col);
                        w = output.get_value(row, col);
                        if w != nodata_output {
                            if w > z {
                                for n in 0..8 {
                                    rn = row + dy[n];
                                    cn = col + dx[n];
                                    wn = output.get_value(rn, cn);
                                    if wn != nodata_output {
                                        wn += small_num;
                                        if z >= wn {
                                            // operation 1
                                            output.set_value(row, col, z);
                                            something_done = true;
                                            num_modified += 1;
                                            break;
                                        } else if w > wn && wn > z {
                                            // operation 2
                                            output.set_value(row, col, wn);
                                            w = wn;
                                            something_done = true;
                                            num_modified += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("progress (Loop {}): {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            i += 1;
            if i > 3 {
                i = 0;
            }
            if verbose {
                println!("Loop {} (Num. modified cells: {})", loop_num, num_modified);
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.display_min = input.configs.display_min;
        output.configs.display_max = input.configs.display_max;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Fix flats: {}", fix_flats));
        if fix_flats {
            output.add_metadata_entry(format!("Flat increment value: {}", small_num));
        }
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
            let elapsed_time = get_formatted_elapsed_time(start_inclusive);
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
