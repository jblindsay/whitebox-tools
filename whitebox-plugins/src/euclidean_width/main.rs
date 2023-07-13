use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use whitebox_common::utils::{ get_formatted_elapsed_time, wrapped_print };
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::time::Instant;
use std::sync::mpsc;
use std::thread;
use std::collections::{BinaryHeap};
use ordered_float::OrderedFloat;
use num_cpus;

/// This tool will estimate the width of the free space between obstacles in the input raster.
/// For each pixel the width is defined as the diameter of the largest circle that covers the
/// pixel and is inscribed in a set of obstacles. Obstacles are all non-zero, non-NoData grid cells.
/// Width in the output image is measured in projection units of the input raster.
///
/// ![](../../doc_img/EuclideanWidth.png)
///
/// # Algorithm Description
/// The algorithm is based on the width estimation procedure developed by Samsonov et al. (2019)
/// to find the appropriate places for drawing the supplementary contour lines. First, the Euclidean
/// distance raster is calculated using the obstacles as target pixels (see more details in the docs
/// of the `Euclidean distance` tool). Next, an output raster with the same geometry is created and
/// initialized with zero values. We now propagate the doubled value of each pixel of the distance
/// raster to the output pixels that are covered by the circle neighbourhood of the corresponding
/// radius. The resulting value is determined by the following rule: If a pixel is empty or has a
/// value smaller than the doubled value of the distance raster, then its value is replaced with
/// the doubled value of the distance raster; otherwise, it remains unchanged. To speed up the
/// computations all pixels in distance raster are organized using the binary heap, so the pixel
/// with the largest distance pops first and the width is calculated only once for each output pixel.
///
/// Since the procedure of reconstructing the pixel neighborhoods is computationally intensive, you
/// can set the upper limit for precise computation of the width. This is done by the optional
/// `max_width` parameter. If this parameter is set, the width will be estimated much faster (and
/// less accurately) in areas of larger widths. This can be very important if you have the clustered
/// distribution of obstacles (e.g. buildings clustered in settlements) and interested only in the
/// width inside clusters. In this case setting the appropriate `max_width` may speed up the
/// computation of width between clusters by an order of magnitude.
///
/// All NoData value grid pixels in the input image will contain NoData values in the
/// output image. As such, NoData is not a suitable background value for non-obstacle pixel.
/// Background areas should be designated with zero values.
///
/// # Reference
/// Samsonov, T., Koshel, S., Walther, D., Jenny, B., 2019. Automated placement of supplementary
/// contour lines. International Journal of Geographical Information Science 33, 2072–2093.
/// https://doi.org/10.1080/13658816.2019.1610965
///
/// # See Also
/// `EuclideanDistance`


