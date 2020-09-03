/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/07/2020
Last Modified: 03/09/2020
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
use chrono::prelude::*;
use chrono::{Date, FixedOffset, NaiveTime, TimeZone};

/// This tool calculates the proportion of time a location is within daylight (i.e. outside of an area of shadow cast by a local object).
pub struct TimeInDaylight {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TimeInDaylight {
    /// public constructor
    pub fn new() -> TimeInDaylight {
        let name = "TimeInDaylight".to_string();
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
            default_value: Some("10.0".to_owned()),
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

        parameters.push(ToolParameter {
            name: "UTC Offset (e.g. -04:00, +06:00)".to_owned(),
            flags: vec!["--utc_offset".to_owned()],
            description: "UTC time offset, in hours (e.g. -04:00, +06:00).".to_owned(),
            parameter_type: ParameterType::String,
            default_value: Some("00:00".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Start Day Of The Year (1-365)".to_owned(),
            flags: vec!["--start_day".to_owned()],
            description: "Start day of the year (1-365).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "End Day Of The Year (1-365)".to_owned(),
            flags: vec!["--end_day".to_owned()],
            description: "End day of the year (1-365).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("365".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Starting Hour (24-hour time: HH:MM:SS e.g. 05:00:00)".to_owned(),
            flags: vec!["--start_time".to_owned()],
            description: "Starting hour to track shadows (e.g. 5, 5:00, 05:00:00). Assumes 24-hour time: HH:MM:SS. 'sunrise' is also a valid time.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: Some("00:00:00".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Ending Hour (24-hour time: HH:MM:SS e.g. 21:00:00)".to_owned(),
            flags: vec!["--end_time".to_owned()],
            description: "Starting hour to track shadows (e.g. 21, 21:00, 21:00:00). Assumes 24-hour time: HH:MM:SS. 'sunset' is also a valid time.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: Some("23:59:59".to_string()),
            optional: true,
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

        TimeInDaylight {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TimeInDaylight {
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
        let mut az_fraction = 10.0f32;
        let mut max_dist = f32::INFINITY;
        let mut latitude = 0f32;
        let mut longitude = 0f32;
        // Guelph:
        // let mut latitude = 43.5448;
        // let mut longitude = -80.2482;
        let mut utc_offset = 0f64;
        let mut start_day = 1u32;
        let mut end_day = 365u32;
        let mut start_time = NaiveTime::from_hms(0, 0, 0); // midnight
        let mut end_time = NaiveTime::from_hms(23, 59, 59); // the second before midnight

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
            } else if flag_val == "-utc_offset" || flag_val == "--utc" {
                let val = if keyval {
                    vec[1].to_string().replace("00", "0").replace("+", "").replace("UTC", "")
                } else {
                    args[i + 1].to_string().replace("00", "0").replace("+", "").replace("UTC", "")
                };
                let val2: Vec<&str> = val.split(":").collect();
                
                utc_offset = val2[0].parse::<f64>().expect(&format!("Error parsing {}", flag_val));
                if val2.len() > 1 {
                    utc_offset += val2[0].parse::<f64>().expect(&format!("Error parsing {}", flag_val)) / 60f64;
                }
                if utc_offset < -12f64 || utc_offset > 12f64 {
                    panic!("The UTC offset must be between -12:00 and +12:00");
                }
            } else if flag_val == "-start_day" {
                start_day = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<u32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<u32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-end_day" {
                end_day = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<u32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<u32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-start_time" {
                let val = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                // The program is already confined to analyzing times between sunrise and sunset. Thus,
                // the default setting for end_time of 00:00:00 is sufficient to work as sunrise. There
                // is no need to parse this value.
                if !val.to_lowercase().contains("sunrise") {
                    let time: Vec<&str> = val.split(":").collect();
                    let hr = time[0].parse::<i32>().expect(&format!("Error parsing {}", flag_val));
                    let min = if time.len() > 1 {
                        time[1].parse::<i32>().expect(&format!("Error parsing {}", flag_val))
                    } else {
                        0i32
                    };
                    let sec = if time.len() > 2 {
                        time[2].parse::<i32>().expect(&format!("Error parsing {}", flag_val))
                    } else {
                        0i32
                    };
                    if hr >= 0 && hr < 24 && min >= 0 && min < 60 && sec >= 0 && sec < 60 {
                        start_time = NaiveTime::from_hms(hr as u32, min as u32, sec as u32);
                    } else {
                        panic!("Invalid start time.");
                    }
                }
            } else if flag_val == "-end_time" {
                let val = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                // The program is already confined to analyzing times between sunrise and sunset. Thus,
                // the default setting for end_time of 23:59:59 is sufficient to work as sunset. There
                // is no need to parse this value.
                if !val.to_lowercase().contains("sunset") {
                    let time: Vec<&str> = val.split(":").collect();
                    let hr = time[0].parse::<i32>().expect(&format!("Error parsing {}", flag_val));
                    let min = if time.len() > 1 {
                        time[1].parse::<i32>().expect(&format!("Error parsing {}", flag_val))
                    } else {
                        0i32
                    };
                    let sec = if time.len() > 2 {
                        time[2].parse::<i32>().expect(&format!("Error parsing {}", flag_val))
                    } else {
                        0i32
                    };
                    if hr >= 0 && hr < 24 && min >= 0 && min < 60 && sec >= 0 && sec < 60 {
                        end_time = NaiveTime::from_hms(hr as u32, min as u32, sec as u32);
                    } else {
                        panic!("Invalid end time.");
                    }
                }
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

        if az_fraction <= 0f32 {
            panic!("The az_fraction parameter must be larger than zero.");
        }

        if az_fraction >= 3600f32 {
            panic!("The az_fraction parameter must smaller than 360.");
        }

        if latitude > 90.0 || latitude < -90.0 {
            panic!("The specified latitude must be between -90 and 90 degrees.");
        }

        if longitude < -180.0 || longitude > 180.0 {
            panic!("The specified longitude must be between -180 and 180 degrees.");
        }

        if end_time < start_time {
            panic!("The start time must occur before the end time.");
        }

        if start_day > 365 {
            panic!("The start day must be between 1 and 365.");
        }

        if end_day > 365 {
            panic!("The start day must be between 1 and 365.");
        }

        if end_day < start_day {
            panic!("The start day must occur before the end day.");
        }

        // Calculate the almanac
        if verbose {
            println!("Calculating the local almanac...");
        }
        let seconds_interval = 10;
        let almanac = generate_almanac(
            latitude as f64,
            longitude as f64, 
            utc_offset,
            az_fraction as f64,
            seconds_interval
        );

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
                cell_size_x = cell_size_x * (111320.0 * mid_lat.cos());
                cell_size_y = cell_size_y * (111320.0 * mid_lat.cos());
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
        // altitudes_and_durations key: altitude, duration, time (as NaiveTime), day (as ordinal)
        let mut altitudes_and_durations = vec![(0f32, 0f64, NaiveTime::from_hms(0, 0, 0), 0u32); 365];
        let mut horizon_angle: Array2D<f32> = Array2D::new(rows, columns, 0f32, nodata_f32).expect("Error creating Array2D");
        let mut total_daylight = 0f64;
        let mut line_slope: f32;
        let mut bin = 0usize;
        while azimuth < 360f32 {
            let mut total_daylight_in_az = 0.0;
            for day in 1..=365usize {
                // if day as u32 >= start_day && day as u32 <= end_day {
                altitudes_and_durations[day-1] = (
                    almanac[day-1].data[bin].altitude as f32, 
                    almanac[day-1].data[bin].duration,
                    almanac[day-1].data[bin].time,
                    almanac[day-1].date.ordinal(),
                );
                if altitudes_and_durations[day-1].3 >= start_day && altitudes_and_durations[day-1].3 <= end_day {
                    if altitudes_and_durations[day-1].2 >= start_time && altitudes_and_durations[day-1].2 <= end_time {
                        // it's in the allowable day range and time-of-day range.
                        if altitudes_and_durations[day-1].1 > 0f64 { // it's daytime.
                            total_daylight_in_az += altitudes_and_durations[day-1].1; 
                        }
                    } 
                }
            }

            total_daylight += total_daylight_in_az;
            
            // Sort the altitudes from high to low.
            altitudes_and_durations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            // if altitudes[0] > 0f32 { // This azimuth sees sunshine at least part of the year.
            if altitudes_and_durations[0].1 > 0f64 && total_daylight_in_az > 0f64 { // This azimuth sees sunshine at least part of the year.
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
                        let mut seconds_in_shadow = 0f64;
                        if *ha != nodata_f32 {
                            for day in 0..365 {
                                if altitudes_and_durations[day].3 >= start_day && altitudes_and_durations[day].3 <= end_day {
                                    if altitudes_and_durations[day].2 >= start_time && altitudes_and_durations[day].2 <= end_time {
                                        if altitudes_and_durations[day].1 > 0f64 { // The sun is up
                                            if altitudes_and_durations[day].0 < *ha { // But it's behind a distant obstacle
                                                seconds_in_shadow += altitudes_and_durations[day].1;
                                            }
                                        } else { 
                                            // because the altitudes are sorted from highest to lowest, we can conclude 
                                            // there are no further daytime days at this azimuth.
                                            break;
                                        }
                                    }
                                }
                            }
                        } else {
                            // num_times_in_shadow = nodata;
                            seconds_in_shadow = nodata;
                        }
                        // num_times_in_shadow
                        seconds_in_shadow
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

            bin += 1;
            azimuth += az_fraction;
        }

        if total_daylight > 0f64 {

            let mut z: f64;
            for row in 0..rows {
                for col in 0..columns {
                    z = output.get_value(row, col);
                    if z != nodata {
                        // output.set_value(row, col, z / total_daylight_time);
                        output.set_value(row, col, 1f64 - (z / total_daylight));
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
        } else {
            println!("No daylight was detected. Either broaden the time-span parameters or decrease the azimuth fraction.");
        }

        Ok(())
    }
}

fn generate_almanac(latitude: f64, longitude: f64, utc_offset: f64, az_interval: f64, seconds_interval: usize) -> Vec<Day> {
    let hour_sec = 3600f64;
    let mut almanac = vec![];
    let mut num_days = 0;
    // let doy = 1; //233;
    for doy in 1..=366 {
        let midnight = if utc_offset < 0f64 {
            FixedOffset::west((utc_offset.abs() * hour_sec) as i32)
                .yo(2020, doy as u32)
                .and_hms(0, 0, 0)
        } else {
            FixedOffset::east((utc_offset * hour_sec) as i32)
                .yo(2020, doy as u32)
                .and_hms(0, 0, 0)
        };
        
        let mut diff: f64;
        let mut sunrise = false;
        let num_bins = (360.0f64 / az_interval).ceil() as usize;
        almanac.push(Day {
            date: midnight.date(),
            sunrise: PositionTime::default(),
            sunset: PositionTime::default(),
            data: vec![PositionTime::default(); num_bins],
        });

        for hr in 0..24 {
            for minute in 0..60 {
                for sec in (0..=45).step_by(seconds_interval) {
                    let dt = if utc_offset < 0f64 {
                        FixedOffset::west((utc_offset.abs() * hour_sec) as i32)
                            .yo(2020, doy as u32)
                            .and_hms(hr, minute, sec)
                    } else {
                        FixedOffset::east((utc_offset * hour_sec) as i32)
                            .yo(2020, doy as u32)
                            .and_hms(hr, minute, sec)
                    };
                    let unixtime = dt.timestamp() * 1000 + dt.timestamp_subsec_millis() as i64;
                    let pos = pos(unixtime, latitude, longitude);
                    let az_actual = pos.azimuth.to_degrees();
                    let alt = pos.altitude.to_degrees();
                    if alt >= -0.5 && !sunrise {
                        almanac[num_days].sunrise.azimuth = az_actual;
                        almanac[num_days].sunrise.actual_azimuth = az_actual;
                        almanac[num_days].sunrise.altitude = alt;
                        almanac[num_days].sunrise.time = dt.time();
                        sunrise = true;
                    }
                    if alt <= -0.5 && sunrise {
                        almanac[num_days].sunset.azimuth = az_actual;
                        almanac[num_days].sunset.actual_azimuth = az_actual;
                        almanac[num_days].sunset.altitude = alt;
                        almanac[num_days].sunset.time = dt.time();
                        sunrise = false;
                    }
                    let mut bin = (az_actual / az_interval).round() as usize;
                    let mut bin_val = bin as f64 * az_interval;
                    diff = (bin_val - az_actual).abs();
                    if bin_val == 360f64 {
                        bin_val = 0f64;
                        bin = 0;
                    }
                    if diff < almanac[num_days].data[bin].diff {
                        almanac[num_days].data[bin].diff = diff;
                        almanac[num_days].data[bin].azimuth = bin_val;
                        almanac[num_days].data[bin].actual_azimuth = az_actual;
                        almanac[num_days].data[bin].altitude = alt;
                        almanac[num_days].data[bin].time = dt.time();
                    }
                    if alt >= -0.5 {
                        almanac[num_days].data[bin].duration += seconds_interval as f64;
                    }
                }
            }
        }
        
        num_days += 1;
    }
    
    almanac
}

pub struct Day {
    date: Date<FixedOffset>,
    sunrise: PositionTime,
    sunset: PositionTime,
    data: Vec<PositionTime>,
}

// impl Day {
//     pub fn sort_by_time(&mut self) {
//         self.data
//             .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
//     }
    
//     pub fn sort_by_az(&mut self) {
//         self.data
//             .sort_by(|a, b| a.azimuth.partial_cmp(&b.azimuth).unwrap());
//     }
// }

#[derive(Debug, Clone)]
pub struct PositionTime {
    azimuth: f64, // in degrees
    actual_azimuth: f64, // in degrees; because we are finding the closest time/position to the target azimuth, this won't be the same as azimuth, with proximity determined by the temporal resolution
    altitude: f64, // in degrees
    time: NaiveTime,
    diff: f64, // only used for the approximation of azimuth
    duration: f64, // in seconds
}

impl PositionTime {
    fn default() -> PositionTime {
        PositionTime {
            azimuth: 0f64,
            actual_azimuth: 0f64,
            altitude: 0f64,
            time: NaiveTime::from_hms(0, 0, 0),
            diff: 360f64,
            duration: 0f64,
        }
    }
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

const MILLISECONDS_PER_DAY: f64 = (1000 * 60 * 60 * 24) as f64;
const J1970: f64 = 2_440_588f64;
const J2000: f64 = 2_451_545f64;
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
    unixtime_in_ms as f64 / (MILLISECONDS_PER_DAY) - 0.5 + J1970
}

fn to_days(unixtime_in_ms: i64) -> f64 {
    to_julian(unixtime_in_ms) - J2000
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
