/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: 22/06/2020
Last Modified: 22/06/2020
License: MIT
*/

use whitebox_raster::*;
use nalgebra::{Matrix5, RowVector5, Vector5};
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use whitebox_common::utils::get_formatted_elapsed_time;

/// This tool is an implementation of the constrained quadratic regression algorithm
/// using a flexible window size described in Wood (1996). A quadratic surface is fit
/// to local areas of input DEM (`--dem`), defined by a filter size
/// (`--filter`) using least squares regression. Note that the model is constrained such
/// that it must pass through the cell at the center of the filter. This is accomplished
/// by representing all elevations relative to the center cell, and by making the equation
/// constant 0.
///
/// Surface derivatives are calculated from the coefficients of the local quadratic
/// surface once they are known. These include: Slope, Aspect, Profile convexity, Plan convexity,
/// Longitudinal curvature, Cross-sectional curvature, and Minimum profile convexity,
/// all as defined in Wood (1996). The goodness-of-fit (r-squared) of each local quadratic
/// model is also returned.
///
/// Due to the fact that large filter sizes require long processing times, and that
/// fitting the surface is the most time consuming part of the algorithm, all LSPs are
/// output every time this tool is run. The content of each output is described by the suffixes
/// of the output file names.
///
/// # Reference
/// Wood, J. (1996). The Geomorphological Characterisation of Digital Elevation Models. University
/// of Leicester.
///
/// # See Also
/// `Aspect`, `Slope`, `PlanCurvature`, `ProfileCurvature`
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

    let exe_name = &format!("local_quadratic_regression{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    local_quadratic_regression Help

    This tool is an implementation of the constrained quadratic regression algorithm
    using a flexible window size described in Wood (1996)

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    -d, --dem      Name of the input DEM raster file.
    -o, --output   Name of the output raster file.
    --filter       Edge length of the filter kernel.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

    Example Usage:
    >> .*EXE_NAME run --dem=DEM.tif --output=out_ras.tif --filter=15
    
    "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "local_quadratic_regression v{} by Dr. John B. Lindsay (c) 2021.",
        VERSION.unwrap_or("Unknown version")
    );
}

