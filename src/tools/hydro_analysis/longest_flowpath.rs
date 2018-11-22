/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 29/10/2018
Last Modified: 29/10/2018
License: MIT
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
use structures::{Array2D, Point2D};
use tools::*;
use vector::ShapefileGeometry;
use vector::*;

/// This tool delineates the longest flowpaths for a group of subbasins or watersheds.
/// Flowpaths are initiated along drainage divides and continue along the D8-defined
/// flow direction until either the subbasin outlet or DEM edge is encountered. Each input
/// subbasin/watershed will have an associated vector flowpath in the output image. `LongestFlowpath`
/// is similar to the `r.lfp` plugin tool for GRASS GIS. The length of the longest flowpath
/// draining to an outlet is related to the time of concentration, which is a parameter
/// used in certain hydrological models.
///
/// The user must input the filename of a digital elevation model (DEM), a basins raster, and the
/// output vector. The DEM must be depressionless and should have been pre-processed using the
/// `BreachDepressions` or `FillDepressions` tool. The *basins raster* must contain features
/// that are delineated by categorical (integer valued) unique indentifier values. All non-NoData,
/// non-zero valued grid cells in the basins raster are interpreted as belonging to features.
/// In practice, this tool is usual run using either a single watershed, a group of contiguous
/// non-overlapping watersheds, or a series of nested subbasins. These are often derived using
/// the `Watershed` tool, based on a series of input outlets, or the `Subbasins` tool, based on
/// an input stream network. If subbasins are input to `LongestFlowpath`, each traced flowpath
/// will include only the non-overlapping portions within nested areas. Therefore, this can be a
/// convienent method of delineating the longest flowpath to each bifurcation in a stream network.
///
/// The output vector file will contain fields in the attribute table that identify the associated
/// basin unique identifier (*BASIN*), the elevation of the flowpath source point on the divide
/// (*UP_ELEV*), the elevation of the outlet point (*DN_ELEV*), the length of the flowpath (*LENGTH*),
/// and finally, the average slope (*AVG_SLOPE*) along the flowpath, measured as a percent grade.
///
/// # See Also
/// `MaximumUpslopeFlowpath`, `BreachDepressions`, `FillDepressions`, `Watershed`, `Subbasins`
pub struct LongestFlowpath {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl LongestFlowpath {
    pub fn new() -> LongestFlowpath {
        // public constructor
        let name = "LongestFlowpath".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Delineates the longest flowpaths for a group of subbasins or watersheds. ".to_string();

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
            name: "Basins File".to_owned(),
            flags: vec!["--basins".to_owned()],
            description: "Input raster basins file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=DEM.tif --basins=basins.tif -o=output.tif",
            short_exe, name
        ).replace("*", &sep);

