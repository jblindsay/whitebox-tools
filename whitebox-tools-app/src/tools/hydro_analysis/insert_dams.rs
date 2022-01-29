/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 19/02/2020
Last Modified: 20/02/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to insert dams at one or more user-specified points (`--dam_pts`), and of a maximum length
/// (`--damlength`), within an input digital elevation model (DEM) (`--dem`). This tool can be thought of as providing
/// the impoundment feature that is calculated internally during a run of the the impoundment size index (ISI) tool for
/// a set of points of interest. from a  (DEM).
///
/// # Reference
/// Lindsay, JB (2015) Modelling the spatial pattern of potential impoundment size from DEMs.
/// Online resource: [Whitebox Blog](https://whiteboxgeospatial.wordpress.com/2015/04/29/modelling-the-spatial-pattern-of-potential-impoundment-size-from-dems/)
///
/// # See Also
/// `ImpoundmentSizeIndex`, `StochasticDepressionAnalysis`
pub struct InsertDams {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl InsertDams {
    pub fn new() -> InsertDams {
        // public constructor
        let name = "InsertDams".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Calculates the impoundment size resulting from damming a DEM.".to_string();

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
            name: "Input Dam Points".to_owned(),
            flags: vec!["--dam_pts".to_owned()],
            description: "Input vector dam points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Max dam length (grid cells)".to_owned(),
            flags: vec!["--damlength".to_owned()],
            description: "Maximum length of the dam.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif --dam_pts=dams.shp -o=out.tif --damlength=11", short_exe, name).replace("*", &sep);