fn main() {

    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {},
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

    let exe_name = &format!("euclidean_width{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    euclidean_width Help

    This tool calculates width of the free space between obstacles. The width is a diameter
    of the largest circle which covers the pixel and is inscribed in a set of obstacles.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -i, --input   Name of the input raster file.
    -o, --output  Name of the output raster file.
    --max_width   Maximum width for high-precision calculation (optional).

    Input/output file names can be fully qualified, or can rely on the
    working directory contained in the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME -r=EuclideanWidth -i=input.tif -o=width.tif --max_width=1000.0

    "#
        .replace("*", &sep)
        .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "euclidean_width v{} by Dr. Timofey E. Samsonov and Dr. John B. Lindsay (c) 2023.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("EuclideanWidth")
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
    let max_procs = configurations.max_procs as isize;

    let mut input_file: String = String::new();
    let mut output_file: String = String::new();
    let mut max_width = f64::INFINITY;

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
        } else if flag_val == "-max_width" {
            max_width = if keyval {
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

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if max_width < 0.0 {
        if configurations.verbose_mode {
            wrapped_print("Warning: Maximum width should be non-negative", 50);
        }
        max_width = 0.0;
    }

    if configurations.verbose_mode {
        println!("Reading data...")
    };

    let input = Raster::new(&input_file, "r")?;

    if configurations.verbose_mode {
        println!("Processing data...")
    };

    let configs = input.configs.clone();

    let nodata = configs.nodata;
    let rows = configs.rows as isize;
    let columns = configs.columns as isize;

    let start = Instant::now();

    //////////////////////////////////
    // Calculate euclidean distance //
    //////////////////////////////////

    let mut rx: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
    let mut ry: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

    let mut distance: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;

    let mut output = Raster::initialize_using_file(&output_file, &input);

    let mut h: f64;
    let mut which_cell: usize;
    let inf_val = f64::INFINITY;
    let dx = [-1, -1, 0, 1, 1, 1, 0, -1];
    let dy = [0, -1, -1, -1, 0, 1, 1, 1];
    let gx = [1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0];
    let gy = [0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
    let (mut x, mut y): (isize, isize);
    let (mut z, mut z2, mut z_min): (f64, f64, f64);

    for row in 0..rows {
        for col in 0..columns {
            z = input.get_value(row, col);
            if z != 0.0 && z != nodata {
                distance.set_value(row, col, 0.0);
            } else {
                distance.set_value(row, col, inf_val);
            }
            output.set_value(row, col, 0.0);
        }
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Initializing Rasters: {}%", progress);
                old_progress = progress;
            }
        }
    }

    for row in 0..rows {
        for col in 0..columns {
            z = distance.get_value(row, col);
            if z != 0.0 {
                z_min = inf_val;
                which_cell = 0;
                for i in 0..4 {
                    x = col + dx[i];
                    y = row + dy[i];
                    z2 = distance.get_value(y, x);
                    if z2 != nodata {
                        h = match i {
                            0 => 2.0 * rx.get_value(y, x) + 1.0,
                            1 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                            2 => 2.0 * ry.get_value(y, x) + 1.0,
                            _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 3
                        };
                        z2 += h;
                        if z2 < z_min {
                            z_min = z2;
                            which_cell = i;
                        }
                    }
                }
                if z_min < z {
                    distance.set_value(row, col, z_min);
                    x = col + dx[which_cell];
                    y = row + dy[which_cell];
                    rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                    ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                }
            }
        }
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (1 of 4): {}%", progress);
                old_progress = progress;
            }
        }
    }

    for row in (0..rows).rev() {
        for col in (0..columns).rev() {
            z = distance.get_value(row, col);
            if z != 0.0 {
                z_min = inf_val;
                which_cell = 0;
                for i in 4..8 {
                    x = col + dx[i];
                    y = row + dy[i];
                    z2 = distance.get_value(y, x);
                    if z2 != nodata {
                        h = match i {
                            5 => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0),
                            4 => 2.0 * rx.get_value(y, x) + 1.0,
                            6 => 2.0 * ry.get_value(y, x) + 1.0,
                            _ => 2.0 * (rx.get_value(y, x) + ry.get_value(y, x) + 1.0), // 7
                        };
                        z2 += h;
                        if z2 < z_min {
                            z_min = z2;
                            which_cell = i;
                        }
                    }
                }
                if z_min < z {
                    distance.set_value(row, col, z_min);
                    x = col + dx[which_cell];
                    y = row + dy[which_cell];
                    rx.set_value(row, col, rx.get_value(y, x) + gx[which_cell]);
                    ry.set_value(row, col, ry.get_value(y, x) + gy[which_cell]);
                }
            }
        }
        if configurations.verbose_mode {
            progress = (100.0_f64 * (rows - row) as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (2 of 4): {}%", progress);
                old_progress = progress;
            }
        }
    }

    let cell_size = (configs.resolution_x + configs.resolution_y) / 2.0;
    for row in 0..rows {
        for col in 0..columns {
            if input.get_value(row, col) != nodata {
                distance.set_value(row, col, distance.get_value(row, col).sqrt() * cell_size);
            } else {
                distance.set_value(row, col, nodata);
            }
        }
        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Progress (3 of 4): {}%", progress);
                old_progress = progress;
            }
        }
    }

    ///////////////////////////////
    // Calculate euclidean width //
    ///////////////////////////////

    let mut num_procs = num_cpus::get_physical() as isize;
    if max_procs > 0 && max_procs < num_procs {
        num_procs = max_procs;
    }

    let mut tasks = Vec::new();
    let (tx, rx) = mpsc::channel(); // used only for progress bar

    for proc in 0..num_procs {

        let mut output_ref = output.clone();
        let input_ref = distance.clone();

        let tx = tx.clone();

        tasks.push(thread::spawn(move || {
            let mut radius: f64;
            let (mut w, mut ik, mut jl): (isize, isize, isize);

            let mut queue = BinaryHeap::new();

            for i in (0..rows).filter(|r| r % num_procs == proc) {
                for j in 0..columns {
                    if input_ref.get_value(i, j) != nodata && input_ref.get_value(i, j) > 0.0 {
                        queue.push((OrderedFloat(input_ref.get_value(i, j)), i, j));
                    } else {
                        tx.send(1.0).unwrap();
                    }
                }
            }

            while queue.len() > 0 {
                let (ord_radius, i, j) = queue.pop().unwrap();

                tx.send(1.0).unwrap();

                if output_ref.get_value(i, j) >= max_width {
                    continue;
                }

                radius = f64::from(ord_radius);

                w = (radius / cell_size).floor() as isize;

                for k in (-w+1)..w {
                    for l in (-w+1)..w {
                        if k*k + l*l > w*w {
                            continue;
                        }
                        ik = i as isize + k;
                        jl = j as isize + l;

                        if ik < 0 || ik >= rows || jl < 0 || jl >= columns {
                            continue;
                        }

                        if output_ref.get_value(ik, jl) < 2.0 * radius && input_ref.get_value(ik, jl) > 0.0 {
                            output_ref.set_value(ik, jl, 2.0 * radius);
                        }
                    }
                }
            }

            return output_ref;
        }))
    }

    let total = (rows * columns) as f64;
    let mut counter = 0.0;
    for _ in 0..rows {
        for _ in 0..columns {
            counter += rx.recv().unwrap();
            if configurations.verbose_mode {
                progress = (100.0_f64 * counter / total) as usize;
                if progress != old_progress {
                    println!("Progress (4 of 4): {}%", progress);
                    old_progress = progress;
                }
            }
        }
    }

    for task in tasks {
        let add = task.join().unwrap();
        for i in 0..rows {
            for j in 0..columns {
                if add.get_value(i, j) > output.get_value(i, j) {
                    output.set_value(i, j, add.get_value(i, j));
                }
            }
        }
    }

    for i in 0..rows {
        for j in 0..columns {
            if input.get_value(i, j) == nodata {
                output.set_value(i, j, nodata);
            }
        }
    }

    // WRITING OUTPUT

    let elapsed_time = get_formatted_elapsed_time(start);
    output.configs.palette = "grey.plt".to_string();
    output.add_metadata_entry(format!(
        "Created by whitebox_tools\' {} tool",
        tool_name
    ));
    output.add_metadata_entry(format!("Input file: {}", input_file));
    output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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

    if configurations.verbose_mode {
        println!(
            "{}",
            &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
        );
    }

    Ok(())

}