fn get_tool_name() -> String {
    String::from("LocalQuadraticRegression") // This should be camel case and is a reference to the tool name.
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

    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut filter_size = 3usize;

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
        if flag_val == "-d" || flag_val == "-dem" {
            if keyval {
                input_file = vec[1].to_string();
            } else {
                input_file = args[i + 1].to_string();
            }
        } else if flag_val == "-o" || flag_val == "-output" {
            if keyval {
                output_file = vec[1].to_string();
            } else {
                output_file = args[i + 1].to_string();
            }
        }  else if flag_val == "-filter" {
            if keyval {
                filter_size = vec[1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val))
                    as usize;
            } else {
                filter_size = args[i + 1]
                    .to_string()
                    .parse::<f32>()
                    .expect(&format!("Error parsing {}", flag_val))
                    as usize;
            }
        }
    }

    if filter_size < 3 { filter_size = 3; }
    // The filter dimensions must be odd numbers such that there is a middle pixel
    if (filter_size as f64 / 2f64).floor() == (filter_size as f64 / 2f64) {
        filter_size += 1;
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

    let sep: String = path::MAIN_SEPARATOR.to_string();

    let mut progress: usize;
    let mut old_progress: usize = 1;

    if !input_file.contains(&sep) && !input_file.contains("/") {
        input_file = format!("{}{}", working_directory, input_file);
    }
    if !output_file.contains(&sep) && !output_file.contains("/") {
        output_file = format!("{}{}", working_directory, output_file);
    }

    if configurations.verbose_mode {
        println!("Reading data...")
    };

    let input = Arc::new(Raster::new(&input_file, "r")?);

    let rows = input.configs.rows as isize;
    let columns = input.configs.columns as isize;
    let nodata = input.configs.nodata;
    let resolution = input.configs.resolution_x; // assume square

    let path_parts: Vec<&str> = output_file.rsplitn(2, ".").collect();
    let mut outputs: [Raster; 8] = [
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "Slp", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "Asp", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "ProC", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "PlaC", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "LonC", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "CrsC", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "PrCM", &path_parts[0]), &input),
        Raster::initialize_using_file(&format!("{}_{}.{}", &path_parts[1], "GoF", &path_parts[0]), &input)
    ];

    let start = Instant::now();

    // no weights simplifies matrices

    let offset = (filter_size - 1) / 2;
    let num_cells = filter_size * filter_size;

    // determine filter offsets
    let mut dx = vec![0isize; num_cells];
    let mut dy = vec![0isize; num_cells];
    let mut idx = 0usize;
    for i in 0..filter_size {
        for j in 0..filter_size {
            dx[idx] = (j - offset) as isize;
            dy[idx] = (i - offset) as isize;
            idx += 1;
        }
    }

    let num_procs = num_cpus::get() as isize;
    let (tx, rx) = mpsc::channel();
    for tid in 0..num_procs {
        let input = input.clone();
        let dx = dx.clone();
        let dy = dy.clone();
        let tx = tx.clone();
        // let a_decomp = a_decomp.clone();
        thread::spawn(move || {

            let mut z: f64;
            let mut zi: f64;

            for row in (0..rows).filter(|r| r % num_procs == tid) {

                let mut slopes = vec![nodata; columns as usize];
                let mut aspects = vec![nodata; columns as usize];
                let mut prof_cs = vec![nodata; columns as usize];
                let mut plan_cs = vec![nodata; columns as usize];
                let mut long_cs = vec![nodata; columns as usize];
                let mut cross_cs = vec![nodata; columns as usize];
                let mut procmin_cs = vec![nodata; columns as usize];
                let mut gofs = vec![nodata; columns as usize];

                for col in 0..columns {

                    z = input[(row, col)];

                    if z != nodata {

                        let (mut zx2, mut zy2, mut zxy, mut zx, mut zy, mut _zw) = (0f64,0f64,0f64,0f64,0f64,0f64);
                        let (mut x2, mut x2y2, mut x4) = (0f64, 0f64, 0f64);
                        let mut num_valid = 0usize;
                        let (mut z_pred, mut z_act): (f64, f64);
                        let (mut sum_x, mut sum_y, mut sum_xy, mut sum_xx, mut sum_yy) = (0f64, 0f64, 0f64, 0f64, 0f64);
                        let (r, n): (f64, f64);

                        let mut xs = vec![];
                        let mut ys = vec![];
                        let mut zs = vec![];

                        for c in 0..num_cells {
                            zi = input[((row + dy[c] as isize), (col + dx[c] as isize))];
                            if zi != nodata {
                                xs.push(dx[c] as f64 * resolution);
                                ys.push(dy[c] as f64 * resolution);
                                zs.push(zi - z); // elevation relative to center
                                num_valid += 1;
                            }
                        }

                        if num_valid >= 8 {//6 { // need at least six samples
                            // compute sums
                            for i in 0..num_valid {
                                zx2 += zs[i] * xs[i].powi(2);
                                zy2 += zs[i] * ys[i].powi(2);
                                zxy += zs[i] * xs[i] * ys[i];
                                zx += zs[i] * xs[i];
                                zy += zs[i] * ys[i];
                                _zw += zs[i];

                                x2 += xs[i].powi(2);
                                x2y2 += xs[i].powi(2) * ys[i].powi(2);
                                x4 += xs[i].powi(4);
                            }

                            let a = Matrix5::from_rows(&[
                                RowVector5::new(x4, x2y2, 0f64, 0f64, 0f64),
                                RowVector5::new(x2y2, x4, 0f64, 0f64, 0f64),
                                RowVector5::new(0f64,0f64,x2y2, 0f64, 0f64),
                                RowVector5::new(0f64, 0f64, 0f64, x2, 0f64),
                                RowVector5::new(0f64, 0f64, 0f64, 0f64, x2),
                            ]);

                            let b = Vector5::new(zx2, zy2, zxy, zx, zy);

                            let fitted_surface = Quadratic2d::from_normals_origin(a, b);

                            for i in 0..num_valid {
                                z_act = zs[i];
                                sum_x += z_act;
                                sum_xx += z_act * z_act;

                                z_pred = fitted_surface.solve(xs[i], ys[i]);
                                sum_y += z_pred;
                                sum_yy += z_pred * z_pred;

                                sum_xy += z_act * z_pred;
                            }

                            n = num_valid as f64;
                            let noom = n * sum_xy - (sum_x * sum_y);
                            let den = (n * sum_xx - (sum_x * sum_x)).sqrt() * ((n * sum_yy - (sum_y * sum_y)).sqrt());
                            if noom == 0f64 || den == 0f64 {
                                r = 0f64;
                            } else {
                                r = noom / den;
                            }

                            slopes[col as usize] = fitted_surface.slope();
                            aspects[col as usize] = fitted_surface.aspect();
                            prof_cs[col as usize] = fitted_surface.profile_convexity();
                            plan_cs[col as usize] = fitted_surface.plan_convexity();
                            long_cs[col as usize] = fitted_surface.longitudinal_curvature();
                            cross_cs[col as usize] = fitted_surface.cross_sectional_curvature();
                            procmin_cs[col as usize] = fitted_surface.min_prof_convexity();
                            gofs[col as usize] = r * r;
                        }
                    }
                }

                tx.send(
                    (row,
                    slopes,
                    aspects,
                    prof_cs,
                    plan_cs,
                    long_cs,
                    cross_cs,
                    procmin_cs,
                    gofs)
                ).unwrap();

            }
        });
    }

    for row in 0..rows {
        let data = rx.recv().expect("Error receiving data from thread.");
        outputs[0].set_row_data(data.0, data.1);
        outputs[1].set_row_data(data.0, data.2);
        outputs[2].set_row_data(data.0, data.3);
        outputs[3].set_row_data(data.0, data.4);
        outputs[4].set_row_data(data.0, data.5);
        outputs[5].set_row_data(data.0, data.6);
        outputs[6].set_row_data(data.0, data.7);
        outputs[7].set_row_data(data.0, data.8);

        if configurations.verbose_mode {
            progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress {
                println!("Performing analysis: {}%", progress);
                old_progress = progress;
            }
        }
    }

    let elapsed_time = get_formatted_elapsed_time(start);

    if configurations.verbose_mode {
        println!("Saving data...")
    };

    for o in 0..outputs.len() {
        outputs[o].configs.palette = "grey.plt".to_string();
        outputs[o].add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            tool_name
        ));
        outputs[o].add_metadata_entry(format!("Input file: {}", input_file));
        outputs[o].add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        let _ = match outputs[o].write() {
            Ok(_) => {
                if configurations.verbose_mode {
                    println!("Output file {:?} written", o+1);
                }
            }
            Err(e) => return Err(e),
        };
    }

    if configurations.verbose_mode {
        println!(
            "{}",
            &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
        );
    }
    Ok(())
}

