/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: January 10, 2018
Last Modified: January 10, 2018
License: MIT

Help: This tool can be used to calculate the viewshed (i.e. the visible area) from a 
location (i.e. viewing station) or group of locations based on the topography defined 
by an input digital elevation model (DEM). The user must specify the name of the input 
DEM, a viewing station input vector file, the output file name, and the viewing height. 
Viewing station locations are specified as points within an input shapefile. The output 
image indicates the number of stations visible from each grid cell. The viewing height 
is in the same units as the elevations of the DEM and represent a height above the ground 
elevation from which the viewshed is calculated. Viewshed analysis is a very 
computationally intensive task. Depending on the size of the input DEM grid and the 
number of viewing stations, this operation may take considerable time to complete.
*/

use num_cpus;
use raster::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use structures::Array2D;
use time;
use tools::*;
use vector::*;

pub struct Viewshed {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl Viewshed {
    /// public constructor
    pub fn new() -> Viewshed {
        let name = "Viewshed".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Identifies the viewshed for a point or set of points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Viewing Station Vector File".to_owned(),
            flags: vec!["--stations".to_owned()],
            description: "Input viewing station vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )), //ExistingFile(ParameterFileType::Raster),
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
            name: "Station Height (in z units)".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "Viewing station height, in z units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem='dem.tif' --stations='stations.shp' -o=output.tif --height=10.0", short_exe, name).replace("*", &sep);

