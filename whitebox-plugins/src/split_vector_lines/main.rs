/* 
Authors: Prof. John Lindsay
Created: 23/04/2021
Last Modified: 23/04/2021
License: MIT
*/

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_common::structures::{Point2D};
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_vector::{AttributeField, FieldData, FieldDataType, Shapefile, ShapefileGeometry, ShapeType};

/// This tool can be used to divide longer vector lines (`--input`) into segments of a maximum specified length
/// (`--length`).
///
/// # See Also
/// `AssessRoute`
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

    let exe_name = &format!("split_vector_lines{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    split_vector_lines Help

    This tool can be used to evaluate the distributions of reflectance values for a series
    of training site polygons in multiple mult-spectral images. 

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input    Name of the input raster image file.
    -o, --output   Name of the output HTML file.
    -s, --sigma    Sigma value used in Gaussian filtering, default = 1.0
    -l, --low      Low threshold, ranges from 0.0-1.0, default = 0.05
    -h, --high     High threshold, ranges from 0.0-1.0, default = 0.15
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=input.tif -o=new.tif --sigma=0.25 --low=0.1 --high=0.2

    Note: Use of this tool requires a valid license. To obtain a license,
    contact Whitebox Geospatial Inc. (whiteboxgeo@gmail.com).
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "split_vector_lines v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("SplitVectorLines") // This should be camel case and is a reference to the tool name.
}

// fn get_toolset() -> String {
//     String::from("GeneralToolset") // This should be CamelCase.
// }

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
    let mut input_file = String::new();
    let mut output_file: String = String::new();
    let mut segment_length = 100f64;
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
        } else if flag_val == "-o" || flag_val == "-output" {
            output_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-length" {
            segment_length = if keyval {
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

    if configurations.verbose_mode {
        println!("***************{}", "*".repeat(tool_name.len()));
        println!("* Welcome to {} *", tool_name);
        println!("***************{}", "*".repeat(tool_name.len()));
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

    // Make sure the input vector file is of polygon type
    if input.header.shape_type.base_shape_type() != ShapeType::PolyLine {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The input vector data must be of PolyLine base shape type.",
        ));
    }


    // create output file
    let mut output = Shapefile::initialize_using_file(&output_file, &input, ShapeType::PolyLine, false)?;

    // add the attributes
    let mut fields_vec: Vec<AttributeField> = vec![];
    fields_vec.push(
        AttributeField::new(
            "FID", 
            FieldDataType::Int, 
            7u8, 
            0u8
        )
    );

    fields_vec.push(
        AttributeField::new(
            "PARENT_ID", 
            FieldDataType::Int, 
            7u8, 
            0u8
        )
    );

    let in_atts = input.attributes.clone();
    let mut parent_fid_att = 999;
    for i in 0..in_atts.fields.len() {
        let field = in_atts.get_field(i);
        if field.name == "FID" {
            parent_fid_att = i;
        } else {
            fields_vec.push(field.clone());
        }
    }

    // println!("parent_fid_att: {}", parent_fid_att);
    // for i in 0..fields_vec.len() {
    //     println!("{:?}", fields_vec[i]);
    // }

    output.attributes.add_fields(&fields_vec);

    let (mut part_start, mut part_end): (usize, usize);
    // let mut points_in_part: usize;
    let (mut x, mut x1, mut x2, mut y, mut y1, mut y2): (f64, f64, f64, f64, f64, f64);
    let mut dist: f64;
    let mut dist_between_points: f64;
    let mut ratio: f64;
    let mut fid = 1i32;
    let mut att_data: Vec<FieldData>;
    for record_num in 0..input.num_records {
        let record = input.get_record(record_num);        
        for part in 0..record.num_parts as usize {
            let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);

            part_start = record.parts[part] as usize;
            part_end = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };

            let mut points: Vec<Point2D> = vec![];
            points.push(record.points[0].clone());
            dist = 0f64;

            let mut i = part_start+1;
            while i <= part_end {
                x1 = points[points.len()-1].x;
                y1 = points[points.len()-1].y;
                
                x2 = record.points[i].x;
                y2 = record.points[i].y;

                dist_between_points = ((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)).sqrt();
                if dist + dist_between_points <= segment_length && dist_between_points > 0f64 {
                    points.push(Point2D::new(x2, y2));
                    dist += dist_between_points;
                } else if dist_between_points > 0f64 {
                    ratio = (segment_length - dist) / dist_between_points;
                    x = x1 + ratio * (x2 - x1);
                    y = y1 + ratio * (y2 - y1);
                    points.push(Point2D::new(x, y));

                    sfg.add_part(&points);
                    output.add_record(sfg);
                    att_data = vec![
                        FieldData::Int(fid),
                        FieldData::Int(record_num as i32 + 1i32),
                    ];
                    let in_atts = input.attributes.get_record(record_num);
                    for a in 0..in_atts.len() {
                        if a != parent_fid_att {
                            att_data.push(in_atts[a].clone());
                        }
                    }
                    output.attributes.add_record(att_data.clone(), false);
                    
                    // reinitialize
                    sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                    points = vec![];
                    points.push(Point2D::new(x, y));
                    dist = 0f64;
                    i -= 1;
                    fid += 1;
                }

                i += 1;
            }

            if points.len() > 1 {
                sfg.add_part(&points);
                output.add_record(sfg);
                // let atts = input.attributes.get_record(record_num);
                att_data = vec![
                    FieldData::Int(fid),
                    FieldData::Int(record_num as i32 + 1i32),
                ];
                let in_atts = input.attributes.get_record(record_num);
                for a in 0..in_atts.len() {
                    if a != parent_fid_att {
                        att_data.push(in_atts[a].clone());
                    }
                }
                output.attributes.add_record(att_data.clone(), false);
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