// Equation of a 2d quadratic model:
// z(x,y) = ax^2 + by^2 + cxy + dx + ey + f
#[derive(Default, Clone, Copy)]
struct Quadratic2d {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64
}

impl Quadratic2d {
    fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Quadratic2d {
        Quadratic2d {
            a: a,
            b: b,
            c: c,
            d: d,
            e: e,
            f: f
        }
    }

    // solves a system of normal equations ax = b
    // fn from_normal_equations(a: Matrix6<f64>, b: Vector6<f64>) -> Quadratic2d {
    //     let decomp = a.lu();
    //     if decomp.is_invertible() {
    //         let x = decomp.solve(&b).expect("Linear resolution failed.");
    //         Quadratic2d::new(
    //             *x.get(0).unwrap(), // a
    //             *x.get(1).unwrap(), // b
    //             *x.get(2).unwrap(), // c
    //             *x.get(3).unwrap(), // d
    //             *x.get(4).unwrap(), // e
    //             *x.get(5).unwrap()  // f
    //         )
    //     } else {
    //         Quadratic2d::new(0f64,0f64,0f64,0f64,0f64,0f64)
    //     }
    // }
    fn from_normals_origin(a: Matrix5<f64>, b: Vector5<f64>) -> Quadratic2d {
        let decomp = a.lu();
        if decomp.is_invertible() {
            let x = decomp.solve(&b).expect("Linear resolution failed.");
            Quadratic2d::new(
                *x.get(0).unwrap(), // a
                *x.get(1).unwrap(), // b
                *x.get(2).unwrap(), // c
                *x.get(3).unwrap(), // d
                *x.get(4).unwrap(), // e
                0f64, //f
            )
        } else {
            Quadratic2d::new(0f64,0f64,0f64,0f64,0f64,0f64)
        }
    }
    // fn from_decomposed_normals(
    //     decomp: LU<f64, nalgebra::base::dimension::U6, nalgebra::base::dimension::U6>,
    //     b: Vector6<f64>
    // ) -> Quadratic2d {
    //     if decomp.is_invertible() {
    //         let x = decomp.solve(&b).expect("Linear resolution fialed.");
    //         Quadratic2d::new(
    //             *x.get(0).unwrap(), // a
    //             *x.get(1).unwrap(), // b
    //             *x.get(2).unwrap(), // c
    //             *x.get(3).unwrap(), // d
    //             *x.get(4).unwrap(), // e
    //             *x.get(5).unwrap()  // f
    //         )
    //     } else {
    //         Quadratic2d::new(0f64,0f64,0f64,0f64,0f64,0f64)
    //     }
    // }

