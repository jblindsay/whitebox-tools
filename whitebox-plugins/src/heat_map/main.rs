/* 
Authors:  Dr. John Lindsay
Created: 01/06/2022
Last Modified: 01/06/2022
License: MIT
*/
extern crate kd_tree;

use kd_tree::{KdPoint, KdTree};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::panic;
use std::path;
use std::process;
use std::str;
use std::time::Instant;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use num_cpus;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;
use whitebox_vector::{
    FieldData,
    ShapeType, 
    Shapefile
};

/// This tool is used to generate a raster heat map, or [kernel density estimation](https://en.wikipedia.org/wiki/Kernel_density_estimation)
/// surface raster from a set of vector points (`--input`). Heat mapping is a visualization and modelling technique
/// used to create the continuous density surface associated with the occurrences of a point phenomenon. Heat maps can 
/// therefore be used to identify point clusters by mapping the concentration of event occurrence. For example, heat
/// maps have been used extensively to map the spatial distributions of crime events (i.e. crime mapping) or disease cases.
/// 
/// By default, the tool maps the density of raw occurrence events, however, the user may optionally specify an associated
/// weights field (`--weights`) from the point file's attribute table. When a weights field is specified, these values 
/// are simply multiplied by each of the individual components of the density estimate. Weights must be numeric.
/// 
/// The bandwidth parameter (--bandwidth) determines the radius of the [kernel](https://en.wikipedia.org/wiki/Kernel_%28statistics%29) 
/// used in calculation of the density surface. There are [guidelines](https://en.wikipedia.org/wiki/Kernel_density_estimation#Bandwidth_selection) 
/// that statisticians use in determining an appropriate bandwidth for a particular population and data set, but often 
/// this parameter is determined through experimentation. The bandwidth of the kernel is a free parameter which exhibits 
/// a strong influence on the resulting estimate. 
/// 
/// The user must specify the kernel [function type](https://en.wikipedia.org/wiki/Kernel_%28statistics%29#Kernel_functions_in_common_use) 
/// (`--kernel`). Options include 'uniform', 'triangular', 'epanechnikov', 'quartic', 'triweight', 'tricube', 'gaussian', 'cosine', 
/// 'logistic', 'sigmoid', and 'silverman'; 'quartic' is the default kernel type. Descriptions of each function can be found at the 
/// link above.
/// 
/// The characteristics of the output raster (resolution and extent) are determined by one of two optional parameters,
/// `--cell_size` and `--base`. If the user optionally specifies the output grid cell size parameter (`--cell_size`) 
/// then the coordinates of the output raster extent are determined by the input vector (i.e. the bounding box) and 
/// the specified cell size determines the number of rows and columns. If the user instead specifies the optional 
/// base raster file parameter (`--base`), the output raster's coordinates (i.e. north, south, east, west) and row 
/// and column count, and therefore, resolution, will be the same as the base file.
/// 
/// # Reference
/// Geomatics (2017) QGIS Heatmap Using Kernel Density Estimation Explained, online resource: [https://www.geodose.com/2017/11/qgis-heatmap-using-kernel-density.html](https://www.geodose.com/2017/11/qgis-heatmap-using-kernel-density.html)
/// visited 02/06/2022.
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

    let exe_name = &format!("heat_map{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    heat_map Help

    This tool is used to generate a flow accumulation grid (i.e. contributing area) using the Qin et al. (2007) 
    flow algorithm.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file; must be depressionless.
    --output       Name of the output raster file.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

     "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "heat_map v{} by Dr. John B. Lindsay (c) 2022.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("HeatMap") // This should be camel case and is a reference to the tool name.
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
    let mut field_name: Option<String> = None;
    let mut output_file: String = String::new();
    let mut bandwidth = 0f64;
    let mut cell_size = 0f64;
    let mut base_file: String = String::new();
    let mut kernel_function: String = String::from("quartic");

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
        } else if flag_val == "-weight_field" || flag_val == "-weights" {
            field_name = if keyval {
                Some(vec[1].to_string())
            } else {
                Some(args[i + 1].to_string())
            };
        } else if flag_val == "-output" {
            output_file = if keyval {
                vec[1].to_string()
            } else {
                args[i + 1].to_string()
            };
        } else if flag_val == "-bandwidth" {
            bandwidth = if keyval {
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
        } else if flag_val == "-kernel" {
            kernel_function = if keyval {
                vec[1].to_string().to_lowercase()
            } else {
                args[i + 1].to_string().to_lowercase()
            };
        } else if flag_val == "-cell_size" {
            cell_size = if keyval {
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
        } else if flag_val == "-base" {
            base_file = if keyval {
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

    if bandwidth <= 0f64 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The bandwidth parameter must be larger than 0.",
        ));
    }

    if configurations.verbose_mode {
        println!("Reading data...")
    };
    let vector_points = Shapefile::read(&input_file)?;

    // make sure the input vector file is of points type
    if vector_points.header.shape_type.base_shape_type() != ShapeType::Point {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The input vector data must be of point base shape type.",
        ));
    }

    let mut n_points = 0;
    for record_num in 0..vector_points.num_records {
        let record = vector_points.get_record(record_num);
        for _ in 0..record.num_points as usize {
            n_points += 1;
        }
    }

    // What is the index of the field to be analyzed?
    if field_name.is_some() {
        let field_index = match vector_points.attributes.get_field_num(&field_name.as_ref().unwrap()) {
            Some(i) => i,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Error: Weight attribute not found in table.",
                ));
            }
        };
    
        // Is the field numeric?
        if !vector_points.attributes.is_field_numeric(field_index) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: Non-numeric weight attributes cannot be used.",
            ));
        }
    }

    let mut points: Vec<TreeItem> = Vec::with_capacity(n_points);
    let (mut x, mut y, mut weight): (f64, f64, f64);
    for record_num in 0..vector_points.num_records {
        let record = vector_points.get_record(record_num);
        for i in 0..record.num_points as usize {
            x = record.points[i].x;
            y = record.points[i].y;
            if field_name.is_some() {
                weight = match vector_points.attributes.get_value(record_num, &field_name.as_ref().unwrap()) {
                    FieldData::Int(val) => {
                        val as f64
                    }
                    FieldData::Real(val) => {
                        val
                    }
                    _ => {
                        1.0
                    }
                };
            } else {
                weight = 1.0
            }
            points.push( TreeItem { point: [x, y], weight: weight } );
        }
        if configurations.verbose_mode {
            progress = (100.0_f64 * (record_num + 1) as f64 / vector_points.num_records as f64) as usize;
            if progress != old_progress {
                println!(
                    "Progress: {progress}%"
                );
                old_progress = progress;
            }
        }
    }

    // build the tree
    if configurations.verbose_mode {
        println!("Building kd-tree...");
    }
    let kdtree: KdTree<TreeItem> = KdTree::build_by_ordered_float(points);

    // calculate the point features
    let kdtree = Arc::new(kdtree);


    // Create the output raster. The process of doing this will
    // depend on whether a cell size or a base raster were specified.
    // If both are specified, the base raster takes priority.

    if base_file.trim().is_empty() && cell_size <= 0f64 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Error: One of the two parameters --cell_size or --base must be specified.",
        ));
    }

    let mut output = if !base_file.trim().is_empty() || cell_size == 0f64 {
        if !base_file.contains(&sep) && !base_file.contains("/") {
            base_file = format!("{}{}", working_directory, base_file);
        }
        let base = Raster::new(&base_file, "r")?;
        Raster::initialize_using_file(&output_file, &base)
    } else {
        // base the output raster on the cell_size and the
        // extent of the input vector.
        let west: f64 = vector_points.header.x_min;
        let north: f64 = vector_points.header.y_max;
        let rows: isize = (((north - vector_points.header.y_min) / cell_size).ceil()) as isize;
        let columns: isize = (((vector_points.header.x_max - west) / cell_size).ceil()) as isize;
        let south: f64 = north - rows as f64 * cell_size;
        let east = west + columns as f64 * cell_size;

        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = cell_size;
        configs.resolution_y = cell_size;
        configs.nodata = -32768.0;
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        // configs.epsg_code = vector_points.projection.clone();
        configs.projection = vector_points.projection.clone();

        Raster::initialize_using_config(&output_file, &configs)
    };

    output.configs.data_type = DataType::F32;
    let nodata = output.configs.nodata;
    let rows = output.configs.rows as isize;
    let columns = output.configs.columns as isize;
    let west = output.configs.west;
    let north = output.configs.north;
    let resolution_x = output.configs.resolution_x;
    let resolution_y = output.configs.resolution_y;


    // Convert the string kernel_function to the numeric kernel 
    let kernel = if kernel_function.contains("uniform") || kernel_function.contains("rect") {
        1
    } else if kernel_function.contains("triang") {
        2
    } else if kernel_function.contains("epan") || kernel_function.contains("parabolic") {
        3
    } else if kernel_function.contains("quartic") || kernel_function.contains("biweight") {
        4
    } else if kernel_function.contains("triweight") || kernel_function.contains("tri-weight") {
        5
    } else if kernel_function.contains("tricube") || kernel_function.contains("tri-cube") {
        6
    } else if kernel_function.contains("gaussian") {
        7
    } else if kernel_function.contains("cosine") {
        8
    } else if kernel_function.contains("logistic") {
        9
    } else if kernel_function.contains("sigmoid") {
        10
    } else { // if kernel_function.contains("silverman") {
        11
    };

    // This is in case an error is thrown from inside a thread
    // take_hook() returns the default hook in case when a custom one is not set
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(1);
    }));

    let mut num_procs = num_cpus::get() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let kdtree = kdtree.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let mut x: f64;
            let mut y: f64;
            let mut d: f64;
            let mut weight: f64;
            let mut density_func_value: f64;
            for row in (0..rows).filter(|r| r % num_procs == tid) {
                let mut data = vec![nodata; columns as usize];
                for col in 0..columns {
                    x = west + resolution_x / 2f64 + col as f64 * resolution_x;
                    y = north - resolution_y / 2f64 - row as f64 * resolution_y;

                    let found = kdtree.within_radius(&[x, y], bandwidth);
                    if found.len() > 0 {
                        density_func_value = 0.0;
                    
                        for i in 0..found.len() {
                            d = ((found[i].point[0] - x).powi(2) + (found[i].point[1] - y).powi(2)).sqrt() / bandwidth;
                            weight = found[i].weight;
                            density_func_value += match kernel {
                                1 => {
                                    // uniform kernel function
                                    weight * 0.5
                                },
                                2 => {
                                    // triangular kernel function
                                    weight * (1.0 - d.abs())
                                },
                                3 => {
                                    // Epanechnikov (parabolic) kernel function
                                    weight * (0.75 * (1.0 - d*d))
                                },
                                4 => {
                                    // quartic kernel function
                                    weight * (0.9375 * (1.0 - d*d).powi(2))
                                },
                                5 => {
                                    // triweight kernel function
                                    weight * (1.09375 * (1.0 - d*d).powi(3))
                                },
                                6 => {
                                    // tricube kernel function
                                    weight * (0.864197531 * (1.0 - d.abs().powi(3)).powi(3))
                                },
                                7 => {
                                    // Gaussian kernel function
                                    weight * (0.398942280401433 * (-0.5 * d*d).exp())
                                },
                                8 => {
                                    // cosine kernel function
                                    weight * (0.785398163397448 * (1.570796326794900 * d).cos())
                                },
                                9 => {
                                    // logistic kernel function
                                    weight * (1.0 / (d.exp() + 2.0 + -d.exp()))
                                },
                                10 => {
                                    // sigmoidal kernel function
                                    weight * (0.636619772367581 * 1.0 / (d.exp() + (-d).exp()))
                                },
                                _ => {
                                    // Silverman kernel function
                                    weight * (0.5 * (-(d.abs() / 1.4142135623731)).exp() * (d.abs() / 1.4142135623731 + 0.785398163397448).sin())
                                }
                            };
                            
                        }

                        data[col as usize] = density_func_value;
                    }
                }

                tx.send((row, data)).unwrap();
            }
        });
    }

    for row in 0..rows {
        let (r, data) = rx.recv().expect("Error receiving data from thread.");
        output.set_row_data(r, data);
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
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


struct TreeItem {
    point: [f64; 2],
    weight: f64,
}

impl KdPoint for TreeItem {
    type Scalar = f64;
    type Dim = typenum::U2; // 2 dimensional tree.
    fn at(&self, k: usize) -> f64 { self.point[k] }
}