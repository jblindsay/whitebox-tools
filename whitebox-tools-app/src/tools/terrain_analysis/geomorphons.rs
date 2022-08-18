/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Daniel Newman
Created: 21/09/2018
Last Modified: 07/06/2022
Last Modified By: Dan Newman
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::path;
use std::f64;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::io::{Error, ErrorKind};
use nalgebra::{Vector3, Matrix3};
//use std::collections::HashSet;

/// This tool can be used to perform a geomorphons landform classification based on an input digital elevation
/// model (`--dem`). The geomorphons concept is based on line-of-sight analysis for the eight
/// topographic profiles in the cardinal directions surrounding each grid cell in the input DEM. The relative
/// sizes of the zenith angle of a profile's maximum elevation angle (i.e. horizon angle) and the nadir angle of
/// a profile's minimum elevation angle are then used to generate a ternary (base-3) digit: 0 when the nadir
/// angle is less than the zenith angle, 1 when the two angles differ by less than a user-defined flatness
/// threshold (`--threshold`), and 2 when the nadir angle is greater than the zenith angle. A ternary number
/// is then derived from the digits assigned to each of the eight profiles, with digits sequenced counter-clockwise
/// from east. This ternary number forms the  geomorphons code assigned to the grid cell. There are
/// 3<sup>8</sup> = 6561 possible codes, although many of these codes are equivalent geomorphons through
/// rotations and reflections. Some of the remaining geomorphons also rarely if ever occur in natural
/// topography. Jasiewicz et al. (2013) identified 10 common landform types by reclassifying related
/// geomorphons codes. The user may choose to output these common forms (`--forms`) rather than the
/// the raw ternary code. These landforms include:
///
/// | Value | Landform Type |
/// |-:|:-|
/// | 1  | Flat |
/// | 2  | Peak (summit) |
/// | 3  | Ridge |
/// | 4  | Shoulder |
/// | 5  | Spur (convex) |
/// | 6  | Slope |
/// | 7  | Hollow (concave) |
/// | 8  | Footslope |
/// | 9  | Valley |
/// | 10 | Pit (depression) |
///
/// One of the main advantages of the geomrophons method is that, being based on minimum/maximum elevation
/// angles, the scale used to estimate the landform type at a site adapts to the surrounding terrain.
/// In principle, choosing a large value of search distance (`--search`) should result in
/// identification of a landform element regardless of its scale.
///
/// An experimental feature has been added to correct for global inclination. Global inclination
/// biases the flatness threshold angle becasue it is measured relative to the z-axis, especially
/// in locally flat areas. Including the `--residuals` flag "flattens" the input by converting
/// elevation to residuals of a 2-d linear model.
///
/// ![](../../doc_img/Geomorphons.jpg)
///
/// # Reference
/// Jasiewicz, J., and Stepinski, T. F. (2013). Geomorphons — a pattern recognition approach to classification
/// and mapping of landforms. Geomorphology, 182, 147-156.
///
/// # See Also
/// `PennockLandformClass`

pub struct Geomorphons {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Geomorphons {
    pub fn new() -> Geomorphons {
        let name = "Geomorphons".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Computes geomorphon patterns.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM file.".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output file.".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Search distance (cells).".to_owned(),
            flags: vec!["--search".to_owned()],
            description: "Look up distance (in cells).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("50".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Flatness threshold (degrees).".to_owned(),
            flags: vec!["--threshold".to_owned()],
            description: "Flatness threshold for the classification function (in degrees).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Flatness distance (cells).".to_owned(),
            flags: vec!["--fdist".to_owned()],
            description: "Distance (in cells) to begin reducing the flatness threshold to avoid problems with pseudo-flat lines-of-sight.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("0".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Skip distance (cells).".to_owned(),
            flags: vec!["--skip".to_owned()],
            description: "Distance (in cells) to begin calculating lines-of-sight.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("0".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output forms".to_owned(),
            flags: vec!["-f".to_owned(), "--forms".to_owned()],
            description: "Classify geomorphons into 10 common land morphologies, else output ternary pattern.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Analyze residuals".to_owned(),
            flags: vec!["--residuals".to_owned()],
            description: "Convert elevation to residuals of a linear model.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=output.tif --search=50 --threshold=0.0 --tdist=0.0 --forms", short_exe, name).replace("*", &sep);

