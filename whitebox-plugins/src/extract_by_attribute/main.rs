/* 
Authors:  Dr. John Lindsay
Created: 03/06/2023
Last Modified: 03/06/2023
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_vector::{FieldData, Shapefile, ShapeType};
use evalexpr::*;
use std::f64::consts::PI;

/// This tool extracts features from an input vector into an output file based on attribute properties. The user must
/// specify the name of the input (`--input`) and output (`--output`) files, along with the filter statement (`--statement`).
/// The conditional statement is a single-line logical condition containing one or more attribute variables contained in
/// the file's attribute table that evaluates to TRUE/FALSE. In addition to the common comparison and logical  
/// operators, i.e. < > <= >= == (EQUAL TO) != (NOT EQUAL TO) || (OR) && (AND), conditional statements may contain a  
/// any valid mathematical operation and the `null` value. 
/// 
/// | Identifier           | Argument Amount | Argument Types                | Description |
/// |----------------------|-----------------|-------------------------------|-------------|
/// | `min`                | >= 1            | Numeric                       | Returns the minimum of the arguments |
/// | `max`                | >= 1            | Numeric                       | Returns the maximum of the arguments |
/// | `len`                | 1               | String/Tuple                  | Returns the character length of a string, or the amount of elements in a tuple (not recursively) |
/// | `floor`              | 1               | Numeric                       | Returns the largest integer less than or equal to a number |
/// | `round`              | 1               | Numeric                       | Returns the nearest integer to a number. Rounds half-way cases away from 0.0 |
/// | `ceil`               | 1               | Numeric                       | Returns the smallest integer greater than or equal to a number |
/// | `if`                 | 3               | Boolean, Any, Any             | If the first argument is true, returns the second argument, otherwise, returns the third  |
/// | `contains`           | 2               | Tuple, any non-tuple          | Returns true if second argument exists in first tuple argument. |
/// | `contains_any`       | 2               | Tuple, Tuple of any non-tuple | Returns true if one of the values in the second tuple argument exists in first tuple argument. |
/// | `typeof`             | 1               | Any                           | returns "string", "float", "int", "boolean", "tuple", or "empty" depending on the type of the argument  |
/// | `math::is_nan`       | 1               | Numeric                       | Returns true if the argument is the floating-point value NaN, false if it is another floating-point value, and throws an error if it is not a number  |
/// | `math::is_finite`    | 1               | Numeric                       | Returns true if the argument is a finite floating-point number, false otherwise  |
/// | `math::is_infinite`  | 1               | Numeric                       | Returns true if the argument is an infinite floating-point number, false otherwise  |
/// | `math::is_normal`    | 1               | Numeric                       | Returns true if the argument is a floating-point number that is neither zero, infinite, [subnormal](https://en.wikipedia.org/wiki/Subnormal_number), or NaN, false otherwise  |
/// | `math::ln`           | 1               | Numeric                       | Returns the natural logarithm of the number |
/// | `math::log`          | 2               | Numeric, Numeric              | Returns the logarithm of the number with respect to an arbitrary base |
/// | `math::log2`         | 1               | Numeric                       | Returns the base 2 logarithm of the number |
/// | `math::log10`        | 1               | Numeric                       | Returns the base 10 logarithm of the number |
/// | `math::exp`          | 1               | Numeric                       | Returns `e^(number)`, (the exponential function) |
/// | `math::exp2`         | 1               | Numeric                       | Returns `2^(number)` |
/// | `math::pow`          | 2               | Numeric, Numeric              | Raises a number to the power of the other number |
/// | `math::cos`          | 1               | Numeric                       | Computes the cosine of a number (in radians) |
/// | `math::acos`         | 1               | Numeric                       | Computes the arccosine of a number. The return value is in radians in the range [0, pi] or NaN if the number is outside the range [-1, 1] |
/// | `math::cosh`         | 1               | Numeric                       | Hyperbolic cosine function |
/// | `math::acosh`        | 1               | Numeric                       | Inverse hyperbolic cosine function |
/// | `math::sin`          | 1               | Numeric                       | Computes the sine of a number (in radians) |
/// | `math::asin`         | 1               | Numeric                       | Computes the arcsine of a number. The return value is in radians in the range [-pi/2, pi/2] or NaN if the number is outside the range [-1, 1] |
/// | `math::sinh`         | 1               | Numeric                       | Hyperbolic sine function |
/// | `math::asinh`        | 1               | Numeric                       | Inverse hyperbolic sine function |
/// | `math::tan`          | 1               | Numeric                       | Computes the tangent of a number (in radians) |
/// | `math::atan`         | 1               | Numeric                       | Computes the arctangent of a number. The return value is in radians in the range [-pi/2, pi/2] |
/// | `math::atan2`        | 2               | Numeric, Numeric              | Computes the four quadrant arctangent in radians |
/// | `math::tanh`         | 1               | Numeric                       | Hyperbolic tangent function |
/// | `math::atanh`        | 1               | Numeric                       | Inverse hyperbolic tangent function. |
/// | `math::sqrt`         | 1               | Numeric                       | Returns the square root of a number. Returns NaN for a negative number |
/// | `math::cbrt`         | 1               | Numeric                       | Returns the cube root of a number |
/// | `math::hypot`        | 2               | Numeric                       | Calculates the length of the hypotenuse of a right-angle triangle given legs of length given by the two arguments |
/// | `math::abs`          | 1               | Numeric                       | Returns the absolute value of a number, returning an integer if the argument was an integer, and a float otherwise |
/// | `str::regex_matches` | 2               | String, String                | Returns true if the first argument matches the regex in the second argument (Requires `regex_support` feature flag) |
/// | `str::regex_replace` | 3               | String, String, String        | Returns the first argument with all matches of the regex in the second argument replaced by the third argument (Requires `regex_support` feature flag) |
/// | `str::to_lowercase`  | 1               | String                        | Returns the lower-case version of the string |
/// | `str::to_uppercase`  | 1               | String                        | Returns the upper-case version of the string |
/// | `str::trim`          | 1               | String                        | Strips whitespace from the start and the end of the string |
/// | `str::from`          | >= 0            | Any                           | Returns passed value as string |
/// | `bitand`             | 2               | Int                           | Computes the bitwise and of the given integers |
/// | `bitor`              | 2               | Int                           | Computes the bitwise or of the given integers |
/// | `bitxor`             | 2               | Int                           | Computes the bitwise xor of the given integers |
/// | `bitnot`             | 1               | Int                           | Computes the bitwise not of the given integer |
/// | `shl`                | 2               | Int                           | Computes the given integer bitwise shifted left by the other given integer |
/// | `shr`                | 2               | Int                           | Computes the given integer bitwise shifted right by the other given integer |
/// | `random`             | 0               | Empty                         | Return a random float between 0 and 1. Requires the `rand` feature flag. |
/// | `pi`                 | 0               | Empty                         | Return the value of the PI constant. |/
///
/// The following are examples of valid conditional statements:
/// 
/// ```
/// HEIGHT >= 300.0
/// 
/// CROP == "corn"
/// 
/// (ELEV >= 525.0) && (HGT_AB_GR <= 5.0)
/// 
/// math::ln(CARBON) > 1.0
/// 
/// VALUE == null
/// ```
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

    let exe_name = &format!("extract_by_attribute{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    extract_by_attribute Help

    The ExtractByAttribute tool can be used to perform an complex mathematical operations on one or more input
    raster images on a cell-to-cell basis.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -o, --output   Name of the output vector file.
    --statement    Statement of a mathematical expression e.g. "raster1" > 35.0.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=input.shp -o=output.shp --statement=\"ELEV>500.0\"
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "extract_by_attribute v{} by Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("ExtractByAttribute") // This should be camel case and is a reference to the tool name.
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
    // read the arguments
    let mut statement = String::new();
    let mut input_file: String = String::new();
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

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }

    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    let input = Shapefile::read(&input_file)?;

    let precompiled = build_operator_tree(&statement).unwrap(); // Do proper error handling here

    // create output file
    let mut output = Shapefile::initialize_using_file(&output_file, &input, input.header.shape_type, true)?;

    let att_fields = input.attributes.get_fields();
    let mut contains_fid = false;
    for i in 0..att_fields.len() {
        if att_fields[i].name.to_lowercase() == "fid" {
            contains_fid = true;
            break;
        }
    }

    let mut num_extracted = 0;
    for record_num in 0..input.num_records {
        let record = input.get_record(record_num);
        if record.shape_type != ShapeType::Null {
            let att_data = input.attributes.get_record(record_num);
            let mut context = HashMapContext::new();
            for i in 0..att_fields.len() {
                match &att_data[i] {
                    FieldData::Int(val) => {
                        _ = context.set_value(att_fields[i].name.clone().into(), (*val as i64).into());
                    }
                    FieldData::Real(val) => {
                        _ = context.set_value(att_fields[i].name.clone().into(), (*val).into());
                    },
                    FieldData::Text(val) => {
                        _ = context.set_value(att_fields[i].name.clone().into(), (&**val).into());
                    },
                    FieldData::Date(val) => {
                        _ = context.set_value(att_fields[i].name.clone().into(), format!("{}", *val).into());
                    },
                    FieldData::Bool(val) => {
                        _ = context.set_value(att_fields[i].name.clone().into(), (*val).into());
                    },
                    FieldData::Null => {
                        _ = context.set_value(att_fields[i].name.clone().into(), "null".into());
                    }
                }
            }

            _ = context.set_value("null".into(), "null".into());
            _ = context.set_value("NULL".into(), "null".into());
            _ = context.set_value("none".into(), "null".into());
            _ = context.set_value("NONE".into(), "null".into());
            _ = context.set_value("nodata".into(), "null".into());
            _ = context.set_value("NoData".into(), "null".into());
            _ = context.set_value("NODATA".into(), "null".into());
            _ = context.set_value("pi".into(), (PI).into());
            _ = context.set_value("PI".into(), (PI).into());
            
            if !contains_fid { // add the FID
                _ = context.set_value("FID".into(), (record_num as i64).into());
            }

            // let ret = eval_boolean_with_context(&statement, &context);
            let ret = precompiled.eval_boolean_with_context(&context);
            if ret.is_ok() {
                let value = ret.unwrap_or(false);
                if value {
                    output.add_record(record.clone());
                    output.attributes.add_record(att_data.clone(), false);
                    num_extracted += 1;
                }
            }
        }
        
        if configurations.verbose_mode {
            progress =
                (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
            if progress != old_progress {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }
    }

    if configurations.verbose_mode {
        println!("Number of extracted features: {num_extracted}");
    }

    if num_extracted > 0 {
        if configurations.verbose_mode {
            println!("Saving data...")
        }
        let _ = match output.write() {
            Ok(_) => {
                if configurations.verbose_mode {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };
    } else {
        println!("WARNING: No features were selected and therefore no output file will be written.");
    }

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "\n{}",
            &format!("Elapsed Time (Including I/O): {}", elapsed_time)
        );
    }

    Ok(())
}