    fn slope(&self) -> f64 {
        // (self.a*self.a + self.b*self.b).sqrt().atan().to_degrees()
        (self.d*self.d + self.e*self.e).sqrt().atan()
    }

    fn aspect(&self) -> f64 {
        if self.e == 0f64 || self.d == 0f64 {
            0f64
        } else {
            (self.e / self.d).atan()
        }
    }

    fn profile_convexity(&self) -> f64 {
        let nu = -200f64 * ((self.a*self.d*self.d) + (self.b*self.e*self.e) + (self.c*self.d*self.e));
        let de  = ((self.e*self.e) + (self.d*self.d)) * (1f64 + (self.d*self.d) + (self.e*self.e)).powf(1.5);
        if nu == 0f64 || de == 0f64 {
            0f64
        } else {
            nu / de
        }
    }
    fn plan_convexity(&self) -> f64 {
        let nu = 200f64 * ((self.b*self.d*self.d) + (self.a*self.e*self.e) - (self.c*self.d*self.e));
        let de  = ((self.e*self.e) + (self.d*self.d)).powf(1.5);
        if nu == 0f64 || de == 0f64 {
            0f64
        } else {
            nu / de
        }
    }
    fn longitudinal_curvature(&self) -> f64 {
        let nu = (self.a*self.d*self.d) + (self.b*self.e*self.e) + (self.c*self.d*self.e);
        let de = (self.d*self.d) + (self.e*self.e);
        if nu == 0f64 || de == 0f64 {
            0f64
        } else{
            -2f64*(nu / de)
        }
    }
    fn cross_sectional_curvature(&self) -> f64 {
        let nu = (self.b*self.d*self.d) + (self.a*self.e*self.e) - (self.c*self.d*self.e);
        let de = (self.d*self.d) + (self.e*self.e);
        if nu == 0f64 || de == 0f64 {
            0f64
        } else{
            -2f64*(nu / de)
        }
    }
    // fn max_prof_convexity(&self) -> f64 {
    //     (self.a * -1f64) - self.b + ((self.a - self.b).powi(2) + (self.c * self.c)).sqrt()
    // }
    fn min_prof_convexity(&self) -> f64 {
        (self.a * -1f64) - self.b - ((self.a - self.b).powi(2) + (self.c * self.c)).sqrt()
    }
    fn solve(&self, x: f64, y: f64) -> f64 {
        // z(x,y) = ax^2 + by^2 + cxy + dx + ey + f
        return (self.a*(x*x)) + (self.b*(y*y)) + (self.c*(x*y)) + (self.d*x) + (self.e*y) + self.f
    }
}