        Geomorphons {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage
        }
    }
}

impl WhiteboxTool for Geomorphons {
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
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut search_radius_cells: usize = 1;
        let mut flat_thresh: f64 = 1f64;
        let mut flat_dist_cells: usize = 0;
        let mut skip_dist_cells: usize = 0;
        let mut forms: bool = false;
        let mut use_residuals = false;

        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                "Tool run with no parameters."));
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
            if flag_val == "-i" || flag_val == "-dem" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-search" {
                search_radius_cells = if keyval {
                    vec[1].to_string().parse::<isize>()
                        .expect(&format!("Error parsing {}", flag_val))
                        .abs() as usize
                } else {
                    args[i+1].to_string().parse::<isize>().unwrap().abs() as usize
                };
            } else if flag_val == "-threshold" {
                flat_thresh = if keyval {
                    vec[1].to_string().parse::<f64>().expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i+1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-fdist" {
                flat_dist_cells = if keyval {
                    vec[1].to_string().parse::<isize>()
                        .expect(&format!("Error parsing {}", flag_val))
                        .abs() as usize
                } else {
                    args[i+1].to_string().parse::<isize>().unwrap().abs() as usize
                };
            } else if flag_val == "-skip" {
                skip_dist_cells = if keyval {
                    vec[1].to_string().parse::<isize>()
                        .expect(&format!("Error parsing {}", flag_val))
                        .abs() as usize
                } else {
                    args[i+1].to_string().parse::<isize>().unwrap().abs() as usize
                };
            } else if flag_val == "-f" || flag_val == "-forms" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    forms = true;
                }
            } else if flag_val == "-residuals" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    use_residuals = true;
                }
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

        if verbose { println!("Reading data...") };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let mut output = Raster::initialize_using_file(&output_file, &input);

        let start = Instant::now();

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;
        let grid_res = input.configs.resolution_y as f64;
        let nodatai16 = i16::MIN as f64;
        let half_pi = f64::consts::FRAC_PI_2;
        let flat_thresh = flat_thresh.to_radians();

        // in units cells
        if search_radius_cells < 1 { search_radius_cells = 1; };
        if (flat_dist_cells > 0 && flat_dist_cells <= skip_dist_cells)
                    || flat_dist_cells >= search_radius_cells
        { flat_dist_cells = 0; }

        // in units resolution
        let search_length = search_radius_cells as f64 * grid_res;
        let flat_length = flat_dist_cells as f64 * grid_res;
        let flat_threshold_height = flat_thresh.tan() * flat_length;

        // generate global ternary codes
        if verbose {
            println!("Generating global ternary codes...");
        }
        let max_codes: usize = 6561; // = 3^8 for 8-bit ternary
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut power, mut k): (i32, i32);
                let (mut value, mut data): (i32, i32);
                let (mut code, mut rev_code): (i32, i32);
                let (mut tmp_code, mut tmp_rev_code): (i32, i32);
                let mut pattern: [i32; 8];
                let mut rev_pattern: [i32; 8];
                let mut tmp_pattern: [i32; 8];
                let mut tmp_rev_pat: [i32; 8];
                for val in (0..max_codes as isize).filter(|v| v % num_procs == tid) {
                    pattern = [0; 8];
                    rev_pattern = [0; 8];
                    value = val as i32;
                    // int to ternary pattern
                    for i in 0..8 {
                        pattern[i] = (value % 3) as i32;
                        rev_pattern[7 - i] = (value % 3) as i32;
                        value /= 3i32;
                    }
                    // rotated and mirrored ternary codes
                    code = i32::max_value();
                    rev_code = i32::max_value();
                    tmp_pattern = [0; 8];
                    tmp_rev_pat = [0; 8];
                    for j in 0..8 as i32 {
                        power = 1;
                        tmp_code = 0;
                        tmp_rev_code = 0;
                        // ternary bit shift
                        for i in 0..8 {
                            if (i as i32 - j) < 0i32 {
                                k = j - 8i32;
                            } else { k = j; }
                            tmp_pattern[i] = pattern[(i as i32 - k) as usize];
                            tmp_rev_pat[i] = rev_pattern[(i as i32 - k) as usize];
                            tmp_code += (tmp_pattern[i]) as i32 * power;
                            tmp_rev_code += (tmp_rev_pat[i]) as i32 * power;
                            power *= 3;
                        }
                        // min of mirrored ternary code
                        if tmp_code < code {
                            code = tmp_code;
                        } else { code = code; }
                        if tmp_rev_code < rev_code {
                            rev_code = tmp_rev_code;
                        } else { rev_code = rev_code; }
                    }
                    //min of rotation and mirrored ternary codes
                    if code < rev_code {
                        data = code;
                    } else { data = rev_code; }
                    tx.send((val, data)).unwrap();
                }
            });
        };

        let mut gtc = [u16::MAX; 6561];
        for _ in 0..max_codes {
            let out = rx.recv().expect("Error receiving data from thread.");
            gtc[out.0 as usize] = out.1 as u16;
        }
        //let mut gtchash = HashSet::new();
        //for i in 0..gtc.len() {
        //    gtchash.insert(gtc[i]);
        //}
        //assert_eq!(gtchash.into_iter().collect::<Vec<u16>>().len(), 498);

        // transform input to residuals
        if use_residuals {
            if verbose { println!("Calculating residuals..."); }
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut sum_y = 0f64;
                    let mut sum_xr_y = 0f64;
                    let mut sum_xc_y = 0f64;
                    let mut sum_xr = 0f64;
                    let mut sum_xc = 0f64;
                    let mut sum_xr_xr = 0f64;
                    let mut sum_xc_xc = 0f64;
                    let mut sum_xr_xc = 0f64;
                    let mut n = 0f64;
                    let mut z: f64;
                    let (mut r, mut c): (f64, f64);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        r = row as f64;
                        for col in 0..columns {
                            c = col as f64;
                            z = input.get_value(row, col);
                            if z != nodata  {
                                sum_y += z;
                                sum_xr_y += r * z;
                                sum_xc_y += c * z;
                                sum_xr += r;
                                sum_xc += c;
                                sum_xr_xr += r * r;
                                sum_xc_xc += c * c;
                                sum_xr_xc += r * c;
                                n += 1f64;
                            }
                        }
                    }
                    tx.send((sum_y, sum_xr_y, sum_xc_y, sum_xr, sum_xc,
                                sum_xr_xr, sum_xc_xc, sum_xr_xc, n)).unwrap();
                });
            }


            let mut sum_y = 0f64;
            let mut sum_xr_y = 0f64;
            let mut sum_xc_y = 0f64;
            let mut sum_xr = 0f64;
            let mut sum_xc = 0f64;
            let mut sum_xr_xr = 0f64;
            let mut sum_xc_xc = 0f64;
            let mut sum_xr_xc = 0f64;
            let mut n = 0f64;
            for _ in 0..num_procs {
                let (y, xry, xcy, xr, xc,
                        xrxr, xcxc, xrxc, num) = rx.recv()
                        .expect("Error receiving data from thread.");
                sum_y += y;
                sum_xr_y += xry;
                sum_xc_y += xcy;
                sum_xr += xr;
                sum_xc += xc;
                sum_xr_xr += xrxr;
                sum_xc_xc += xcxc;
                sum_xr_xc += xrxc;
                n += num;
            }
            //          | n         sum_xr      sum_xc    |   | b0  |           | sum_y    |
            // X'X =    | sum_xr    sum_xr_xr   sum_xr_xc | . | b1r | =  X'Y =  | sum_xr_y |
            //          | sum_xc    sum_xr_xc   sum_xc_xc |   | b1c |           | sum_xc_y |

            //let det = (n*sum_xr_xr*sum_xc_xc) + (sum_xr*sum_xr_xc*sum_xc) + (sum_xc*sum_xr*sum_xr_xc)
            //     - (n*sum_xr_xc*sum_xr_xc) - (sum_xr*sum_xr*sum_xc_xc) - (sum_xc*sum_xr_xr*sum_xc)

            let yx = Vector3::new(sum_y, sum_xr_y, sum_xc_y);
            let xtx = Matrix3::new
                                (
                                    n,      sum_xr,     sum_xc,
                                    sum_xr, sum_xr_xr,  sum_xr_xc,
                                    sum_xc, sum_xr_xc,  sum_xc_xc
                                );
            let solution = xtx.lu().solve(&yx)
                            .expect("Linear resolution failed");
            let b0 = *solution.get(0).unwrap();
            let b1r = *solution.get(1).unwrap();
            let b1c = *solution.get(2).unwrap();

            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input = input.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut z: f64;
                    let (mut r, mut c): (f64, f64);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        r = row as f64;
                        let mut data = vec![nodata; columns as usize];
                        for col in 0..columns {
                            c = col as f64;
                            z = input.get_value(row, col);
                            if z != nodata {
                                data[col as usize] = z -
                                    (b0 + (b1r * r) + (b1c * c)); // estimate
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }
            for _ in 0..rows {
                let data = rx.recv().expect("Error receiving data from thread.");
                output.set_row_data(data.0, data.1);
            }
        }
        let input = if use_residuals {
            // use residuals as input for geomorphons
            Arc::new(output.clone())
        } else {
            input
        };

        // main loop
        if verbose { println!("Computing geomorphons..."); }

        let classes: [[u8; 9]; 9] = [ /* 0  1  2  3  4  5  6  7  8 */   // 1  = Flat
                                /* 0*/  [1, 1, 1, 8, 8, 9, 9, 9,10],    // 2  = Peak // Summit
                                /*-1*/  [1, 1, 8, 8, 8, 9, 9, 9, 0],    // 3  = Ridge
                                /*-2*/  [1, 4, 6, 6, 7, 7, 9, 0, 0],    // 4  = Shoulder
                                /*-3*/  [4, 4, 6, 6, 6, 7, 0, 0, 0],    // 5  = Convex // Spur
                                /*-4*/  [4, 4, 5, 6, 6, 0, 0, 0, 0],    // 6  = Slope
                                /*-5*/  [3, 3, 5, 5, 0, 0, 0, 0, 0],    // 7  = Concave // Hollow
                                /*-6*/  [3, 3, 3, 0, 0, 0, 0, 0, 0],    // 8  = Footslope
                                /*-7*/  [3, 3, 0, 0, 0, 0, 0, 0, 0],    // 9  = Valley
                                /*-8*/  [2, 0, 0, 0, 0, 0, 0, 0, 0]     // 10 = Pit // Depression
                                    ];                                  // 0  = Error

        let gtc = Arc::new(gtc);
        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }


        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let gtc = gtc.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (mut z, mut z2, mut angle, mut distance): (f64, f64, f64, f64);
                let (mut r, mut c, mut d): (isize, isize, isize);
                let (mut x1, mut x2, mut xdif): (f64, f64, f64);
                let (mut y1, mut y2, mut ydif): (f64, f64, f64);
                let (mut zenith_distance, mut nadir_distance): (f64, f64);
                let (mut zenith_threshold, mut nadir_threshold): (f64, f64);
                let (mut zenith_angle, mut nadir_angle): (f64, f64);
                let (mut code, mut power): (usize, usize);
                let (mut count_pos, mut count_neg): (usize, usize);

                let dx = [0,1,1,1,0,-1,-1,-1];  // | 8 | 1 | 2 |
                let dy = [-1,-1,0,1,1,1,0,-1];  // | 7 | 0 | 3 |
                let mut pattern: [u8; 8];       // | 6 | 5 | 4 |

                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodatai16; columns as usize];
                    let skip = (skip_dist_cells + 1) as isize;
                    let columnslessone = columns - 1;
                    let rowslessone = rows - 1;
                    for col in 0..columns {
                        count_pos = 0;
                        count_neg = 0;
                        pattern = [1; 8]; // 0 for balanced ternary

                        if row >= (skip) && row <= (rowslessone - skip)
                            && col >= (skip) && col <= (columnslessone - skip) { // buffer for edges
                            y1 = input.get_y_from_row(row);
                            x1 = input.get_x_from_column(col);
                            z = input.get_value(row, col);
                            if z != nodata {

                                // scan profile in 8 compass directions
                                'directions: for dir in 0..8 {
                                    (zenith_distance, nadir_distance) = (0f64, 0f64);
                                    zenith_angle = -half_pi;
                                    nadir_angle = half_pi;
                                    d = skip;
                                    r = row + (d * dy[dir]);
                                    c = col + (d * dx[dir]);
                                    if r < 0 || r > rowslessone || c < 0 || c > columnslessone {
                                        // reached edge. new direction
                                        continue 'directions;
                                    }
                                    z2 = input.get_value(r,c);
                                    y2 = input.get_y_from_row(r);
                                    x2 = input.get_x_from_column(c);
                                    ydif = y2 - y1;
                                    xdif = x2 - x1;
                                    distance = (ydif*ydif + xdif*xdif).sqrt();

                                    while distance < search_length {
                                        if z2 != nodata { // line-of-sight exists
                                            angle = (z2 - z).atan2(distance);
                                            if angle > zenith_angle { // get max angle
                                                zenith_angle = angle;
                                                zenith_distance = distance;
                                            }
                                            if angle < nadir_angle { // get min angle
                                                nadir_angle = angle;
                                                nadir_distance = distance;
                                            }
                                        }
                                        d += 1;
                                        r = row + (d * dy[dir]);
                                        c = col + (d * dx[dir]);
                                        if r < 0 || r > rowslessone || c < 0 || c > columnslessone {
                                            // reached edge. new direction
                                            continue 'directions;
                                        }
                                        z2 = input.get_value(r,c);
                                        y2 = input.get_y_from_row(r);
                                        x2 = input.get_x_from_column(c);
                                        ydif = y2 - y1;
                                        xdif = x2 - x1;
                                        distance = (ydif*ydif + xdif*xdif).sqrt();
                                    }

                                    // lower flatness threshold if distance exceeds flatness distance
                                    if flat_length < zenith_distance && flat_length > 0f64 {
                                        zenith_threshold = flat_threshold_height.atan2(zenith_distance);
                                    } else { zenith_threshold = flat_thresh; }
                                    if flat_length < nadir_distance && flat_length > 0f64 {
                                        nadir_threshold = flat_threshold_height.atan2(nadir_distance);
                                    } else { nadir_threshold = flat_thresh; }

                                    // classifier function
                                    if zenith_angle.abs() > zenith_threshold
                                        || nadir_angle.abs() > nadir_threshold {
                                        if nadir_angle.abs() < zenith_angle.abs() {
                                            pattern[dir] = 2; // +1 in balanced ternary
                                            count_pos += 1;
                                        }
                                        if nadir_angle.abs() > zenith_angle.abs() {
                                            pattern[dir] = 0; //-1 in balanced ternary
                                            count_neg += 1;
                                        }
                                    } // else flat
                                }

                                if forms {
                                    data[col as usize] = classes[count_neg][count_pos] as f64;
                                } else {
                                    // calculate ternary code from pattern
                                    power = 1;
                                    code = 0;
                                    for p in 0..8 {
                                        code += (pattern[p] as usize) * power;
                                        power *= 3;
                                    }
                                    // extract rotated and mirrored code using the ternary code
                                    data[col as usize] = gtc[code] as f64;
                                }
                            }
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.configs.nodata = nodatai16;
        output.configs.data_type = DataType::I16;
        output.configs.palette = "qual.plt".to_string();
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("Search radius: {}", search_radius_cells));
        output.add_metadata_entry(format!("Flatness threshold: {}", flat_thresh));
        output.add_metadata_entry(format!("Flatness threshold distance: {}", flat_dist_cells));
        output.add_metadata_entry(format!("Skip distance: {}", skip_dist_cells));
        if forms == true {
            output.add_metadata_entry(format!("Output: Forms"));
        } else { output.add_metadata_entry(format!("Output: Ternary pattern")); }

        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

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
            println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));
        }

        Ok(())
    }
}
