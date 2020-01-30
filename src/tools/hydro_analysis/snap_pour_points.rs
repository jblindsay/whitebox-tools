/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/072017
Last Modified: 12/10/2018
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use crate::vector::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// The `SnapPourPoints` tool can be used to move the location of vector pour points (i.e. outlets used in a `Watershed`
/// operation) (`--pour_pts`) to the location coincident with the highest flow accumulation (`--flow_accum`) value within
/// a specified maximum distance (`--snap_dist`). The pour points file (`--pour_pts`) must be a vector file of *Point* ShapeType.
///
/// If the output of the `SnapPourPoints` tool is to be used with the `Watershed` tool, the flow accumulation raster should
/// be generated using the `D8FlowAccumulation` algorithm. The snap distance (`--snap_dist`), measured in map units (e.g.
/// meters), must also be specified. This distance will serve as the search radius placed around each pour point during the
/// search for the maximum flow accumulation. In general, each outlet will be relocated the distance specified by the snap
/// distance.
///
/// Lindsay et al. (2008) provide a detailed discussion of the `SnapPourPoints` technique, and other more sophisticated
/// techniques for adjusting pour point locations used in watershedding operations including Jenson's snap pour points
/// (`JensonSnapPourPoints`) method. In most cases, the `JensonSnapPourPoints` tool should be prefered for applications of
/// repositioning outlet points used in watershedding operations onto the digital stream lines contained in local drainage
/// direction rasters. Jenson's method relocates outlet points to the *nearest* stream cell while `SnapPourPoints` relocated
/// outlets to the *largest* stream (designated by the largest flow accumulation value). In the common situation where outlet
/// cells are position near the confluence point of smaller tributary streams, the `SnapPourPoints` tool may re-position
/// outlets on the main-trunk stream, which will result in watershed delineation of incorrect sub-basins.
///
/// # Reference
/// Lindsay JB, Rothwell JJ, and Davies H. 2008. Mapping outlet points used for watershed delineation onto DEM-derived stream
/// networks, Water Resources Research, 44, W08442, doi:10.1029/2007WR006507.
///
/// # See Also:
/// `Watershed`, `JensonSnapPourPoints`, `D8FlowAccumulation`
pub struct SnapPourPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SnapPourPoints {
    pub fn new() -> SnapPourPoints {
        // public constructor
        let name = "SnapPourPoints".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Moves outlet points used to specify points of interest in a watershedding operation to the cell with the highest flow accumulation in its neighbourhood.".to_string();

        let mut parameters = vec![];
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
            name: "Input D8 Flow Accumulation File".to_owned(),
            flags: vec!["--flow_accum".to_owned()],
            description: "Input raster D8 flow accumulation file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Maximum Snap Distance (map units)".to_owned(),
            flags: vec!["--snap_dist".to_owned()],
            description: "Maximum snap distance in map units.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --pour_pts='pour_pts.shp' --flow_accum='d8accum.tif' -o='output.shp' --snap_dist=15.0", short_exe, name).replace("*", &sep);

        SnapPourPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SnapPourPoints {
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
        let mut pourpts_file = String::new();
        let mut flow_accum_file = String::new();
        let mut output_file = String::new();
        let mut snap_dist = 0.0;

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
            if flag_val == "-pour_pts" {
                pourpts_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-flow_accum" {
                flow_accum_file = if keyval {
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
            } else if flag_val == "-snap_dist" {
                snap_dist = if keyval {
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

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !pourpts_file.contains(&sep) && !pourpts_file.contains("/") {
            pourpts_file = format!("{}{}", working_directory, pourpts_file);
        }
        if !flow_accum_file.contains(&sep) && !flow_accum_file.contains("/") {
            flow_accum_file = format!("{}{}", working_directory, flow_accum_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        // let pourpts = Raster::new(&pourpts_file, "r")?;
        let pourpts = Shapefile::read(&pourpts_file)?;

        // make sure the input vector file is of points type
        if pourpts.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let flow_accum = Raster::new(&flow_accum_file, "r")?;

        let start = Instant::now();

        // let rows = flow_accum.configs.rows as isize;
        // let columns = flow_accum.configs.columns as isize;
        let nodata = flow_accum.configs.nodata;

        let mut output =
            Shapefile::initialize_using_file(&output_file, &pourpts, ShapeType::Point, true)?;

        let snap_dist_int: isize =
            ((snap_dist / flow_accum.configs.resolution_x) / 2.0).floor() as isize;

        let mut max_accum: f64;
        let mut zn: f64;
        let (mut row, mut col): (isize, isize);
        let (mut xn, mut yn): (isize, isize);
        let (mut x, mut y): (f64, f64);
        for record_num in 0..pourpts.num_records {
            let record = pourpts.get_record(record_num);
            let attr_rec = pourpts.attributes.get_record(record_num);
            output
                .attributes
                .add_record(attr_rec, pourpts.attributes.is_deleted[record_num]);
            row = flow_accum.get_row_from_y(record.points[0].y);
            col = flow_accum.get_column_from_x(record.points[0].x);
            max_accum = 0.0;
            xn = col;
            yn = row;
            for x in (col - snap_dist_int)..(col + snap_dist_int + 1) {
                for y in (row - snap_dist_int)..(row + snap_dist_int + 1) {
                    zn = flow_accum.get_value(y, x);
                    if zn > max_accum && zn != nodata {
                        max_accum = zn;
                        xn = x;
                        yn = y;
                    }
                }
            }
            x = flow_accum.get_x_from_column(xn);
            y = flow_accum.get_y_from_row(yn);
            output.add_point_record(x, y);
            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (pourpts.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // let flow_accum = Raster::new(&flow_accum_file, "r")?;

        // let start = time::now();

        // let rows = pourpts.configs.rows as isize;
        // let columns = pourpts.configs.columns as isize;
        // let nodata = pourpts.configs.nodata;
        // let fa_nodata = flow_accum.configs.nodata;

        // // make sure the input files have the same size
        // if pourpts.configs.rows != flow_accum.configs.rows
        //     || pourpts.configs.columns != flow_accum.configs.columns
        // {
        //     return Err(Error::new(
        //         ErrorKind::InvalidInput,
        //         "The input files must have the same number of rows and columns and spatial extent.",
        //     ));
        // }

        // let snap_dist_int: isize =
        //     ((snap_dist / pourpts.configs.resolution_x) / 2.0).floor() as isize;

        // let mut output = Raster::initialize_using_file(&output_file, &pourpts);

        // let mut outlet_id: f64;
        // let mut max_accum: f64;
        // let mut zn: f64;
        // let mut xn: isize;
        // let mut yn: isize;
        // for row in 0..rows {
        //     for col in 0..columns {
        //         outlet_id = pourpts[(row, col)];
        //         if outlet_id > 0.0 && outlet_id != nodata {
        //             max_accum = 0.0;
        //             xn = col;
        //             yn = row;
        //             for x in (col - snap_dist_int)..(col + snap_dist_int + 1) {
        //                 for y in (row - snap_dist_int)..(row + snap_dist_int + 1) {
        //                     zn = flow_accum[(y, x)];
        //                     if zn > max_accum && zn != fa_nodata {
        //                         max_accum = zn;
        //                         xn = x;
        //                         yn = y;
        //                     }
        //                 }
        //             }
        //             output[(yn, xn)] = outlet_id;
        //         }
        //     }
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Initializing: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }

        let elapsed_time = get_formatted_elapsed_time(start);
        // output.add_metadata_entry(format!(
        //     "Created by whitebox_tools\' {} tool",
        //     self.get_tool_name()
        // ));
        // output.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
        // output.add_metadata_entry(format!("D8 flow accumulation file: {}", flow_accum_file));
        // output.add_metadata_entry(format!("Snap distance: {}", snap_dist));
        // output.add_metadata_entry(
        //     format!("Elapsed Time (excluding I/O): {}", elapsed_time),
        // );

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