        InsertDams {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for InsertDams {
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
        let mut dam_file = String::new();
        let mut dam_length = 111f64;

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
            if flag_val == "-i" || flag_val == "-dem" {
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
            } else if flag_val == "-dam_pts" {
                dam_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-damlength" {
                dam_length = if keyval {
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }
        if !dam_file.contains(&sep) && !dam_file.contains("/") {
            dam_file = format!("{}{}", working_directory, dam_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Raster::new(&input_file, "r").expect("Error reading input DEM file.");
        let nodata = input.configs.nodata;

        let dam_pts = Shapefile::read(&dam_file).expect("Error reading input dam file.");

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output
            .set_data_from_raster(&input)
            .expect("Error copying data to output file.");
        output.configs.palette = input.configs.palette.clone();

        let start = Instant::now();

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        // The following perpendicular direction represent perpendiculars
        // to the NE-SW, E-W, SE-NW, and N-S directions.
        let perpendicular1 = [2, 3, 4, 1];
        let perpendicular2 = [6, 7, 0, 5];
        let half_dam_length = (dam_length / 2f64).floor() as usize;
        let dam_profile_length = half_dam_length * 2 + 1;
        let mut dam_profile = vec![0f64; dam_profile_length];
        let mut dam_profile_filled = vec![0f64; dam_profile_length];
        let (mut perp_dir1, mut perp_dir2): (i8, i8);
        let mut z: f64;
        let mut z_n: f64;
        let (mut r_n, mut c_n, mut r_n2, mut c_n2): (isize, isize, isize, isize);
        let (mut target_row, mut target_col): (isize, isize);
        let mut profile_intersects_target: bool;
        let mut max_dam_height: f64;
        let mut dam_z: f64;
        let mut target_cell: usize;
        let (mut dam_row, mut dam_col): (isize, isize);
        let mut dam_dir: usize;
        let mut best_dam_profile_filled: Vec<f64>; // = vec![0f64; dam_profile_length];
                                                   /* Calculate dam heights
                                                   Each cell will be assigned the altitude (ASL) of the highest dam that
                                                   passes through the cell. Potential dams are calculated for each
                                                   grid cell in the N-S, NE-SW, E-W, SE-NW directions.
                                                   */
        let mut num_points_found = 0;
        for record_num in 0..dam_pts.num_records {
            let record = dam_pts.get_record(record_num);
            target_row = input.get_row_from_y(record.points[0].y);
            target_col = input.get_column_from_x(record.points[0].x);
            dam_z = input.get_value(target_row, target_col);
            dam_row = 0;
            dam_col = 0;
            dam_dir = 0;
            max_dam_height = f64::MIN;
            // dam_z = f64::MIN;
            best_dam_profile_filled = vec![0f64; dam_profile_length];
            for row in
                (target_row - half_dam_length as isize)..=(target_row + half_dam_length as isize)
            {
                for col in (target_col - half_dam_length as isize)
                    ..=(target_col + half_dam_length as isize)
                {
                    z = input.get_value(row, col);
                    if z != nodata {
                        // dam_z = z;
                        for dir in 0..4 {
                            profile_intersects_target = false;
                            target_cell = 0;

                            // what is the perpendicular direction?
                            perp_dir1 = perpendicular1[dir];
                            perp_dir2 = perpendicular2[dir];
                            dam_profile[half_dam_length] = input.get_value(row, col);

                            // find the profile elevations
                            r_n = row;
                            c_n = col;
                            r_n2 = row;
                            c_n2 = col;
                            for i in 1..=half_dam_length {
                                r_n += dy[perp_dir1 as usize];
                                c_n += dx[perp_dir1 as usize];
                                if r_n == target_row && c_n == target_col {
                                    profile_intersects_target = true;
                                    target_cell = half_dam_length + i as usize;
                                }
                                z_n = input.get_value(r_n, c_n);
                                if z_n != nodata {
                                    dam_profile[half_dam_length + i as usize] = z_n;
                                } else {
                                    dam_profile[half_dam_length + i as usize] = f64::NEG_INFINITY;
                                }

                                r_n2 += dy[perp_dir2 as usize];
                                c_n2 += dx[perp_dir2 as usize];
                                if r_n2 == target_row && c_n2 == target_col {
                                    profile_intersects_target = true;
                                    target_cell = half_dam_length - i as usize;
                                }
                                z_n = input.get_value(r_n2, c_n2);
                                if z_n != nodata {
                                    dam_profile[half_dam_length - i] = z_n;
                                } else {
                                    dam_profile[half_dam_length - i] = f64::NEG_INFINITY;
                                }
                            }

                            if profile_intersects_target {
                                dam_profile_filled[0] = dam_profile[0];
                                for i in 1..dam_profile_length - 1 {
                                    if dam_profile_filled[i - 1] > dam_profile[i] {
                                        dam_profile_filled[i] = dam_profile_filled[i - 1];
                                    } else {
                                        dam_profile_filled[i] = dam_profile[i];
                                    }
                                }

                                dam_profile_filled[dam_profile_length - 1] =
                                    dam_profile[dam_profile_length - 1];
                                for i in (1..dam_profile_length - 1).rev() {
                                    if dam_profile_filled[i + 1] > dam_profile[i] {
                                        if dam_profile_filled[i + 1] < dam_profile_filled[i] {
                                            dam_profile_filled[i] = dam_profile_filled[i + 1];
                                        }
                                    } else {
                                        dam_profile_filled[i] = dam_profile[i];
                                    }
                                }

                                if dam_profile_filled[target_cell] > max_dam_height {
                                    max_dam_height = dam_profile_filled[target_cell];
                                    dam_row = row;
                                    dam_col = col;
                                    dam_dir = dir;
                                    best_dam_profile_filled = dam_profile_filled.clone();
                                }
                            }
                        }
                    }
                }
            }

            if max_dam_height > f64::MIN && max_dam_height > dam_z {
                // perform the actual damming
                perp_dir1 = perpendicular1[dam_dir];
                perp_dir2 = perpendicular2[dam_dir];

                r_n = dam_row;
                c_n = dam_col;
                r_n2 = dam_row;
                c_n2 = dam_col;
                if best_dam_profile_filled[half_dam_length as usize] > output.get_value(r_n, c_n) {
                    output.set_value(r_n, c_n, best_dam_profile_filled[half_dam_length as usize]);
                }
                if best_dam_profile_filled[half_dam_length as usize]
                    > output.get_value(r_n - 1, c_n)
                {
                    output.set_value(
                        r_n - 1,
                        c_n,
                        best_dam_profile_filled[half_dam_length as usize],
                    );
                }
                for i in 1..=half_dam_length {
                    r_n += dy[perp_dir1 as usize];
                    c_n += dx[perp_dir1 as usize];
                    z_n = input.get_value(r_n, c_n);
                    if z_n != nodata {
                        if best_dam_profile_filled[half_dam_length + i as usize]
                            > output.get_value(r_n, c_n)
                        {
                            output.set_value(
                                r_n,
                                c_n,
                                best_dam_profile_filled[half_dam_length + i as usize],
                            );
                        }
                        if dam_dir == 0 || dam_dir == 2 {
                            // diagonal
                            if best_dam_profile_filled[half_dam_length + i as usize]
                                > output.get_value(r_n - 1, c_n)
                            {
                                output.set_value(
                                    r_n - 1,
                                    c_n,
                                    best_dam_profile_filled[half_dam_length + i as usize],
                                );
                            }
                        }
                    }

                    r_n2 += dy[perp_dir2 as usize];
                    c_n2 += dx[perp_dir2 as usize];
                    z_n = input.get_value(r_n2, c_n2);
                    if z_n != nodata {
                        if best_dam_profile_filled[half_dam_length - i as usize]
                            > output.get_value(r_n2, c_n2)
                        {
                            output.set_value(
                                r_n2,
                                c_n2,
                                best_dam_profile_filled[half_dam_length - i as usize],
                            );
                        }
                    }
                    if dam_dir == 0 || dam_dir == 2 {
                        // diagonal
                        if best_dam_profile_filled[half_dam_length - i as usize]
                            > output.get_value(r_n2 - 1, c_n2)
                        {
                            output.set_value(
                                r_n2 - 1,
                                c_n2,
                                best_dam_profile_filled[half_dam_length - i as usize],
                            );
                        }
                    }
                }

                num_points_found += 1;
            } else {
                // No dam was found that covered the point
                if verbose {
                    println!("Warning: No dam could be identified for Point {} due to its position in non-impoundable terrain.", record_num+1);
                }
            }

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (dam_pts.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if num_points_found == 0 {
            println!(
                "No points were dammed. If this is an unexpected result, it may be that the 
                coordinate reference system (CRS) are not the same for the DEM and points file."
            )
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Dam length: {}", dam_length));
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
