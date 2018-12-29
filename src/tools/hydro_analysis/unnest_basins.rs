/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/04/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// In some applications it is necessary to relate a measured variable for a group of
/// hydrometric stations (e.g. characteristics of flow timing and duration or water
/// chemistry) to some characteristics of each outlet's catchment (e.g. mean slope,
/// area of wetlands, etc.). When the group of outlets are nested, i.e. some stations
/// are located downstream of others, then performing a watershed operation will
/// result in inappropriate watershed delineation. In particular, the delineated
/// watersheds of each nested outlet will not include the catchment areas of upstream
/// outlets. This creates a serious problem for this type of application.
///
/// The Unnest Basin tool can be used to perform a watershedding operation based on a
/// group of specified pour points, i.e. outlets or target cells, such that each
/// complete watershed is delineated. The user must specify the name of a flow pointer
/// (flow direction) raster, a pour point raster, and the name of the output rasters.
/// Multiple numbered outputs will be created, one for each nesting level. Pour point,
/// or target, cells are denoted in the input pour-point image as any non-zero,
/// non-NoData value. The flow pointer raster should be generated using the D8
/// algorithm.
pub struct UnnestBasins {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl UnnestBasins {
    pub fn new() -> UnnestBasins {
        // public constructor
        let name = "UnnestBasins".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Extract whole watersheds for a set of outlet points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input D8 pointer raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Pour Points (Outlet) File".to_owned(),
            flags: vec!["--pour_pts".to_owned()],
            description: "Input vector pour points (outlet) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
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
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr='d8pntr.tif' --pour_pts='pour_pts.shp' -o='output.tif'", short_exe, name).replace("*", &sep);

        UnnestBasins {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for UnnestBasins {
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
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
        }
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
        let mut d8_file = String::new();
        let mut pourpts_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;

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
            if flag_val == "-d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-pour_pts" {
                pourpts_file = if keyval {
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
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                esri_style = true;
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

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !pourpts_file.contains(&sep) && !pourpts_file.contains("/") {
            pourpts_file = format!("{}{}", working_directory, pourpts_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        if verbose {
            println!("Reading data...")
        };

        let pntr = Raster::new(&d8_file, "r")?;

        let pourpts = Shapefile::read(&pourpts_file)?;

        // make sure the input vector file is of points type
        if pourpts.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = -32768f64; //pour_pts.configs.nodata;
        let pntr_nodata = pntr.configs.nodata;

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -2, -2)?;
        let mut outlet_points: Array2D<isize> = Array2D::new(rows, columns, 0, 0)?;
        let mut outlet_rows = vec![0isize; pourpts.num_records + 1];
        let mut outlet_columns = vec![0isize; pourpts.num_records + 1];
        let mut nesting_order = vec![0usize; pourpts.num_records + 1];
        let mut outlet: usize;

        for record_num in 0..pourpts.num_records {
            let record = pourpts.get_record(record_num);
            outlet = record_num + 1;
            let row = pntr.get_row_from_y(record.points[0].y);
            let col = pntr.get_column_from_x(record.points[0].x);
            outlet_points.set_value(row, col, outlet as isize);
            outlet_rows[outlet] = row;
            outlet_columns[outlet] = col;

            if verbose {
                progress = (100.0_f64 * outlet as f64 / pourpts.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Locating pour points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [i8; 129] = [0i8; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 0i8;
            pntr_matches[2] = 1i8;
            pntr_matches[4] = 2i8;
            pntr_matches[8] = 3i8;
            pntr_matches[16] = 4i8;
            pntr_matches[32] = 5i8;
            pntr_matches[64] = 6i8;
            pntr_matches[128] = 7i8;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 1i8;
            pntr_matches[2] = 2i8;
            pntr_matches[4] = 3i8;
            pntr_matches[8] = 4i8;
            pntr_matches[16] = 5i8;
            pntr_matches[32] = 6i8;
            pntr_matches[64] = 7i8;
            pntr_matches[128] = 0i8;
        }

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = pntr.get_value(row, col);
                if z != pntr_nodata {
                    if z > 0.0 {
                        flow_dir.set_value(row, col, pntr_matches[z as usize]);
                    } else {
                        flow_dir.set_value(row, col, -1i8);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // calculate the nesting order for each outlet point
        let mut flag: bool;
        let mut cur_order: usize;
        let (mut x, mut y): (isize, isize);
        let mut dir: i8;
        let mut max_nesting_order = 1;
        for record_num in 0..pourpts.num_records {
            outlet = record_num + 1;
            cur_order = 1;
            if nesting_order[outlet] < cur_order {
                nesting_order[outlet] = cur_order;
                flag = false;
                y = outlet_rows[outlet];
                x = outlet_columns[outlet];
                while !flag {
                    // find its downslope neighbour
                    dir = flow_dir.get_value(y, x);
                    if dir >= 0 {
                        // move x and y accordingly
                        x += dx[dir as usize];
                        y += dy[dir as usize];
                        if outlet_points.get_value(y, x) > 0 {
                            outlet = outlet_points.get_value(y, x) as usize;
                            cur_order += 1;
                            if nesting_order[outlet] < cur_order {
                                nesting_order[outlet] = cur_order;
                                if cur_order > max_nesting_order {
                                    max_nesting_order = cur_order;
                                }
                            } else {
                                flag = true;
                            }
                        }
                    } else {
                        flag = true;
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * outlet as f64 / pourpts.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Calculating outlet nesting order: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for order in 1..max_nesting_order + 1 {
            let start2 = Instant::now();
            // there will be an output file for each nesting order
            let pos_of_dot = output_file.rfind('.').unwrap_or(0);
            let ext = &output_file[pos_of_dot..];
            let output_file_order = output_file.replace(ext, &format!("_{}{}", order, ext));

            let mut output = Raster::initialize_using_file(&output_file_order, &pntr);
            output.configs.nodata = nodata;
            output.configs.data_type = DataType::I16;
            output.configs.photometric_interp = PhotometricInterpretation::Categorical;
            output.configs.palette = "qual.pal".to_string();
            let low_value = f64::MIN;
            output.reinitialize_values(low_value);

            for outlet in 1..pourpts.num_records + 1 {
                if nesting_order[outlet] == order {
                    y = outlet_rows[outlet];
                    x = outlet_columns[outlet];
                    output.set_value(y, x, outlet as f64);
                }
            }

            let mut outlet_id: f64;
            for row in 0..rows {
                for col in 0..columns {
                    if flow_dir.get_value(row, col) == -2 {
                        output.set_value(row, col, nodata);
                    }
                    if output.get_value(row, col) == low_value {
                        flag = false;
                        x = col;
                        y = row;
                        outlet_id = nodata;
                        while !flag {
                            // find its downslope neighbour
                            dir = flow_dir.get_value(y, x);
                            if dir >= 0 {
                                // move x and y accordingly
                                x += dx[dir as usize];
                                y += dy[dir as usize];

                                // if the new cell already has a value in the output, use that as the outletID
                                z = output.get_value(y, x);
                                if z != low_value {
                                    outlet_id = z;
                                    flag = true;
                                }
                            } else {
                                flag = true;
                            }
                        }

                        flag = false;
                        x = col;
                        y = row;
                        output.set_value(y, x, outlet_id);
                        while !flag {
                            // find its downslope neighbour
                            dir = flow_dir.get_value(y, x);
                            if dir >= 0 {
                                // move x and y accordingly
                                x += dx[dir as usize];
                                y += dy[dir as usize];

                                // if the new cell already has a value in the output, use that as the outletID
                                if output.get_value(y, x) != low_value {
                                    flag = true;
                                }
                            } else {
                                flag = true;
                            }
                            output.set_value(y, x, outlet_id);
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Progress (Loop {} of {}): {}%",
                            order, max_nesting_order, progress
                        );
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time2 = get_formatted_elapsed_time(start2);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("D8 pointer file: {}", d8_file));
            output.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time2));

            if verbose {
                println!("Saving data for nesting order {}...", order)
            };
            let _ = match output.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
