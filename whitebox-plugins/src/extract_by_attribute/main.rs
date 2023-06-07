/* 
Authors:  Dr. John Lindsay
Created: 21/07/2021
Last Modified: 21/07/2021
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
// use std::sync::mpsc;
// use std::sync::Arc;
// use std::thread;
// use num_cpus;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_vector::{FieldData, Shapefile, ShapeType};
use evalexpr::*;

/// The 
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
    // let max_procs = configurations.max_procs;

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

                    // tx.send((i, true)).unwrap();
                // } else {
                //     tx.send((i, false)).unwrap();
                }
            // } else {
            //     tx.send((i, false)).unwrap();
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
