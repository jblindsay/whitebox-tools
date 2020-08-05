/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/07/2020
Last Modified: 29/07/2020
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::f64::consts::PI;
use crate::structures::Array2D;
use rayon::prelude::*;

/// This tool calculates the proportion of time a location is within an area of shadow.
pub struct ShadowTime {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ShadowTime {
    /// public constructor
    pub fn new() -> ShadowTime {
        let name = "ShadowTime".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Calculates the proportion of time a location is within an area of shadow."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Azimuth Fraction".to_owned(),
            flags: vec!["--az_fraction".to_owned()],
            description: "Azimuth fraction in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("12.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Search Distance".to_owned(),
            flags: vec!["--max_dist".to_owned()],
            description: "Optional maximum search distance. Minimum value is 5 x cell size."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("100.0".to_owned()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Centre Point Latitude".to_owned(),
            flags: vec!["--lat".to_owned()],
            description: "Centre point latitude.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Centre Point Longitude".to_owned(),
            flags: vec!["--long".to_owned()],
            description: "Centre point longitude.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o=output.tif --az_fraction=15.0 --max_dist=100.0 --lat=43.545 --long=-80.248", short_exe, name).replace("*", &sep);

        ShadowTime {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ShadowTime {
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

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut az_fraction = 0.0f32;
        let mut max_dist = f32::INFINITY;
        let mut latitude = 0f32;
        let mut longitude = 0f32;
        // let mut latitude = 43.5448;
        // let mut longitude = -80.2482;
    

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
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
            } else if flag_val == "-az_fraction" {
                az_fraction = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-max_dist" {
                max_dist = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-lat" || flag_val == "--latitude" {
                latitude = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-long" || flag_val == "--longitude" {
                longitude = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
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

        if verbose {
            println!("Reading data...")
        };
        let inputf64 = Arc::new(Raster::new(&input_file, "r")?);
        let configs = inputf64.configs.clone();
        let mut cell_size_x = configs.resolution_x as f32;
        let mut cell_size_y = configs.resolution_y as f32;
        if inputf64.is_in_geographic_coordinates() {
            let mut mid_lat = ((configs.north - configs.south) / 2.0) as f32;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                cell_size_x = cell_size_x * (113200.0 * mid_lat.cos());
                cell_size_y = cell_size_y * (113200.0 * mid_lat.cos());
            }
        }

        if max_dist <= 5f32 * cell_size_x {
            panic!("The maximum search distance parameter (--max_dist) must be larger than 5 x cell size.");
        }

        // The longest that max_dist ever needs to be is the raster diagonal length.
        let diag_length = ((configs.north - configs.south)*(configs.north - configs.south)+(configs.east - configs.west)*(configs.east - configs.west)).sqrt() as f32;
        if max_dist > diag_length {
            max_dist = diag_length;
        }
        
        let input = inputf64.get_data_as_f32_array2d();

        let start = Instant::now();

        let rows = configs.rows as isize;
        let columns = configs.columns as isize;
        let nodata = configs.nodata;
        let nodata_f32 = nodata as f32;

        let mut output = Raster::initialize_using_file(&output_file, &inputf64);

        drop(inputf64);

        let mut azimuth = 0f32;
        let mut altitudes = vec![0f32; 365];
        let mut horizon_angle: Array2D<f32> = Array2D::new(rows, columns, 0f32, nodata_f32).expect("Error creating Array2D");
        let mut total_daylight_time = 0f64;
        let mut line_slope: f32;
        while azimuth < 360f32 {
            for day in 1..=365 {
                altitudes[day-1] = find_alt(azimuth as f64, latitude as f64, longitude as f64, day as i64) as f32;
                if altitudes[day-1] > 0f32 { total_daylight_time += 1f64; } // it's daytime.
            }
            
            // Sort the altitudes from high to low.
            altitudes.sort_by(|a, b| b.partial_cmp(&a).unwrap());

            if altitudes[0] > 0f32 { // This azimuth sees sunshine at least part of the year.

                line_slope = if azimuth < 180f32 {
                    (90f32 - azimuth).to_radians().tan()
                } else {
                    (270f32 - azimuth).to_radians().tan()
                };
        
                // Now perform the filter
                let num_procs = num_cpus::get() as isize;
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {

                        // The ray-tracing operation can be viewed as a linear maximum filter. The first step 
                        // is to create the filter offsets and calculate the offset distances.

                        let x_step: isize;
                        let y_step: isize;
                        if azimuth > 0f32 && azimuth <= 90f32 {
                            x_step = 1;
                            y_step = 1;
                        } else if azimuth <= 180f32 {
                            x_step = 1;
                            y_step = -1;
                        } else if azimuth <= 270f32 {
                            x_step = -1;
                            y_step = -1;
                        } else {
                            x_step = -1;
                            y_step = 1;
                        }

                        let mut flag: bool;
                        let (mut delta_x, mut delta_y): (f32, f32);
                        let (mut x, mut y): (f32, f32);
                        let (mut x1, mut y1): (isize, isize);
                        let (mut x2, mut y2): (isize, isize);
                        let (mut z1, mut z2): (f32, f32);
                        let mut dist: f32;
                        let mut weight: f32;
                        let mut offsets = vec![];

                        // Find all of the horizontal grid intersections.
                        if line_slope != 0f32 { // Otherwise, there are no horizontal intersections.
                            y = 0f32;
                            flag = true;
                            while flag {
                                y += y_step as f32;
                                x = y / line_slope;

                                // calculate the distance
                                delta_x = x * cell_size_x;
                                delta_y = -y * cell_size_y;
                                dist = delta_x.hypot(delta_y); 
                                if dist <= max_dist {
                                    x1 = x.floor() as isize;
                                    x2 = x1 + 1;
                                    y1 = -y as isize;
                                    weight = x - x1 as f32;
                                    offsets.push((x1, y1, x2, y1, weight, dist));
                                } else {
                                    flag = false;
                                }
                            }
                        }

                        // Find all of the vertical grid intersections.
                        x = 0f32;
                        flag = true;
                        while flag {
                            x += x_step as f32;
                            y = -(line_slope * x); // * -1f32;

                            // calculate the distance
                            delta_x = x * cell_size_x;
                            delta_y = y * cell_size_y;

                            dist = delta_x.hypot(delta_y);
                            if dist <= max_dist {
                                y1 = y.floor() as isize;
                                y2 = y1 + 1; // - y_step;
                                x1 = x as isize;
                                weight = y - y1 as f32;
                                offsets.push((x1, y1, x1, y2, weight, dist));
                            } else {
                                flag = false;
                            }
                        }

                        // Sort by distance.
                        offsets.sort_by(|a, b| a.5.partial_cmp(&b.5).unwrap());

                        let num_offsets = offsets.len();
                        let mut z: f32;
                        let mut slope: f32;
                        let early_stopping_slope = 80f32.to_radians().tan();
                        let mut current_elev: f32;
                        let mut current_max_slope: f32;
                        let mut current_max_elev: f32;
                        let a_small_value = -9999999f32;
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data: Vec<f32> = vec![nodata_f32; columns as usize];
                            for col in 0..columns {
                                current_elev = input.get_value(row, col);
                                if current_elev != nodata_f32 {

                                    // Run down the offsets of the ray
                                    current_max_slope = a_small_value;
                                    current_max_elev = a_small_value;
                                    for i in 0..num_offsets {
                                        // Where are we on the grid?
                                        x1 = col + offsets[i].0;
                                        y1 = row + offsets[i].1;
                                        x2 = col + offsets[i].2;
                                        y2 = row + offsets[i].3;
                                        
                                        // What is the elevation?
                                        z1 = input.get_value(y1, x1);
                                        z2 = input.get_value(y2, x2);

                                        if z1 == nodata_f32 && z2 == nodata_f32 {
                                            break; // We're likely off the grid.
                                        } else if z1 == nodata_f32 {
                                            z1 = z2;
                                        } else if z2 == nodata_f32 {
                                            z2 = z1;
                                        }

                                        z = z1 + offsets[i].4 * (z2 - z1);

                                        // All previous cells are nearer, and so if this isn't a higher 
                                        // cell than the current highest, it can't be the horizon cell.
                                        if z > current_max_elev {
                                            current_max_elev = z;
                                            
                                            // Calculate the slope
                                            slope = (z - current_elev) / offsets[i].5;
                                            if slope > current_max_slope {
                                                current_max_slope = slope;
                                                if slope > early_stopping_slope {
                                                    break; // we're unlikely to find a farther horizon cell.
                                                }
                                            }
                                        }
                                    }

                                    if current_max_slope == a_small_value {
                                        data[col as usize] = 0f32; // It's a zero-length scan. We didn't encounter any valid cells.
                                    } else {
                                        data[col as usize] = current_max_slope.atan().to_degrees();
                                    }
                                }
                            }
                            tx.send((row, data)).unwrap();
                        }
                    });
                }

                for row in 0..rows {
                    let (r, data) = rx.recv().expect("Error receiving data from thread.");
                    horizon_angle.set_row_data(r, data);
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress (Az={}): {}%", azimuth, progress);
                            old_progress = progress;
                        }
                    }
                }


