/* 
Authors:  Dr. John Lindsay
Created: 21/07/2021
Last Modified: 21/07/2021
License: MIT
*/

use std::collections::{BTreeMap, HashSet};
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

/// The RasterCalculator tool can be used to perform a complex mathematical operations on one or more input
/// raster images on a cell-to-cell basis. The user specifies the name of the output raster (`--output`)
/// and a mathematical expression, or statement (`--statement`). Rasters are treated like variables (that
/// change value with each grid cell) and are specified within the statement with the file name contained 
/// within either double or single quotation marks (e.g. "DEM.tif" > 500.0). Raster variables may or may not include the file directory.
/// If unspecified, a raster is assumed to exist within the working directory. Similarly, if the file extension
/// is unspecified, it is assumed to be '.tif'. **Note, all input rasters must share the same number of rows
/// and columns and spatial extent. Use the `Resample` tool if this is not the case to convert the one raster's
/// grid resolution to the others.
///
/// The mathematical expression supports all of the standard algebraic unary and binary operators (+ - * / ^ %), 
/// as well as comparisons (< <= == != >= >) and logical operators (&& ||) with short-circuit support. The
/// order of operations, from highest to lowest is as follows.
///
/// Listed in order of precedence:
///
/// | Order | Symbol | Description |
/// | -: | :- | :- |
/// |  (Highest Precedence) | ^               | Exponentiation |
/// |                       | %               | Modulo |
/// |                       | /               | Division |
/// |                       | *               | Multiplication |
/// |                       | -               | Subtraction |
/// |                       | +               | Addition |
/// |                       | == != < <= >= > | Comparisons (all have equal precedence) |
/// |                       | && and          | Logical AND with short-circuit |
/// |  (Lowest Precedence)  | &#124;&#124; or | Logical OR with short-circuit |
/// |  
///
/// Several common mathematical functions are also available for use in the input statement. For example:
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
/// Notice that the constants pi and e must be specified as functions, `pi()` and `e()`. A number of global variables 
/// are also available to build conditional statements. These include the following:
/// 
/// **Special Variable Names For Use In Conditional Statements:**
///
/// | Name | Description |
/// | :-- | :-- |
/// | `nodata` | An input raster's NoData value. |
/// | `null` | Same as `nodata`. |
/// | `minvalue` | An input raster's minimum value. |
/// | `maxvalue` | An input raster's maximum value. |
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
/// The special variable names are case-sensitive. If there are more than one raster inputs used in the statement,
/// the functional forms of the `nodata`, `null`, `minvalue`, and `maxvalue` variables should be used, e.g. 
/// `nodata("InputRaster")`, otherwise the value is assumed to specify the attribute of the first raster in the 
/// statement. The following are examples of valid statements:
/// 
/// ```
///  "raster" != 300.0
/// 
///  "raster" >= (minvalue + 35.0)
/// 
///  ("raster1" >= 25.0) && ("raster2" <= 75.0) -- Evaluates to 1 where both conditions are true.
/// 
///  tan("raster" * pi() / 180.0) > 1.0
/// 
///  "raster" == nodata
/// ```
///
/// Any grid cell in the input rasters containing the NoData value will be assigned NoData in the output raster, 
/// unless a NoData grid cell value allows the statement to evaluate to True (i.e. the mathematical expression 
/// includes the `nodata` value).
///
/// # See Also
/// `ConditionalEvaluation`
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

    let exe_name = &format!("raster_calculator{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    raster_calculator Help

    The RasterCalculator tool can be used to perform an complex mathematical operations on one or more input
    raster images on a cell-to-cell basis.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -o, --output   Name of the output raster file.
    --statement    Statement of a mathematical expression e.g. "raster1" > 35.0.
    
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
        "raster_calculator v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("RasterCalculator") // This should be camel case and is a reference to the tool name.
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
    let mut statement = String::new();
    let mut output_file: String = String::new();
            
    if args.len() <= 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Tool run with too few parameters.",
        ));
    }
    for i in 0..args.len() {
        let arg = if !args[i].contains("--statement") {
            args[i].replace("\"", "").replace("\'", "")
        } else {
            args[i].clone()
        };
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        let mut keyval = false;
        if vec.len() > 1 {
            keyval = true;
        }
        let flag_val = vec[0].to_lowercase().replace("--", "-");
        if flag_val == "-o" || flag_val == "-output" {
            output_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if arg.contains("-statement") {
            statement = arg.replace("--statement=", "")
                           .replace("-statement=", "")
                           .replace("--statement", "")
                           .replace("-statement", "");
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

    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    // We need to find and read the input files
    let mut delimiter = "\"";
    let mut num_quotation_marks = statement.matches(delimiter).count();
    if num_quotation_marks == 0 {
        delimiter = "'";
        num_quotation_marks = statement.matches(delimiter).count();
        if num_quotation_marks == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "No rasters specified.",
            ));
        }
    }
    if num_quotation_marks % 2 != 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Unmatched quotation marks.",
        ));
    }
    let vals = statement.split(delimiter).collect::<Vec<&str>>();
    let mut input_hs = HashSet::new();
    for i in (0..vals.len()).filter(|i| i % 2 == 1) {
        input_hs.insert(vals[i].to_string());
    }

    let mut input_files = Vec::with_capacity(input_hs.len());
    for v in input_hs {
        input_files.push(v);
    }

    let num_inputs = input_files.len();
    for i in 0..num_inputs {
        statement = statement.replace(&format!("{}{}{}", delimiter, input_files[i], delimiter), &format!("value{}", i));
    }
    statement = statement.replace("'", "");

    for i in 0..num_inputs {
        if !input_files[i].contains(".") {
            input_files[i].push_str(".tif");
        }
        if !input_files[i].contains(&sep) && !input_files[i].contains("/") {
            input_files[i] = format!("{}{}", working_directory, input_files[i]);
        }
    }

    ////////////////////////////
    // Open the raster images //
    ////////////////////////////
    let mut rows = -1isize;
    let mut columns = -1isize;

    let mut nodata = vec![0f64; num_inputs];
    let mut input_raster: Vec<Raster> = Vec::with_capacity(num_inputs);
    // let mut file_names = vec![];
    if configurations.verbose_mode {
        println!("Reading data...");
    }
    for i in 0..num_inputs {
        if !input_files[i].trim().is_empty() {
            // quality control on the image file name.
            let mut input_file = input_files[i].trim().to_owned();
            if !input_file.contains(&sep) && !input_file.contains("/") {
                input_file = format!("{}{}", working_directory, input_file);
            }

            // read the image
            input_raster.push(Raster::new(&input_file, "r")?);

            // get the nodata value, the number of valid cells, and the average
            nodata[i] = input_raster[i].configs.nodata;
            // file_names.push(input_raster[i].get_short_filename());

            // initialize the rows and column and check that each image has the same dimensions
            if rows == -1 || columns == -1 {
                rows = input_raster[i].configs.rows as isize;
                columns = input_raster[i].configs.columns as isize;
            } else {
                if input_raster[i].configs.rows as isize != rows
                    || input_raster[i].configs.columns as isize != columns
                {
                    return Err(Error::new(ErrorKind::InvalidInput,
                        "All input images must share the same dimensions (rows and columns) and spatial extent."));
                }
            }
        } else {
            return Err(Error::new(ErrorKind::InvalidInput,
                "There is something incorrect with the input files. At least one is an empty string."));
        }

        if configurations.verbose_mode {
            progress = (100.0_f64 * i as f64 / (num_inputs - 1) as f64) as usize;
            if progress != old_progress {
                println!("Reading data: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if rows == -1 || columns == -1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Something is incorrect with the specified input files.",
        ));
    }

    let statement_contains_nodata = if statement.contains("nodata") {
        true
    } else {
        false
    };
    
    for i in 0..num_inputs {
        statement = statement.replace(&format!("nodata(value{})", i), &format!("{}", nodata[i]));
        statement = statement.replace("nodata()", &format!("{}", nodata[0]));
        statement = statement.replace("nodata", &format!("{}", nodata[0]));
        statement = statement.replace(&format!("null(value{})", i), &format!("{}", nodata[i]));
        statement = statement.replace("null()", &format!("{}", nodata[0]));
        statement = statement.replace("null", &format!("{}", nodata[0]));
        statement = statement.replace(&format!("minvalue(value{})", i), &format!("{}", input_raster[i].configs.minimum));
        statement = statement.replace("minvalue()", &format!("{}", input_raster[0].configs.minimum));
        statement = statement.replace("minvalue", &format!("{}", input_raster[0].configs.minimum));
        statement = statement.replace(&format!("maxvalue(value{})", i), &format!("{}", input_raster[i].configs.maximum));
        statement = statement.replace("maxvalue()", &format!("{}", input_raster[0].configs.maximum));
        statement = statement.replace("maxvalue", &format!("{}", input_raster[0].configs.maximum));
    }

    statement = statement
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

    
    let mut output = Raster::initialize_using_config(&output_file, &input_raster[0].configs.clone());
    let out_nodata = -32_768f64;
    output.configs.nodata = out_nodata;
    
    let mut num_procs = num_cpus::get() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    let input_raster = Arc::new(input_raster);
    let nodata = Arc::new(nodata);
    // calculate the number of inflowing cells
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let tx = tx.clone();
        let statement = statement.clone();
        let input_raster = input_raster.clone();
        let nodata = nodata.clone();
        thread::spawn(move || {
            let mut value: f64;
            let mut is_nodata: bool;
            let mut map : BTreeMap<String, f64> = BTreeMap::new();
            map.insert("rows".to_string(), rows as f64);
            map.insert("columns".to_string(), columns as f64);
            map.insert("north".to_string(), input_raster[0].configs.north);
            map.insert("south".to_string(), input_raster[0].configs.south);
            map.insert("east".to_string(), input_raster[0].configs.east);
            map.insert("west".to_string(), input_raster[0].configs.west);
            map.insert("cellsizex".to_string(), input_raster[0].configs.resolution_x);
            map.insert("cellsizey".to_string(), input_raster[0].configs.resolution_y);
            map.insert("cellsize".to_string(), (input_raster[0].configs.resolution_x + input_raster[0].configs.resolution_y)/2.0);

            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data: Vec<f64> = vec![out_nodata; columns as usize];
                map.insert("row".to_string(), row as f64);
                map.insert("rowy".to_string(), input_raster[0].get_y_from_row(row));
                for col in 0..columns {
                    map.insert("column".to_string(), col as f64);
                    map.insert("columnx".to_string(), input_raster[0].get_x_from_column(col));
                    is_nodata = false;
                    for i in 0..num_inputs {
                        value = input_raster[i].get_value(row, col);
                        if value == nodata[i] { is_nodata = true; }
                        map.insert(format!("value{}", i), value);
                    }
                    if !is_nodata || statement_contains_nodata {
                        let ret = fasteval::ez_eval(&statement, &mut map);
                        if ret.is_ok() {
                            value = ret.unwrap();
                            data[col as usize] = value;
                        }
                    }
                }
                tx.send((row, data))
                    .expect("Error sending data to thread.");
            }
        });
    }

    let mut is_float_data = false;
    for r in 0..rows {
        let (row, data) = rx.recv().expect("Error receiving data from thread.");
        
        if !is_float_data {
            for i in 0..data.len() {
                if data[i] != nodata[0] {
                    if data[i].round() != data[i] {
                        is_float_data = true;
                        break;
                    }
                }
            }
        }

        output.set_row_data(row, data);

        if configurations.verbose_mode {
            progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if is_float_data {
        // Are any of the inputs F64
        let mut is_f64 = false;
        for i in 0..num_inputs {
            if input_raster[i].configs.data_type == DataType::F64 {
                is_f64 = true;
                break;
            }
        }
        if !is_f64 {
            output.configs.data_type = DataType::F32;
        } else {
            output.configs.data_type = DataType::F64;
        }
    } else {
        output.update_min_max();
        if output.configs.minimum >= -32_768f64 && output.configs.maximum <= 32_767f64 {
            output.configs.data_type = DataType::I16;
        } else if output.configs.minimum >= -2_147_483_648f64 && output.configs.maximum <= 2_147_483_647f64 {
            output.configs.data_type = DataType::I32;
        } else {
            output.configs.data_type = DataType::I64;
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