        Viewshed {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for Viewshed {
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
        let mut stations_file = String::new();
        let mut output_file = String::new();
        let mut height = 2.0;

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no paramters.",
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
            } else if flag_val == "-stations" || flag_val == "-station" {
                stations_file = if keyval {
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
            } else if flag_val == "-height" {
                height = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let num_procs = num_cpus::get() as isize;

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !stations_file.contains(&sep) && !stations_file.contains("/") {
            stations_file = format!("{}{}", working_directory, stations_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let dem = Arc::new(Raster::new(&input_file, "r")?);

        let start = time::now();

        if height < 0f64 {
            println!("Warning: Input station height cannot be less than zero.");
            height = 0f64;
        }

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;

        // let stations = Arc::new(Raster::new(&stations_file, "r")?);
        // let stations = Raster::new(&stations_file, "r")?;
        let stations = Shapefile::read(&stations_file)?;

        // make sure the input vector file is of points type
        if stations.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let mut output = Raster::initialize_using_file(&output_file, &dem);

        // scan the stations raster and place each non-zero grid cell into Vecs
        // let mut z: f64;
        let mut station_x = vec![];
        let mut station_y = vec![];
        // for row in 0..rows {
        //     for col in 0..columns {
        //         z = stations.get_value(row, col);
        //         if z > 0f64 && dem.get_value(row, col) != nodata {
        //             station_x.push(stations.get_x_from_column(col));
        //             station_y.push(stations.get_y_from_row(row));
        //         }
        //     }

        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Locating stations: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        for record_num in 0..stations.num_records {
            let record = stations.get_record(record_num);
            station_y.push(record.points[0].y);
            station_x.push(record.points[0].x);

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (stations.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Locating view stations: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let (mut stn_x, mut stn_y): (f64, f64);
        let mut stn_z: f64;
        let (mut stn_row, mut stn_col): (isize, isize);
        let mut view_angle: Array2D<f32> = Array2D::new(rows, columns, -32768f32, -32768f32)?;
        let mut stn_num = 0;
        let num_stn = station_x.len();
        while !station_x.is_empty() {
            stn_num += 1;
            println!("Station {} of {}", stn_num, num_stn);

            stn_x = station_x.pop().unwrap();
            stn_col = dem.get_column_from_x(stn_x);
            stn_y = station_y.pop().unwrap();
            stn_row = dem.get_row_from_y(stn_y);
            stn_z = dem.get_value(stn_row, stn_col) + height;

            if (stn_col < 0 || stn_col >= columns) && (stn_row < 0 || stn_row >= rows) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "The input stations is not located within the footprint of the DEM.",
                ));
            }

            // now calculate the view angle
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let dem = dem.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let (mut x, mut y): (f64, f64);
                    let mut z: f64;
                    let mut dz: f64;
                    let mut dist: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data: Vec<f32> = vec![-32768f32; columns as usize];
                        for col in 0..columns {
                            z = dem.get_value(row, col);
                            if z != nodata {
                                x = dem.get_x_from_column(col);
                                y = dem.get_y_from_row(row);
                                dz = z - stn_z;
                                dist =
                                    ((x - stn_x) * (x - stn_x) + (y - stn_y) * (y - stn_y)).sqrt();
                                if dist != 0.0 {
                                    data[col as usize] = (dz / dist * 1000f64) as f32;
                                } else {
                                    data[col as usize] = 0f32;
                                }
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            for r in 0..rows {
                let (row, data) = rx.recv().unwrap();
                view_angle.set_row_data(row, data);

                if verbose {
                    progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Calculating view angle (Station {} of {}): {}%",
                            stn_num, num_stn, progress
                        );
                        old_progress = progress;
                    }
                }
            }

            let mut max_view_angle: Array2D<f32> =
                Array2D::new(rows, columns, -32768f32, -32768f32)?;

            let mut z: f32;

            // perform the simple scan lines.
            for row in stn_row - 1..stn_row + 2 {
                for col in stn_col - 1..stn_col + 2 {
                    max_view_angle.set_value(row, col, view_angle.get_value(row, col));
                }
            }

            let mut max_va = view_angle.get_value(stn_row - 1, stn_col);
            for row in (0..stn_row - 1).rev() {
                z = view_angle.get_value(row, stn_col);
                if z > max_va {
                    max_va = z;
                }
                max_view_angle.set_value(row, stn_col, max_va);
            }

            max_va = view_angle.get_value(stn_row + 1, stn_col);
            for row in stn_row + 2..rows {
                z = view_angle.get_value(row, stn_col);
                if z > max_va {
                    max_va = z;
                }
                max_view_angle.set_value(row, stn_col, max_va);
            }

            max_va = view_angle.get_value(stn_row, stn_col + 1);
            for col in stn_col + 2..columns {
                z = view_angle.get_value(stn_row, col);
                if z > max_va {
                    max_va = z;
                }
                max_view_angle.set_value(stn_row, col, max_va);
            }

            max_va = view_angle.get_value(stn_row, stn_col - 1);
            for col in (0..stn_col - 1).rev() {
                z = view_angle.get_value(stn_row, col);
                if z > max_va {
                    max_va = z;
                }
                max_view_angle.set_value(stn_row, col, max_va);
            }

            //solve the first triangular facet
            let mut tva: f32;
            let mut va: f32;
            let mut t1: f32;
            let mut t2: f32;
            let mut vert_count = 1f32;
            let mut horiz_count: f32;
            for row in (0..stn_row - 1).rev() {
                vert_count += 1f32;
                horiz_count = 0f32;
                for col in stn_col + 1..stn_col + (vert_count as isize) + 1 {
                    if col <= columns {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row + 1, col - 1);
                            t2 = max_view_angle.get_value(row + 1, col);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row + 1, col - 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            //solve the second triangular facet
            vert_count = 1f32;
            for row in (0..stn_row - 1).rev() {
                vert_count += 1f32;
                horiz_count = 0f32;
                for col in (stn_col - (vert_count as isize)..stn_col).rev() {
                    if col >= 0 {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row + 1, col + 1);
                            t2 = max_view_angle.get_value(row + 1, col);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row + 1, col + 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the third triangular facet
            vert_count = 1f32;
            for row in stn_row + 2..rows {
                vert_count += 1f32;
                horiz_count = 0f32;
                for col in (stn_col - (vert_count as isize)..stn_col).rev() {
                    if col >= 0 {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row - 1, col + 1);
                            t2 = max_view_angle.get_value(row - 1, col);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row - 1, col + 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the fourth triangular facet
            vert_count = 1f32;
            for row in stn_row + 2..rows {
                vert_count += 1f32;
                horiz_count = 0f32;
                for col in stn_col + 1..stn_col + (vert_count as isize) + 1 {
                    if col < columns {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row - 1, col - 1);
                            t2 = max_view_angle.get_value(row - 1, col);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row - 1, col - 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the fifth triangular facet
            vert_count = 1f32;
            for col in stn_col + 2..columns {
                vert_count += 1f32;
                horiz_count = 0f32;
                for row in (stn_row - (vert_count as isize)..stn_row).rev() {
                    if row >= 0 {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row + 1, col - 1);
                            t2 = max_view_angle.get_value(row, col - 1);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row + 1, col - 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the sixth triangular facet
            vert_count = 1f32;
            for col in stn_col + 2..columns {
                vert_count += 1f32;
                horiz_count = 0f32;
                for row in stn_row + 1..stn_row + (vert_count as isize) + 1 {
                    if row < rows {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row - 1, col - 1);
                            t2 = max_view_angle.get_value(row, col - 1);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row - 1, col - 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the seventh triangular facet
            vert_count = 1f32;
            for col in (0..stn_col - 1).rev() {
                vert_count += 1f32;
                horiz_count = 0f32;
                for row in stn_row + 1..stn_row + (vert_count as isize) + 1 {
                    if row < rows {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row - 1, col + 1);
                            t2 = max_view_angle.get_value(row, col + 1);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row - 1, col + 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            // solve the eigth triangular facet
            vert_count = 1f32;
            for col in (0..stn_col - 1).rev() {
                vert_count += 1f32;
                horiz_count = 0f32;
                for row in (stn_row - (vert_count as isize)..stn_row).rev() {
                    if row < rows {
                        va = view_angle.get_value(row, col);
                        horiz_count += 1f32;
                        if horiz_count != vert_count {
                            t1 = max_view_angle.get_value(row + 1, col + 1);
                            t2 = max_view_angle.get_value(row, col + 1);
                            tva = t2 + horiz_count / vert_count * (t1 - t2);
                        } else {
                            tva = max_view_angle.get_value(row + 1, col + 1);
                        }
                        if tva > va {
                            max_view_angle.set_value(row, col, tva);
                        } else {
                            max_view_angle.set_value(row, col, va);
                        }
                    } else {
                        break;
                    }
                }
            }

            let mut value: f64;
            for row in 0..rows {
                for col in 0..columns {
                    // z = max_view_angle.get_value(row, col);
                    // if z > -32768f32 {
                    //     output.set_value(row, col, z as f64);
                    // } else {
                    //     output.set_value(row, col, nodata);
                    // }
                    if dem.get_value(row, col) != nodata {
                        value = if max_view_angle.get_value(row, col)
                            > view_angle.get_value(row, col)
                        {
                            0f64
                        } else {
                            1f64
                        };
                        output.increment(row, col, value);
                    }
                }

                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Creating output: (Station {} of {}): {}%",
                            stn_num, num_stn, progress
                        );
                        old_progress = progress;
                    }
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        // output.configs.palette = "grey.plt".to_string();
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("DEM file: {}", input_file));
        output.add_metadata_entry(
            format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
        );

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