                // Loop through each cell in the grid, counting the number of days that it is in shadow.
                // Specifically, we want to count the number of days for which the sun is above the 
                // horizon at the azimuth (i.e. altitudes[day] > 0) but below the local horizon angle.

                for row in 0..rows {
                    let values = horizon_angle.get_row_data(row);
                    let new_vals = values.par_iter()
                    .map(|ha| {
                        let mut num_times_in_shadow = 0f64;
                        if *ha != nodata_f32 {
                            for day in 0..365 {
                                if altitudes[day] > 0f32 { // The sun is up
                                    if altitudes[day] < *ha { // But it's behind a distant obstacle
                                        num_times_in_shadow += 1f64;
                                    }
                                } else { 
                                    // because the altitudes are sorted from highest to lowest, we can conclude 
                                    // there are no further daytime days at this azimuth.
                                    break;
                                }
                            }
                        } else {
                            num_times_in_shadow = nodata;
                        }
                        num_times_in_shadow
                    }
                    )
                    .collect();

                    output.increment_row_data(row, new_vals);

                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                        if progress != old_progress {
                            println!("Progress (Az={}): {}%", azimuth, progress);
                            old_progress = progress;
                        }
                    }
                }
            }

            azimuth += az_fraction;
        }

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = output.get_value(row, col);
                if z != nodata {
                    output.set_value(row, col, z / total_daylight_time);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Azimuth fraction: {}", az_fraction));
        output.add_metadata_entry(format!("Max dist: {}", max_dist));
        output.add_metadata_entry(format!("Latitude/Longitude: {}, {}", latitude, longitude));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

fn find_alt(az: f64, lat: f64, lon: f64, julian_day: i64) -> f64 { //, utc_offset: i64) -> f64 {
    let mut unixtime = (julian_day - 1) * 86400000i64; // - utc_offset * 3600000;
    let hour_fraction = 120i64;
    let time_step = 3600000i64 / hour_fraction;
    let mut target_alt = 0f64;
    let mut mindiff = f64::INFINITY;
    let mut diff: f64;
    // let hour: String;
    // let mut t = 0i64;
    for _a in 0..=24*hour_fraction {
        unixtime += time_step;
        let pos = pos(unixtime,lat,lon);
        let az_actual = pos.azimuth.to_degrees();
        let alt = pos.altitude.to_degrees();
        diff = (az - az_actual).abs();
        if diff < mindiff {
            mindiff = diff;
            target_alt = alt;
            // t = unixtime;
        }
    }
    // let millisec = t - (julian_day * 86400000i64 - utc_offset * 3600000);
    // let hr = millisec / 3600000;
    // let min = (millisec % 3600000) / 60000;
    // let mut m = format!("{}", min);
    // if m.len() == 1 {
    //     m = format!("0{}", min);
    // }
    // hour = format!("{}:{}", hr, m);
    
    target_alt
}


// The following code has been modified from the original rust-sun library [https://github.com/flosse/rust-sun]

// The `sun` crate is a library for calculating the position of the sun.
// It is a port of the `JavaScript` library
// [suncalc](https://github.com/mourner/suncalc).
//
// # Example
//
// ```
// let unixtime = 1362441600000;
// let lat = 48.0;
// let lon = 9.0;
// let pos = sun::pos(unixtime,lat,lon);
// let az  = pos.azimuth.to_degrees();
// let alt = pos.altitude.to_degrees();
// println!("The position of the sun is {}/{}", az, alt);
// ```

// use std::time::SystemTime;

// date/time constants and conversions

const MILLISECONDS_PER_DAY: u32 = 1000 * 60 * 60 * 24;
const J1970: u32 = 2_440_588;
const J2000: u32 = 2_451_545;
const TO_RAD: f64 = PI / 180.0;
const OBLIQUITY_OF_EARTH: f64 = 23.4397 * TO_RAD;
const PERIHELION_OF_EARTH: f64 = 102.9372 * TO_RAD;

/// Holds the [azimuth](https://en.wikipedia.org/wiki/Azimuth)
/// and [altitude](https://en.wikipedia.org/wiki/Horizontal_coordinate_system)
/// angles of the sun position.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub azimuth: f64,
    pub altitude: f64,
}

