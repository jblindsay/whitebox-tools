/* 
Authors: Prof. John Lindsay
Created: 23/02/2022
Last Modified: 23/02/2022
License: MIT
*/
extern crate tsp_rs;

use tsp_rs::Tour;
use tsp_rs::Metrizable;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::str;
use std::time::Instant;
use whitebox_common::structures::Point2D;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_vector::{AttributeField, FieldData, FieldDataType, Shapefile, ShapefileGeometry, ShapeType};

/// This tool finds approximate solutions to [travelling salesman problems](https://en.wikipedia.org/wiki/Travelling_salesman_problem), 
/// the goal of which is to identify the shortest route connecting a set of locations. The tool uses
/// an algorithm that applies a [2-opt heuristic](https://en.wikipedia.org/wiki/2-opt) and a 
/// [3-opt](https://en.wikipedia.org/wiki/3-opt) heuristic as a fall-back if the initial approach 
/// takes too long. The user must specify the names of the input points vector (`--input`) and output lines
/// vector file (`--output`), as well as the duration, in seconds, over which the algorithm is allowed to search
/// for improved solutions (`--duration`).
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

    let exe_name = &format!("travelling_salesman_problem{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    travelling_salesman_problem Help

    This tool can be used to find approximate solutions to travelling salesman problems, which attempt to
    find the shortest route connecting a set of locations. 

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input    Name of the input lines shapefile.
    -o, --output   Name of the output lines shapefile.
    --length       Maximum segment length (m).
    
    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run -i=input.shp -o=line_segments.shp --length=100.0
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "travelling_salesman_problem v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("TravellingSalesmanProblem") // This should be camel case and is a reference to the tool name.
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
    let mut input_file = String::new();
    let mut output_file: String = String::new();
    let mut duration = 60u64;
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
        } else if flag_val == "-duration" {
            duration = if keyval {
                vec[1]
                    .to_string()
                    .parse::<u64>()
                    .expect(&format!("Error parsing {}", flag_val))
            } else {
                args[i + 1]
                    .to_string()
                    .parse::<u64>()
                    .expect(&format!("Error parsing {}", flag_val))
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

    let input = Shapefile::read(&input_file)?;

    // Make sure the input vector file is of point type
    if input.header.shape_type.base_shape_type() != ShapeType::Point {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The input vector data must be of Point base shape type.",
        ));
    }


    let mut tour: Vec<Point> = vec![];
    for record_num in 0..input.num_records {
        let record = input.get_record(record_num);
        if record.shape_type != ShapeType::Null {
            for i in 0..record.num_points as usize {
                tour.push(Point::new(
                    record.points[i].x,
                    record.points[i].y,
                ));
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
        println!("The tour includes {} locations.", tour.len());
    }


    let mut tour = Tour::from(&tour);

    if configurations.verbose_mode {
        println!("Finding optimal route, please be patient...");
    }

    tour.optimize_kopt(std::time::Duration::from_secs(duration));
    let tour_len = tour.tour_len();
    if configurations.verbose_mode {
        println!("Tour distance: {:.3}", tour_len);
    }

    // create output file
    let mut output = Shapefile::new(&output_file, ShapeType::PolyLine).expect("Error creating shapefile");
    output.projection = input.projection.clone();
    output.attributes.add_field(&AttributeField::new("FID", FieldDataType::Int, 3u8, 0u8));
    output.attributes.add_field(&AttributeField::new("LENGTH", FieldDataType::Real, 9u8, 3u8));

    let mut vec_pts = vec![];
    for pt in tour.path {
        vec_pts.push(Point2D::new(pt.x, pt.y));
    }
    let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
    sfg.add_part(&vec_pts);
    output.add_record(sfg);
    output
        .attributes
        .add_record(vec![FieldData::Int(1i32), FieldData::Real(tour_len)], false);

    if configurations.verbose_mode {
        println!("Saving data...")
    };
    output.write().expect("Error saving Shapefile");

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!(
            "\n{}",
            &format!("Elapsed Time (Including I/O): {}", elapsed_time)
        );
    }

    
    Ok(())
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

impl Metrizable for Point {
    fn cost(&self, other: &Point) -> f64 {
        // (self.x - other.x)*(self.x - other.x) + (self.y - other.y)*(self.y - other.y)
        ((self.x - other.x)*(self.x - other.x) + (self.y - other.y)*(self.y - other.y)).sqrt()
    }
}

// impl Metrizable for Point2D {
//     fn cost(&self, other: &Point2D) -> f64 {
//         ((self.x - other.x)*(self.x - other.x) + (self.y - other.y)*(self.y - other.y)).sqrt()
//     }
// }