/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay and Nigel Van Nieuwenhuizen
Created: 21/09/2020
Last Modified: 05/10/2020
License: MIT
*/

use whitebox_raster::Raster;
use whitebox_common::structures::{Array2D, BoundingBox, DistanceMetric, FixedRadiusSearch2D};
use crate::tools::*;
use whitebox_vector::{ShapeType, Shapefile};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to map and/or remove road embankments from an input fine-resolution digital elevation 
/// model (`--dem`). Fine-resolution LiDAR DEMs can represent surface features such as road and railway 
/// embankments with high fidelity. However, transportation embankments are problematic for several 
/// environmental modelling applications, including soil an vegetation distribution mapping, where the pre-embankment
/// topography is the contolling factor. The algorithm utilizes repositioned (`--search_dist`) transportation 
/// network cells, derived from rasterizing a transportation vector (`--road_vec`), as seed points in a 
/// region-growing operation. The embankment region grows based on derived morphometric parameters, including 
/// road surface width (`--min_road_width`), embankment width (`--typical_width` and `--max_width`), embankment 
/// height (`--max_height`), and absolute slope (`--spillout_slope`). The tool can be run in two modes. By default
/// the tool will simply map embankment cells, with a Boolean output raster. If, however, the `--remove_embankments`
/// flag is specified, the tool will instead output a DEM for which the mapped embankment grid cells have been
/// excluded and new surfaces have been interpolated based on the surrounding elevation values (see below).
///
/// Hillshade from original DEM:
/// ![](../../doc_img/EmbankmentMapping1.png)
/// 
/// Hillshade from embankment-removed DEM:
/// ![](../../doc_img/EmbankmentMapping2.png)
///
/// # References
/// Van Nieuwenhuizen, N, Lindsay, JB, DeVries, B. 2021. [Automated mapping of transportation embankments in 
/// fine-resolution LiDAR DEMs](https://www.mdpi.com/2072-4292/13/7/1308/htm). Remote Sensing. 13(7), 1308; https://doi.org/10.3390/rs13071308
/// 
/// # See Also:
/// `RemoveOffTerrainObjects`, `SmoothVegetationResidual`
pub struct EmbankmentMapping {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl EmbankmentMapping {
    pub fn new() -> EmbankmentMapping {
        // public constructor
        let name = "EmbankmentMapping".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Maps and/or removes road embankments from an input fine-resolution DEM.".to_string();

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
            name: "Input Vector Transportation Line File".to_owned(),
            flags: vec!["--road_vec".to_owned()],
            description: "Input vector polygons file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
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
            name: "Search Distance (in map units)".to_owned(),
            flags: vec!["--search_dist".to_owned()],
            description:
                "Search distance used to reposition transportation vectors onto road embankments (in map units)."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.5".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Road Width (in map units)".to_owned(),
            flags: vec!["--min_road_width".to_owned()],
            description:
                "Minimum road width; this is the width of the paved road surface (in map units)."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("6.0".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Typical Embankment Width (in map units)".to_owned(),
            flags: vec!["--typical_width".to_owned()],
            description:
                "Typical embankment width; this is the maximum width of an embankment with roadside ditches (in map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("30.0".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Typical Embankment Max Height (in map units)".to_owned(),
            flags: vec!["--max_height".to_owned()],
            description:
                "Typical embankment maximum height; this is the height a typical embankment with roadside ditches (in map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("2.0".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Embankment Max Width (in map units)".to_owned(),
            flags: vec!["--max_width".to_owned()],
            description:
                "Maximum embankment width, typically where embankments traverse steep-sided valleys (in map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("60.0".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Max Upwards Increment (in elevation units)".to_owned(),
            flags: vec!["--max_increment".to_owned()],
            description:
                "Maximum upwards increment between neighbouring cells on an embankment (in elevation units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.05".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Spillout Slope (in map units)".to_owned(),
            flags: vec!["--spillout_slope".to_owned()],
            description: "Spillout slope (in degrees).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("4.0".to_string()),
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Remove mapped embankments?".to_owned(),
            flags: vec!["--remove_embankments".to_owned()],
            description:
                "Optional flag indicating whether to output a DEM with embankments removed (true) or an embankment raster map (false)."
                    .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif --search_dist=1.0 --min_road_width=6.0 --typical_width=30.0 --max_height=2.0 --max_width=60.0 --max_increment=0.05 --spillout_slope=4.0 --remove_embankments=true",
            short_exe, name
        )
        .replace("*", &sep);

        EmbankmentMapping {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for EmbankmentMapping {
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
        let mut roads_file = String::new();
        let mut output_file = String::new();

        // parameters
        let mut search_dist = 2.5f64;
        let mut min_road_width = 6.0;
        let mut typical_width = 30.0;
        let mut max_height = 2.0;
        let mut max_width = 60.0;
        let mut max_increment = 0.05;
        let mut spillout_slope = 4.0;
        let mut remove_embankments = false;

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
            } else if flag_val == "-road_vec" {
                roads_file = if keyval {
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
            } else if flag_val == "-search_dist" {
                search_dist = if keyval {
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
            } else if flag_val == "-min_road_width" {
                min_road_width = if keyval {
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
            } else if flag_val == "-typical_width" {
                typical_width = if keyval {
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
            } else if flag_val == "-max_height" {
                max_height = if keyval {
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
            } else if flag_val == "-max_increment" {
                max_increment = if keyval {
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
            } else if flag_val == "-spillout_slope" {
                spillout_slope = if keyval {
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
            } else if flag_val == "-remove_embankments" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    remove_embankments = true;
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

        // Parameter quality control
        if min_road_width > typical_width {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "min_road_width must be less than the typical_width parameter.",
            ));
        }

        if typical_width > max_width {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "typical_width must be less than the max_width parameter.",
            ));
        }

        if max_increment < 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The min_increment parameter must be >= 0.0.",
            ));
        }

        if max_height < 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The max_height parameter must be >= 0.0.",
            ));
        }