fn to_julian(unixtime_in_ms: i64) -> f64 {
    unixtime_in_ms as f64 / (MILLISECONDS_PER_DAY as f64) - 0.5 + J1970 as f64
}

fn to_days(unixtime_in_ms: i64) -> f64 {
    to_julian(unixtime_in_ms) - J2000 as f64
}

// general calculations for position

fn right_ascension(l: f64, b: f64) -> f64 {
    (l.sin() * OBLIQUITY_OF_EARTH.cos() - b.tan() * OBLIQUITY_OF_EARTH.sin()).atan2(l.cos())
}

fn declination(l: f64, b: f64) -> f64 {
    (b.sin() * OBLIQUITY_OF_EARTH.cos() + b.cos() * OBLIQUITY_OF_EARTH.sin() * l.sin()).asin()
}

fn azimuth(h: f64, phi: f64, dec: f64) -> f64 {
    h.sin().atan2(h.cos() * phi.sin() - dec.tan() * phi.cos()) + PI
}

fn altitude(h: f64, phi: f64, dec: f64) -> f64 {
    (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin()
}

fn sidereal_time(d: f64, lw: f64) -> f64 {
    (280.16 + 360.985_623_5 * d).to_radians() - lw
}

// general sun calculations

fn solar_mean_anomaly(d: f64) -> f64 {
    (357.5291 + 0.985_600_28 * d).to_radians()
}

fn equation_of_center(m: f64) -> f64 {
    (1.9148 * (1.0 * m).sin() + 0.02 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin()).to_radians()
}

fn ecliptic_longitude(m: f64) -> f64 {
    m + equation_of_center(m) + PERIHELION_OF_EARTH + PI
}

/// Calculates the sun position for a given date and latitude/longitude.
/// The angles are calculated as [radians](https://en.wikipedia.org/wiki/Radian).
///
/// * `unixtime`  - [unix time](https://en.wikipedia.org/wiki/Unix_time) in milliseconds.
/// * `lat`       - [latitude](https://en.wikipedia.org/wiki/Latitude) in degrees.
/// * `lon`       - [longitude](https://en.wikipedia.org/wiki/Longitude) in degrees.
/// calculates the sun position for a given date and latitude/longitude
pub fn pos(unixtime_in_ms: i64, lat: f64, lon: f64) -> Position {
    let lw = -lon.to_radians();
    let phi = lat.to_radians();
    let d = to_days(unixtime_in_ms);
    let m = solar_mean_anomaly(d);
    let l = ecliptic_longitude(m);
    let dec = declination(l, 0.0);
    let ra = right_ascension(l, 0.0);
    let h = sidereal_time(d, lw) - ra;

    Position {
        azimuth: azimuth(h, phi, dec),
        altitude: altitude(h, phi, dec),
    }
}
