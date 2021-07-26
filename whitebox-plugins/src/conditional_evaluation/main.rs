/* 
Authors:  Dr. John Lindsay
Created: 21/07/2021
Last Modified: 21/07/2021
License: MIT
*/

use std::collections::BTreeMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use num_cpus;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;
// use v_eval::{Value, Eval};
use fasteval;

/// The ConditionalEvaluation tool can be used to perform an if-then-else style conditional evaluation 
/// on a raster image on a cell-to-cell basis. The user specifies the names of an input raster image (`--input`) 
/// and an output raster (`--output`), along with a conditional statement (`--statement`). The grid cell values 
/// in the output image will be determined by the TRUE and FALSE values and conditional statement. The conditional 
/// statement is a logical expression that must evaluate to either a Boolean, i.e. TRUE or FALSE. Then depending on 
/// how this statement evaluates for each grid cell, the TRUE or FALSE values will be assigned to the corresponding  
/// grid cells of the output raster. The TRUE or FALSE values may take the form of either a constant numerical value 
/// or a raster image (which may be the same image as the input). These are specified by the `--true` and `--false`
/// parameters, which can be either a file name pointing to existing rasters, or numerical values.
///
/// The conditional statement is a single-line logical condition. In additon to the common comparison and logical  
/// operators, i.e. < > <= >= == (EQUAL TO) != (NOT EQUAL TO) || (OR) && (AND), conditional statements may contain a  
/// number of valid mathematical functions. For example:
/// 
/// ```
///  * log(base=10, val) -- Logarithm with optional 'base' as first argument.
///  If not provided, 'base' defaults to '10'.
///  Example: log(100) + log(e(), 100)
/// 
///  * e()  -- Euler's number (2.718281828459045)
///  * pi() -- π (3.141592653589793)
/// 
///  * int(val)
///  * ceil(val)
///  * floor(val)
///  * round(modulus=1, val) -- Round with optional 'modulus' as first argument.
///      Example: round(1.23456) == 1 && round(0.001, 1.23456) == 1.235
/// 
///  * abs(val)
///  * sign(val)
/// 
///  * min(val, ...) -- Example: min(1, -2, 3, -4) == -4
///  * max(val, ...) -- Example: max(1, -2, 3, -4) == 3
/// 
///  * sin(radians)    * asin(val)
///  * cos(radians)    * acos(val)
///  * tan(radians)    * atan(val)
///  * sinh(val)       * asinh(val)
///  * cosh(val)       * acosh(val)
///  * tanh(val)       * atanh(val)
/// ```
///
/// Notice that the constants Pi and e must be specified as functions, pi() and e(). A number of 
/// global variables are also available to build conditional statements. These include the following:
/// 
/// **Special Variable Names For Use In Conditional Statements:**
///
/// | Name | Description |
/// | :-- | :-- |
/// | `value` | The grid cell value. |
/// | `nodata` | The input raster's NoData value. |
/// | `null` | Same as `nodata`. |
/// | `minvalue` | The input raster's minimum value. |
/// | `maxvalue` | The input raster's maximum value. |
/// | `rows` | The input raster's number of rows. |
/// | `columns` | The input raster's number of columns. |
/// | `row` | The grid cell's row number. |
/// | `column` | The grid cell's column number. |
/// | `rowy` | The row's y-coordinate. |
/// | `columnx` | The column's x-coordinate. |
/// | `north` | The input raster's northern coordinate. |
/// | `south` | The input raster's southern coordinate. |
/// | `east` | The input raster's eastern coordinate. |
/// | `west` | The input raster's western coordinate. |
/// | `cellsizex` | The input raster's grid resolution in the x-direction. |
/// | `cellsizey` | The input raster's grid resolution in the y-direction. |
/// | `cellsize` | The input raster's average grid resolution. |
///
/// The special variable names are case-sensitive. Each of the special variable names can also be used as valid 
/// TRUE or FALSE constant values.
///
/// The following are examples of valid conditional statements:
/// 
/// ```
/// value != 300.0
/// 
/// row > (rows / 2)
/// 
/// value >= (minvalue + 35.0)
/// 
/// (value >= 25.0) && (value <= 75.0)
/// 
/// tan(value * pi() / 180.0) > 1.0
/// 
/// value == nodata
/// ```
///
/// Any grid cell in the input raster containing the NoData value will be assigned NoData in the output raster, 
/// unless a NoData grid cell value allows the conditional statement to evaluate to True (i.e. the conditional 
/// statement includes the NoData value), in which case the True value will be assigned to the output.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}
            Err(e) => panic!("{:?}", e),
        }
    }

    if args.len() <= 1 || args[1].trim() == "help" {
        // print help
        help();
    }

    if args[1].trim() == "version" {
        // print version information
        version();
    }
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("conditional_evaluation{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    conditional_evaluation Help

    The Conditional Evaluation tool can be used to perform an if-then-else style conditional evaluation 
    on a raster image on a cell-to-cell basis.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input    Name of the input raster file.
    --statement    Conditional statement e.g. value > 35.0. This statement must be a valid Rust statement.
    --true         Value where condition evaluates TRUE (input raster or constant value).
    --false        Value where condition evaluates FALSE (input raster or constant value).
    -o, --output   Name of the output raster image file.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=DEM.tif --statement='value > 2500.0' --true=2500.0 --false=DEM.tif --output=onlyLowPlaces.tif
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "conditional_evaluation v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("ConditionalEvaluation") // This should be camel case and is a reference to the tool name.
}

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    let tool_name = get_tool_name();

    let sep: String = path::MAIN_SEPARATOR.to_string();

    // Read in the environment variables and get the necessary values
    let configurations = whitebox_common::configs::get_configs()?;
    let mut working_directory = configurations.working_directory.clone();
    if !working_directory.is_empty() && !working_directory.ends_with(&sep) {
        working_directory += &sep;
    }
    let max_procs = configurations.max_procs;

    // read the arguments
    let mut input_file = String::new();
    let mut con_statement = String::new();
    let mut true_value = String::new();
    let mut false_value = String::new();
    let mut output_file: String = String::new();
            
    if args.len() <= 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Tool run with too few parameters.",
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
        } else if flag_val == "-statement" {
            con_statement = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-true" {
            true_value = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-false" {
            false_value = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-output" {
            output_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        }
    }

    if configurations.verbose_mode {
        let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
        // 28 = length of the 'Powered by' by statement.
        println!("{}", "*".repeat(welcome_len));
        println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
        println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
        println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
        println!("{}", "*".repeat(welcome_len));
    }

    let mut progress: usize;
    let mut old_progress: usize = 1;

    let start = Instant::now();

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    // Read in the input file
    let input = Arc::new(Raster::new(&input_file, "r")?);
    let header = input.configs.clone();
    let rows = header.rows as isize;
    let columns = header.columns as isize;
    let nodata = header.nodata;

    // Are either of the Boolean inputs constants?
    let mut true_constant = f64::NEG_INFINITY;
    let mut is_true_a_constant = match true_value.parse::<f64>() {
        Ok(val) => {
            true_constant = val;
            true
        }
        Err(_) => false,
    };
    if !is_true_a_constant {
        if true_value.trim().is_empty() || 
        true_value.trim().to_lowercase() == "nodata" ||
        true_value.trim().to_lowercase() == "null" {
            true_constant = nodata;
            is_true_a_constant = true;
        } else if !true_value.contains(&sep) && !true_value.contains("/") {
            true_value = format!("{}{}", working_directory, true_value);
        }
    }

    let mut false_constant = f64::NEG_INFINITY;
    let mut is_false_a_constant = match false_value.parse::<f64>() {
        Ok(val) => {
            false_constant = val;
            true
        }
        Err(_) => false,
    };
    if !is_false_a_constant {
        if false_value.trim().is_empty() || 
        false_value.trim().to_lowercase() == "nodata" ||
        false_value.trim().to_lowercase() == "null" {
            false_constant = nodata;
            is_false_a_constant = true;
        } else if !false_value.contains(&sep) && !false_value.contains("/") {
            false_value = format!("{}{}", working_directory, false_value);
        }
    }

    con_statement = con_statement
        .replace("NoData", "nodata")
        .replace("Nodata", "nodata")
        .replace("NODATA", "nodata")
        .replace("null", "nodata")
        .replace("NULL", "nodata")
        .replace("Null", "nodata")
        .replace("COLS", "columns")
        .replace("Cols", "columns")
        .replace("cols", "columns")
        .replace("Columns", "columns")
        .replace("COL", "column")
        .replace("Col", "column")
        .replace("col", "column")
        .replace("ROWS", "rows")
        .replace("Rows", "rows")
        .replace("ROW", "row")
        .replace("Row", "row");

    let statement_contains_nodata = if con_statement.contains("nodata") {
        true
    } else {
        false
    };

    
    let mut output = Raster::initialize_using_config(&output_file, &header);

    let true_raster = if !is_true_a_constant {
        let r = Raster::new(&true_value, "r")?;
        if r.configs.rows as isize != rows || r.configs.columns as isize != columns {
            panic!("Error: All of the input rasters must share the same rows and columns.");
        }
        output.configs.data_type = r.configs.data_type;
        Some(r)
    } else {
        None
    };

    let false_raster = if !is_false_a_constant {
        let r = Raster::new(&false_value, "r")?;
        if r.configs.rows as isize != rows || r.configs.columns as isize != columns {
            panic!("Error: All of the input rasters must share the same rows and columns.");
        }
        output.configs.data_type = r.configs.data_type;
        Some(r)
    } else {
        None
    };

    let true_raster = Arc::new(true_raster);
    let false_raster = Arc::new(false_raster);

    let mut num_procs = num_cpus::get() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    // calculate the number of inflowing cells
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let tx = tx.clone();
        let con_statement = con_statement.clone();
        let true_raster = true_raster.clone();
        let false_raster = false_raster.clone();
        thread::spawn(move || {
            let mut value: f64;
            let mut ret_value: f64;
            // let mut ret: Option<Value>;
            let mut true_val: f64;
            let mut false_val: f64;
            let mut map : BTreeMap<String, f64> = BTreeMap::new();
            map.insert("nodata".to_string(), nodata);
            map.insert("rows".to_string(), rows as f64);
            map.insert("columns".to_string(), columns as f64);
            map.insert("north".to_string(), input.configs.north);
            map.insert("south".to_string(), input.configs.south);
            map.insert("east".to_string(), input.configs.east);
            map.insert("west".to_string(), input.configs.west);
            map.insert("cellsizex".to_string(), input.configs.resolution_x);
            map.insert("cellsizey".to_string(), input.configs.resolution_y);
            map.insert("cellsize".to_string(), (input.configs.resolution_x + input.configs.resolution_y)/2.0);
            map.insert("minvalue".to_string(), input.configs.minimum);
            map.insert("maxvalue".to_string(), input.configs.maximum);

            // let mut e = Eval::default()
            //     .insert("nodata", &format!("{}", nodata)).unwrap()
            //     .insert("rows", &format!("{}", rows)).unwrap()
            //     .insert("columns", &format!("{}", columns)).unwrap()
            //     .insert("minvalue", &format!("{}", input.configs.minimum)).unwrap()
            //     .insert("maxvalue", &format!("{}", input.configs.maximum)).unwrap()
            //     .insert("north", &format!("{}", input.configs.north)).unwrap()
            //     .insert("south", &format!("{}", input.configs.south)).unwrap()
            //     .insert("east", &format!("{}", input.configs.east)).unwrap()
            //     .insert("west", &format!("{}", input.configs.west)).unwrap()
            //     .insert("cellsizex", &format!("{}", input.configs.resolution_x)).unwrap()
            //     .insert("cellsizey", &format!("{}", input.configs.resolution_y)).unwrap()
            //     .insert("cellsize", &format!("{}", (input.configs.resolution_x + input.configs.resolution_y)/2.0)).unwrap();
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data: Vec<f64> = vec![nodata; columns as usize];
                map.insert("row".to_string(), row as f64);
                map.insert("rowy".to_string(), input.get_y_from_row(row));
                // e = e.insert("row", &format!("{}", row)).unwrap();
                // e = e.insert("rowy", &format!("{}", input.get_y_from_row(row))).unwrap();
                for col in 0..columns {
                    map.insert("column".to_string(), col as f64);
                    map.insert("columnx".to_string(), input.get_x_from_column(col));
                    // e = e.insert("column", &format!("{}", col)).unwrap();
                    // e = e.insert("columnx", &format!("{}", input.get_x_from_column(col))).unwrap();
                    value = input.get_value(row, col);
                    if value != nodata || statement_contains_nodata {
                        if let Some(ref tr) = *true_raster {
                            true_val = tr.get_value(row, col);
                            if true_val == tr.configs.nodata {
                                true_val = nodata;
                            }
                        } else {
                            true_val = true_constant;
                        }

                        if let Some(ref fr) = *false_raster {
                            false_val = fr.get_value(row, col);
                            if false_val == fr.configs.nodata {
                                false_val = nodata;
                            }
                        } else {
                            false_val = false_constant;
                        }

                        map.insert("value".to_string(), value);

                        let ret = fasteval::ez_eval(&con_statement, &mut map);
                        if ret.is_ok() {
                            ret_value = ret.unwrap();
                            if ret_value == 1f64 {
                                data[col as usize] = true_val;
                            } else {
                                data[col as usize] = false_val;
                            }
                        }

                        // e = e.insert("value", &format!("{}", value)).unwrap();
                        // ret = e.eval(&con_statement);
                        // if ret.is_some() {
                        //     ret_value = match ret.unwrap() {
                        //         Value::Bool(v) => v,
                        //         _ => false,
                        //     };
                        //     if ret_value {
                        //         data[col as usize] = true_val;
                        //     } else {
                        //         data[col as usize] = false_val;
                        //     }
                        // }
                    }
                }
                tx.send((row, data))
                    .expect("Error sending data to thread.");
            }
        });
    }

    for r in 0..rows {
        let (row, data) = rx.recv().expect("Error receiving data from thread.");
        output.set_row_data(row, data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Calculating index: {}%", progress);
                old_progress = progress;
            }
        }
    }
    
    if configurations.verbose_mode {
        println!("Saving data...")
    };

    let _ = match output.write() {
        Ok(_) => {
            if configurations.verbose_mode {
                println!("Output file written")
            }
        }
        Err(e) => return Err(e),
    };

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "\n{}",
            &format!("Elapsed Time (Including I/O): {}", elapsed_time)
        );
    }

    Ok(())
}