        // The algorithm actually needs half-widths.
        min_road_width /= 2.0;
        typical_width /= 2.0;
        max_width /= 2.0;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading data...")
        };

        let dem = Raster::new(&input_file, "r").expect("Error reading input DEM.");
        let vector_data = Shapefile::read(&roads_file).expect("Error reading input Shapefile.");

        let start = Instant::now();

        ////////////////////////////////////////////////
        // Start off by rasterizing the roads vector. //
        ////////////////////////////////////////////////

        if verbose {
            println!("Rasterizing transportation vector...");
        }

        // make sure the input vector file is of polyline type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::PolyLine {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must either be of polyline base shape type.",
            ));
        }

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;

        let mut roads: Array2D<i8> =
            Array2D::new(rows, columns, 0i8, -1i8).expect("Error creating Array2D.");

        let raster_bb = BoundingBox::new(
            dem.configs.west,
            dem.configs.east,
            dem.configs.south,
            dem.configs.north,
        );
        let mut bb = BoundingBox {
            ..Default::default()
        };
        let (mut top_row, mut bottom_row, mut left_col, mut right_col): (
            isize,
            isize,
            isize,
            isize,
        );
        let mut row_y_coord: f64;
        let mut col_x_coord: f64;
        let (mut x1, mut x2, mut y1, mut y2): (f64, f64, f64, f64);
        let (mut x_prime, mut y_prime): (f64, f64);
        let mut start_point_in_part: usize;
        let mut end_point_in_part: usize;
        // let mut output_something = false;
        let num_records = vector_data.num_records;
        for record_num in 0..vector_data.num_records {
            let record = vector_data.get_record(record_num);
            let rec_bb = BoundingBox::new(record.x_min, record.x_max, record.y_min, record.y_max);
            if rec_bb.overlaps(raster_bb) {
                for part in 0..record.num_parts as usize {
                    start_point_in_part = record.parts[part] as usize;
                    if part < record.num_parts as usize - 1 {
                        end_point_in_part = record.parts[part + 1] as usize - 1;
                    } else {
                        end_point_in_part = record.num_points as usize - 1;
                    }

                    bb.initialize_to_inf();
                    for i in start_point_in_part..end_point_in_part + 1 {
                        if record.points[i].x < bb.min_x {
                            bb.min_x = record.points[i].x;
                        }
                        if record.points[i].x > bb.max_x {
                            bb.max_x = record.points[i].x;
                        }
                        if record.points[i].y < bb.min_y {
                            bb.min_y = record.points[i].y;
                        }
                        if record.points[i].y > bb.max_y {
                            bb.max_y = record.points[i].y;
                        }
                    }
                    top_row = dem.get_row_from_y(bb.max_y);
                    bottom_row = dem.get_row_from_y(bb.min_y);
                    left_col = dem.get_column_from_x(bb.min_x);
                    right_col = dem.get_column_from_x(bb.max_x);

                    if top_row < 0 {
                        top_row = 0;
                    }
                    if bottom_row < 0 {
                        bottom_row = 0;
                    }
                    if top_row >= rows {
                        top_row = rows - 1;
                    }
                    if bottom_row >= rows {
                        bottom_row = rows - 1;
                    }

                    if left_col < 0 {
                        left_col = 0;
                    }
                    if right_col < 0 {
                        right_col = 0;
                    }
                    if left_col >= columns {
                        left_col = columns - 1;
                    }
                    if right_col >= columns {
                        right_col = columns - 1;
                    }

                    // find each intersection with a row.
                    for row in top_row..bottom_row + 1 {
                        row_y_coord = dem.get_y_from_row(row);
                        // find the x-coordinates of each of the line segments
                        // that intersect this row's y coordinate
                        for i in start_point_in_part..end_point_in_part {
                            if is_between(row_y_coord, record.points[i].y, record.points[i + 1].y) {
                                y1 = record.points[i].y;
                                y2 = record.points[i + 1].y;
                                if y2 != y1 {
                                    x1 = record.points[i].x;
                                    x2 = record.points[i + 1].x;

                                    // calculate the intersection point
                                    x_prime = x1 + (row_y_coord - y1) / (y2 - y1) * (x2 - x1);
                                    let col = dem.get_column_from_x(x_prime);

                                    roads.set_value(row, col, 1i8);
                                    // output_something = true;
                                }
                            }
                        }
                    }

                    // find each intersection with a column.
                    for col in left_col..right_col + 1 {
                        col_x_coord = dem.get_x_from_column(col);
                        for i in start_point_in_part..end_point_in_part {
                            if is_between(col_x_coord, record.points[i].x, record.points[i + 1].x) {
                                x1 = record.points[i].x;
                                x2 = record.points[i + 1].x;
                                if x1 != x2 {
                                    y1 = record.points[i].y;
                                    y2 = record.points[i + 1].y;

                                    // calculate the intersection point
                                    y_prime = y1 + (col_x_coord - x1) / (x2 - x1) * (y2 - y1);

                                    let row = dem.get_row_from_y(y_prime);

                                    roads.set_value(row, col, 1i8);
                                    // output_something = true;
                                }
                            }
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * (record_num + 1) as f64 / num_records as f64) as usize;
                if progress != old_progress {
                    println!(
                        "Rasterizing roads {} of {}: {}%",
                        record_num + 1,
                        num_records,
                        progress
                    );
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!(
                "Time for transportation vector rasterization: {}",
                elapsed_time
            );
        }
        let start2 = Instant::now();

        // Now perform the analysis. //

        if verbose {
            println!("Mapping embankment cells...");
        }

        let nodata = dem.configs.nodata;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut output = Raster::initialize_using_file(&output_file, &dem);

        let mut zn: f64;
        let mut maxval: f64;
        let mut max_point: [isize; 2];
        let mut seed_search = search_dist / dem.configs.resolution_x;
        let mut dx_seed = vec![]; // the repositioning filter size
        let mut dy_seed = vec![]; // the repositioning filter size
        let diag_dist = dem.configs.resolution_x.hypot(dem.configs.resolution_y);
        let dist_array = [
            diag_dist,
            dem.configs.resolution_x,
            diag_dist,
            dem.configs.resolution_y,
            diag_dist,
            dem.configs.resolution_x,
            diag_dist,
            dem.configs.resolution_y,
        ];
        let mut pqueue = BinaryHeap::with_capacity((rows * columns * 2) as usize);
        let mut pqueue_dist = BinaryHeap::with_capacity((rows * columns * 2) as usize);
        let mut distance: Array2D<f64> =
            Array2D::new(rows, columns, -1f64, -1f64).expect("Error creating Array2D.");
        let mut seed_elev: Array2D<f64> =
            Array2D::new(rows, columns, -1f64, -1f64).expect("Error creating Array2D.");
        let mut max_abs_slope: Array2D<f64> =
            Array2D::new(rows, columns, 0f64, -1f64).expect("Error creating Array2D.");
        let mut cell: CellInfo;
        let (mut r, mut c): (isize, isize);
        let (mut seed_z, mut z): (f64, f64);
        let mut dist: f64;
        let mut embankment_height: f64;
        let mut embankment_slope: f64;
        let mut num_mapped_cells = 0f64;
        let num_cells = rows * columns;

        // ############################
        // # Seed cell re-positioning #
        // ############################
        // if verbose {
        //     println!("Re-positioning seed cells...");
        // }

        if seed_search as usize % 2 == 0 {
            seed_search += 1f64;
        }

        let midpoint = (seed_search / 2f64) as isize;

        for r in 0..seed_search as isize {
            for c in 0..seed_search as isize {
                dx_seed.push(c - midpoint);
                dy_seed.push(r - midpoint);
            }
        }

        for row in 0..rows {
            for col in 0..columns {
                if roads.get_value(row, col) > 0 {
                    maxval = dem.get_value(row, col);
                    max_point = [row, col];
                    for n in 0..dx_seed.len() {
                        zn = dem.get_value(row + dy_seed[n], col + dx_seed[n]);
                        if roads.get_value(row + dy_seed[n], col + dx_seed[n]) <= 0 {
                            if zn > maxval
                                && output.get_value(row + dy_seed[n], col + dx_seed[n]) != 1f64
                            {
                                // Notice this AND condition? It's important, otherwise you will lose many possible seeds.
                                maxval = zn;
                                max_point = [row + dy_seed[n], col + dx_seed[n]];
                                // set maxPoint as the cell of maximum elevation
                            }
                        }
                    }

                    output.set_value(max_point[0], max_point[1], 1f64);
                    pqueue_dist.push(CellInfo {
                        row: max_point[0],
                        col: max_point[1],
                        distance: 0.0f64,
                    });
                    distance.set_value(max_point[0], max_point[1], 0f64);
                    seed_elev.set_value(max_point[0], max_point[1], maxval);
                    max_abs_slope.set_value(max_point[0], max_point[1], 0f64);
                    pqueue.push(CellInfo {
                        row: max_point[0],
                        col: max_point[1],
                        distance: 0.0,
                    });
                }
            }
            progress = (100.0 * row as f64 / (rows - 1) as f64) as usize;
            if progress != old_progress && verbose {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }

        // println!("Calculating embankment parameters...");
        let mut total_cells_in_buffer = 0.0;
        while pqueue_dist.len() != 0 {
            cell = pqueue_dist.pop().expect("Error during pop operation.");
            r = cell.row;
            c = cell.col;
            seed_z = seed_elev.get_value(r, c);
            // z = dem.get_value(r, c);

            for n in 0..8 {
                zn = dem.get_value(r + dy[n], c + dx[n]);
                if zn != nodata && distance.get_value(r + dy[n], c + dx[n]) < 0.0 {
                    dist = cell.distance + dist_array[n];
                    if dist < max_width {
                        distance.set_value(r + dy[n], c + dx[n], dist);
                        seed_elev.set_value(r + dy[n], c + dx[n], seed_z);
                        embankment_height = seed_z - zn;
                        embankment_slope =
                            (embankment_height / (cell.distance + dist_array[n])).atan();
                        max_abs_slope.set_value(
                            r + dy[n],
                            c + dx[n],
                            embankment_slope
                                .abs()
                                .to_degrees()
                                .max(max_abs_slope.get_value(r, c)),
                        );
                        pqueue_dist.push(CellInfo {
                            row: r + dy[n],
                            col: c + dx[n],
                            distance: dist,
                        });
                        total_cells_in_buffer += 1.0;
                    }
                }
            }

            if verbose {
                num_mapped_cells += 1f64;
                progress = (100.0 * num_mapped_cells / (num_cells as f64)) as usize;
                if progress != old_progress {
                    println!("Percent of total cells parameterized: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        num_mapped_cells = 0.0;
        while pqueue.len() != 0 {
            cell = pqueue.pop().expect("Error during pop operation.");
            r = cell.row;
            c = cell.col;
            // seed_z = seed_elev.get_value(r, c);
            z = dem.get_value(r, c);

            for n in 0..8 {
                zn = dem.get_value(r + dy[n], c + dx[n]);
                if zn != nodata && output.get_value(r + dy[n], c + dx[n]) == nodata {
                    dist = distance.get_value(r + dy[n], c + dx[n]);
                    if dist >= 0.0 {
                        if dist <= min_road_width {
                            // If we're within this narrow distance of a seed cell, it's road regardless of other factors.
                            output.set_value(r + dy[n], c + dx[n], 1.0); // 2.0);
                            pqueue.push(CellInfo {
                                row: r + dy[n],
                                col: c + dx[n],
                                distance: dist,
                            });
                        } else if dist <= max_width {
                            seed_z = seed_elev.get_value(r + dy[n], c + dx[n]);
                            embankment_height = seed_z - zn;
                            embankment_slope = (embankment_height / dist).atan().to_degrees();
                            if dist <= typical_width
                                && z - zn > -max_increment
                                && embankment_height <= max_height
                            {
                                if zn <= z {
                                    output.set_value(r + dy[n], c + dx[n], 1.0); // 3.0);
                                    pqueue.push(CellInfo {
                                        row: r + dy[n],
                                        col: c + dx[n],
                                        distance: dist,
                                    });
                                } else if zn > z
                                    && max_abs_slope.get_value(r + dy[n], c + dx[n])
                                        < spillout_slope
                                {
                                    output.set_value(r + dy[n], c + dx[n], 1.0); // 4.0);
                                    pqueue.push(CellInfo {
                                        row: r + dy[n],
                                        col: c + dx[n],
                                        distance: dist,
                                    });
                                }
                            } else if max_abs_slope.get_value(r + dy[n], c + dx[n])
                                - embankment_slope.abs()
                                <= 1.0
                                && embankment_slope >= 0.0
                            {
                                // there isn't a major break-in-slope between the cell and the source cell and it's downward.
                                output.set_value(r + dy[n], c + dx[n], 1.0); // 5.0);
                                pqueue.push(CellInfo {
                                    row: r + dy[n],
                                    col: c + dx[n],
                                    distance: dist,
                                });
                            }
                        }
                    }
                }
            }

            num_mapped_cells += 1f64;
            progress = (100.0 * num_mapped_cells / total_cells_in_buffer) as usize;
            if progress != old_progress && verbose {
                println!("Progress: {}%", progress);
                old_progress = progress;
            }
        }

        if verbose {
            let elapsed_time = get_formatted_elapsed_time(start2);
            println!("Time for embankment mapping: {}", elapsed_time);
        }
        let start3 = Instant::now();

        let mut total_time = get_formatted_elapsed_time(start);

        if remove_embankments {
            if verbose {
                println!("Creating embankment-less DEM...");
            }
            // Not let's interpolate a surface beneath the road embankments.
            let mut frs: FixedRadiusSearch2D<f64> =
                FixedRadiusSearch2D::new(max_width, DistanceMetric::SquaredEuclidean);

            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let (mut col_n, mut row_n): (isize, isize);
            let (mut x, mut y): (f64, f64);
            for row in 0..rows {
                for col in 0..columns {
                    if output.get_value(row, col) >= 1f64 {
                        for i in 0..8 {
                            row_n = row + dy[i];
                            col_n = col + dx[i];
                            if (output.get_value(row_n, col_n) == nodata
                                || output.get_value(row_n, col_n) == 0f64)
                                && dem.get_value(row_n, col_n) != nodata
                            {
                                y = dem.get_y_from_row(row_n);
                                x = dem.get_x_from_column(col_n);
                                frs.insert(x, y, dem.get_value(row_n, col_n));
                            }
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                    if progress != old_progress {
                        println!("Finding edge cells: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let file_ext = output.get_file_extension();
            let out_dem_str =
                output_file.replace(&format!(".{}", file_ext), &format!("_dem.{}", file_ext));
            let mut out_dem = Raster::initialize_using_file(&out_dem_str, &dem);

            let mut sum_weights: f64;
            let mut dist: f64;

            for row in 0..rows {
                y = dem.get_y_from_row(row);
                for col in 0..columns {
                    if output.get_value(row, col) >= 1f64 {
                        x = dem.get_x_from_column(col);
                        sum_weights = 0f64;
                        let ret = frs.search(x as f64, y as f64);
                        for j in 0..ret.len() {
                            dist = ret[j].1;
                            if dist > 0.0 {
                                sum_weights += 1.0 / dist;
                            }
                        }
                        z = 0.0;
                        for j in 0..ret.len() {
                            dist = ret[j].1;
                            if dist > 0.0 {
                                z += ret[j].0 * (1.0 / dist) / sum_weights;
                            }
                        }
                        if ret.len() > 0 {
                            out_dem.set_value(row, col, z);
                        } else {
                            out_dem.set_value(row, col, nodata);
                        }
                    } else {
                        out_dem.set_value(row, col, dem.get_value(row, col));
                    }
                }
                if verbose {
                    progress = (100.0_f32 * row as f32 / (rows - 1) as f32) as usize;
                    if progress != old_progress {
                        println!("Interpolating data holes: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            if verbose {
                let elapsed_time = get_formatted_elapsed_time(start3);
                println!("Time to create embankment-less DEM: {}", elapsed_time);
            }

            total_time = get_formatted_elapsed_time(start);
            out_dem.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            out_dem.add_metadata_entry(format!("Input file: {}", input_file));
            out_dem.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", total_time));

            if verbose {
                println!("Saving data...")
            };
            let _ = match out_dem.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };
        }

        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", total_time));

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
                &format!("Elapsed Time (excluding I/O): {}", total_time)
            );
        }

        Ok(())
    }
}

#[derive(PartialEq, Debug)]
struct CellInfo {
    row: isize,
    col: isize,
    distance: f64,
}

impl Eq for CellInfo {}

impl PartialOrd for CellInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for CellInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn is_between(val: f64, threshold1: f64, threshold2: f64) -> bool {
    if val == threshold1 || val == threshold2 {
        return true;
    }
    if threshold2 > threshold1 {
        return val > threshold1 && val < threshold2;
    }
    val > threshold2 && val < threshold1
}