        LongestFlowpath {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LongestFlowpath {
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
        let mut input_file = String::new();
        let mut basins_file = String::new();
        let mut output_file = String::new();

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
            if flag_val == "-i" || flag_val == "-dem" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-basin" || flag_val == "-basins" {
                basins_file = if keyval {
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
        if !basins_file.contains(&sep) && !basins_file.contains("/") {
            basins_file = format!("{}{}", working_directory, basins_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let basins = Arc::new(Raster::new(&basins_file, "r")?);

        if input.configs.rows != basins.configs.rows
            || input.configs.columns != basins.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input rasters must have the same spatial extent (i.e. number of rows and columns).",
            ));
        }

        // calculate the flow direction
        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resolution_x;
        let cell_size_y = input.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        let basin_nodata = basins.configs.nodata;

        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let basins = basins.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let grid_lengths = [
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                    diag_cell_size,
                    cell_size_x,
                    diag_cell_size,
                    cell_size_y,
                ];
                let (mut z, mut z_n): (f64, f64);
                let (mut max_slope, mut slope): (f64, f64);
                let mut dir: i8;
                let mut neighbouring_nodata: bool;
                let mut interior_pit_found = false;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata && basins.get_value(row, col) != basin_nodata {
                            dir = 0i8;
                            max_slope = f64::MIN;
                            neighbouring_nodata = false;
                            for i in 0..8 {
                                z_n = input.get_value(row + d_y[i], col + d_x[i]);
                                if z_n != nodata {
                                    slope = (z - z_n) / grid_lengths[i];
                                    if slope > max_slope && slope > 0f64 {
                                        max_slope = slope;
                                        dir = i as i8;
                                    }
                                } else {
                                    neighbouring_nodata = true;
                                }
                            }
                            if max_slope > 0f64 {
                                data[col as usize] = dir;
                            } else {
                                if !neighbouring_nodata {
                                    interior_pit_found = true;
                                }
                            }
                        }
                    }
                    tx.send((row, data, interior_pit_found)).unwrap();
                }
            });
        }

        let mut interior_pit_found = false;
        for r in 0..rows {
            let (row, data, pit) = rx.recv().unwrap();
            flow_dir.set_row_data(row, data);
            if pit {
                interior_pit_found = true;
            }
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Flow directions: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // calculate the number of inflowing cells
        let flow_dir = Arc::new(flow_dir);
        let mut num_inflowing: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let flow_dir = flow_dir.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
                let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
                let inflowing_vals: [i8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];
                let mut z: f64;
                let mut count: i8;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<i8> = vec![-1i8; columns as usize];
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            count = 0i8;
                            for i in 0..8 {
                                if flow_dir.get_value(row + d_y[i], col + d_x[i])
                                    == inflowing_vals[i]
                                {
                                    count += 1;
                                }
                            }
                            data[col as usize] = count;
                        } else {
                            data[col as usize] = -1i8;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut fp_source: Array2D<isize> = Array2D::new(rows, columns, num_cells, num_cells)?;
        let mut lfp: Array2D<f64> = Array2D::new(rows, columns, 0f64, nodata)?;
        let mut stack = Vec::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        for r in 0..rows {
            let (row, data) = rx.recv().unwrap();
            num_inflowing.set_row_data(row, data);
            for col in 0..columns {
                if num_inflowing[(row, col)] == 0i8 {
                    stack.push((row, col));
                    fp_source.set_value(row, col, row * columns + col);
                } else if num_inflowing[(row, col)] == -1i8 {
                    num_solved_cells += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Num. inflowing neighbours: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let d_x = [1, 1, 1, 0, -1, -1, -1, 0];
        let d_y = [-1, 0, 1, 1, 1, 0, -1, -1];
        let grid_lengths = [
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
            diag_cell_size,
            cell_size_x,
            diag_cell_size,
            cell_size_y,
        ];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        let mut dir: i8;
        let mut length: f64;
        let mut basin_val: f64;
        let mut basin_val_n: f64;
        let mut upstream_fp_source: isize;
        let mut list_of_basins = vec![];
        while !stack.is_empty() {
            let cell = stack.pop().unwrap();
            row = cell.0;
            col = cell.1;
            basin_val = basins.get_value(row, col);
            upstream_fp_source = fp_source.get_value(row, col);
            num_inflowing.decrement(row, col, 1i8);
            dir = flow_dir.get_value(row, col);
            if dir >= 0 {
                length = lfp.get_value(row, col) + grid_lengths[dir as usize];

                row_n = row + d_y[dir as usize];
                col_n = col + d_x[dir as usize];

                if length > lfp.get_value(row_n, col_n) {
                    lfp.set_value(row_n, col_n, length);
                    fp_source.set_value(row_n, col_n, upstream_fp_source);
                }

                basin_val_n = basins.get_value(row_n, col_n);
                if basin_val_n != basin_val {
                    list_of_basins.push(row * columns + col);
                }

                num_inflowing.decrement(row_n, col_n, 1i8);
                if num_inflowing.get_value(row_n, col_n) == 0i8 {
                    stack.push((row_n, col_n));
                }
            } else {
                list_of_basins.push(row * columns + col);
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Measuring flowpath length: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // create output file
        let mut output = Shapefile::new(&output_file, ShapeType::PolyLine)?;

        // set the projection information
        output.projection = input.configs.coordinate_ref_system_wkt.clone();

        // add the attributes
        output
            .attributes
            .add_field(&AttributeField::new("FID", FieldDataType::Int, 6u8, 0u8));

        output.attributes.add_field(&AttributeField::new(
            "BASIN",
            FieldDataType::Real,
            10u8,
            3u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "UP_ELEV",
            FieldDataType::Real,
            10u8,
            3u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "DN_ELEV",
            FieldDataType::Real,
            10u8,
            3u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "LENGTH",
            FieldDataType::Real,
            18u8,
            34u8,
        ));

        output.attributes.add_field(&AttributeField::new(
            "AVG_SLOPE",
            FieldDataType::Real,
            8u8,
            3u8,
        ));

        list_of_basins.reverse();
        let (mut x, mut y): (f64, f64);
        let mut prev_dir: i8;
        let mut first_cell_encountered: bool;
        let mut current_id = 1i32;
        let mut flag: bool;
        let mut already_added_point: bool;
        while !list_of_basins.is_empty() {
            let basin_cell = list_of_basins.pop().unwrap();
            let basin_row = basin_cell / columns;
            let basin_col = basin_cell - basin_row * columns;
            let basin_val = basins.get_value(basin_row, basin_col);
            let basin_z = input.get_value(basin_row, basin_col);
            let length = lfp.get_value(basin_row, basin_col);

            let source_cell = fp_source.get_value(basin_row, basin_col);
            row = source_cell / columns;
            col = source_cell - row * columns;
            let source_z = input.get_value(row, col);

            let slope = if length > 0f64 {
                100f64 * (source_z - basin_z) / length
            } else {
                0f64
            };

            let mut points = vec![];

            // descend the flowpath
            prev_dir = 99; // this way the first point in the line is always output.
            first_cell_encountered = false;
            flag = true;
            while flag {
                if input.get_value(row, col) != nodata {
                    dir = flow_dir.get_value(row, col);
                    already_added_point = if dir != prev_dir || !first_cell_encountered {
                        prev_dir = dir;
                        if basins.get_value(row, col) == basin_val {
                            x = input.get_x_from_column(col);
                            y = input.get_y_from_row(row);
                            points.push(Point2D::new(x, y));
                            first_cell_encountered = true;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if dir >= 0i8 {
                        if dir > 7 {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "An unexpected value has been identified in the pointer image. 
                            This tool requires a pointer grid that has been created using 
                            either the D8 or Rho8 tools.",
                            ));
                        }
                        row_n = row + d_y[dir as usize];
                        col_n = col + d_x[dir as usize];

                        if row_n == basin_row && col_n == basin_col {
                            x = input.get_x_from_column(col_n);
                            y = input.get_y_from_row(row_n);
                            points.push(Point2D::new(x, y));

                            dir = flow_dir.get_value(row_n, col_n);
                            if dir >= 0i8 {
                                row_n = row_n + d_y[dir as usize];
                                col_n = col_n + d_x[dir as usize];
                                if basins.get_value(row_n, col_n) != basin_nodata {
                                    x = input.get_x_from_column(col_n);
                                    y = input.get_y_from_row(row_n);
                                    points.push(Point2D::new(x, y));
                                }
                            }

                            flag = false;
                        }

                        row = row_n;
                        col = col_n;
                    } else {
                        if !already_added_point {
                            // this way the last point in the line is always output.
                            x = input.get_x_from_column(col);
                            y = input.get_y_from_row(row);
                            points.push(Point2D::new(x, y));
                        }
                        flag = false;
                    }
                } else {
                    flag = false;
                }
            }

            if points.len() > 1 {
                if points[points.len() - 1] == points[points.len() - 2] {
                    points.pop();
                }
                let mut sfg = ShapefileGeometry::new(ShapeType::PolyLine);
                sfg.add_part(&points);
                output.add_record(sfg);

                output.attributes.add_record(
                    vec![
                        FieldData::Int(current_id),
                        FieldData::Real(basin_val),
                        FieldData::Real(source_z),
                        FieldData::Real(basin_z),
                        FieldData::Real(length),
                        FieldData::Real(slope),
                    ],
                    false,
                );

                current_id += 1;
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Vectorizing flowpaths: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
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
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        if interior_pit_found {
            println!("**********************************************************************************");
            println!("WARNING: Interior pit cells were found within the input DEM. It is likely that the 
            DEM needs to be processed to remove topographic depressions and flats prior to
            running this tool.");
            println!("**********************************************************************************");
        }

        Ok(())
    }
}
