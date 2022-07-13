/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/07/2017
Last Modified: 30/01/2020
License: MIT
*/

use whitebox_raster::*;
use crate::tools::*;
use num_cpus;
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool can be used to extract channel networks from an input digital elevation models (`--dem`) using
/// one of three techniques that are based on local topography alone.
///
/// The Lindsay (2006) 'lower-quartile' method (`--variant='LQ'`) algorithm is a type of 'valley recognition'
/// method. Other channel mapping methods, such as the Johnston and Rosenfeld (1975) algorithm, experience
/// problems because channel profiles are not always 'v'-shaped, nor are they always apparent in small
/// 3 x 3 windows. The lower-quartile method was developed as an alternative and more flexible valley
/// recognition channel mapping technique. The lower-quartile method operates by running a filter over the
/// DEM that calculates the percentile value of the centre cell with respect to the distribution of
/// elevations within the filter window. The roving window is circular, the diameter of which should reflect
/// the topographic variation of the area (e.g. the channel width or average hillslope length). If this variant
/// is selected, the user must specify the filter size (`--filter`), in pixels, and this value should be an odd
/// number (e.g. 3, 5, 7, etc.). The appropriateness of the selected window diameter will depend on the grid
/// resolution relative to the scale of topographic features. Cells that are within the lower quartile of the
/// distribution of elevations of their neighbourhood are flagged. Thus, the algorithm identifies grid cells
/// that are in relatively low topographic positions at a local scale. This approach to channel mapping is only
/// appropriate in fluvial landscapes. In regions containing numerous lakes and wetlands, the algorithm will
/// pick out the edges of features.
///
/// The Johnston and Rosenfeld (1975) algorithm (`--variant='JandR'`) is a type of 'valley recognition' method
/// and operates as follows: channel cells are flagged in a 3 x 3 window if the north and south neighbours are
/// higher than the centre grid cell or if the east and west neighbours meet this same criterion. The group of
/// cells that are flagged after one pass of the roving window constituted the drainage network. This method is
/// best applied to DEMs that are relatively smooth and do not exhibit high levels of short-range roughness. As
/// such, it may be desirable to use a smoothing filter before applying this tool. The `FeaturePreservingSmoothing`
/// is a good option for removing DEM roughness while preserving the topographic information contain in
/// breaks-in-slope (i.e. edges).
///
/// The Peucker and Douglas (1975) algorithm (`--variant='PandD'`) is one of the simplest and earliest algorithms
/// for topography-based network extraction. Their 'valley recognition' method operates by passing a 2 x 2 roving
/// window over a DEM and flagging the highest grid cell in each group of four. Once the window has passed over
/// the entire DEM, channel grid cells are left unflagged. This method is also best applied to DEMs that are relatively
/// smooth and do not exhibit high levels of short-range roughness. Pre-processing the DEM with the `FeaturePreservingSmoothing`
/// tool may also be useful when applying this method.
///
/// Each of these methods of extracting valley networks result in line networks that can be wider than a single
/// grid cell. As such, it is often desirable to thin the resulting network using a line-thinning algorithm.
/// The option to perform line-thinning is provided by the tool as a post-processing step (`--line_thin`).
///
/// # References
///
/// Johnston, E. G., & Rosenfeld, A. (1975). Digital detection of pits, peaks, ridges, and ravines. IEEE
/// Transactions on Systems, Man, and Cybernetics, (4), 472-480.
///
/// Lindsay, J. B. (2006). Sensitivity of channel mapping techniques to uncertainty in digital elevation data.
/// International Journal of Geographical Information Science, 20(6), 669-692.
///
/// Peucker, T. K., & Douglas, D. H. (1975). Detection of surface-specific points by local parallel
/// processing of discrete terrain elevation data. Computer Graphics and image processing, 4(4), 375-387.
///
/// # See Also
/// `FeaturePreservingSmoothing`
pub struct ExtractValleys {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ExtractValleys {
    pub fn new() -> ExtractValleys {
        // public constructor
        let name = "ExtractValleys".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Identifies potential valley bottom grid cells based on local topolography alone."
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

        parameters.push(ToolParameter{
            name: "Variant".to_owned(), 
            flags: vec!["--variant".to_owned()], 
            description: "Options include 'LQ' (lower quartile), 'JandR' (Johnston and Rosenfeld), and 'PandD' (Peucker and Douglas); default is 'LQ'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["LQ".to_owned(), "JandR".to_owned(), "PandD".to_owned()]),
            default_value: Some("LQ".to_owned()),
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Perform line-thinning?".to_owned(), 
            flags: vec!["--line_thin".to_owned()], 
            description: "Optional flag indicating whether post-processing line-thinning should be performed.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("true".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Filter Size (Only For Lower Quartile)".to_owned(), 
            flags: vec!["--filter".to_owned()], 
            description: "Optional argument (only used when variant='lq') providing the filter size, in grid cells, used for lq-filtering (default is 5).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("5".to_owned()),
            optional: true
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=pointer.tif -o=out.tif --variant='JandR' --line_thin
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=pointer.tif -o=out.tif --variant='lq' --filter=7 --line_thin", short_exe, name).replace("*", &sep);

        ExtractValleys {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ExtractValleys {
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
        let mut variant = String::from("lq");
        let mut line_thin = false;
        let mut filter_size = 5;

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
            if flag_val == "-dem" {
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
            } else if flag_val == "-variant" {
                if keyval {
                    variant = vec[1].to_string();
                } else {
                    variant = args[i + 1].to_string();
                }
                if variant.to_lowercase().contains("q") {
                    variant = String::from("lq");
                } else if variant.to_lowercase().contains("j") {
                    variant = String::from("JandR");
                } else {
                    //if variant.to_lowercase().contains("p") {
                    variant = String::from("PandD");
                }
            } else if flag_val == "-line_thin" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    line_thin = true;
                }
            } else if flag_val == "-filter" {
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

                //the filter dimensions must be odd numbers such that there is a middle pixel
                if (filter_size as f64 / 2f64).floor() == filter_size as f64 / 2f64 {
                    println!("WARNING: Filter dimensions must be odd numbers. The specified filter dimension has been modified.");
                    filter_size += 1;
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();
        let mut progress: i32;
        let mut old_progress: i32 = -1;

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut output = Raster::initialize_using_file(&output_file, &input);

        match &variant as &str {
            "lq" => {
                output.reinitialize_values(0f64);

                // This one can be performed conccurently.
                let mut num_procs = num_cpus::get() as isize;
                let configs = whitebox_common::configs::get_configs()?;
                let max_procs = configs.max_procs;
                if max_procs > 0 && max_procs < num_procs {
                    num_procs = max_procs;
                }
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let num_cells_in_filter = filter_size * filter_size;
                        let mut dx = vec![0isize; num_cells_in_filter];
                        let mut dy = vec![0isize; num_cells_in_filter];
                        let midpoint = (filter_size as f64 / 2f64).floor() as isize;
                        let mut z: f64;
                        let mut zn: f64;
                        let large_value = f64::INFINITY;
                        let mut n: f64;
                        let mut lower_quartile: usize;

                        let mut filter_shape = vec![true; num_cells_in_filter];
                        let sqr_radius = (midpoint * midpoint) as f64;
                        let mut i = 0;
                        for row in 0..filter_size as isize {
                            for col in 0..filter_size as isize {
                                dx[i] = col - midpoint;
                                dy[i] = row - midpoint;
                                z = (dx[i] * dx[i]) as f64 as f64
                                    + (dy[i] * dy[i]) as f64 as f64;
                                if z > sqr_radius {
                                    filter_shape[i] = false;
                                }
                                i += 1;
                            }
                        }

                        let mut cell_data = vec![0f64; num_cells_in_filter];
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                z = input.get_value(row, col);
                                if z != nodata {
                                    n = 0f64;
                                    // let mut cell_data = Vec::with_capacity(num_cells_in_filter);
                                    for i in 0..num_cells_in_filter {
                                        if filter_shape[i] {
                                            zn = input.get_value(row + dy[i], col + dx[i]);
                                            if zn != nodata {
                                                // cell_data.push(zn);
                                                cell_data[i] = zn;
                                                n += 1f64;
                                            } else {
                                                cell_data[i] = large_value;
                                            }
                                        }
                                    }
                                    // n = cell_data.len() as f64;
                                    if n > 0f64 {
                                        // sort the array
                                        cell_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
                                        lower_quartile = (n / 4f64).floor() as usize;
                                        if z <= cell_data[lower_quartile] {
                                            data[col as usize] = 1f64;
                                        } else {
                                            data[col as usize] = 0f64;
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
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            "JandR" => {
                // This one can be performed conccurently.
                // output.reinitialize_values(0f64);
                let mut num_procs = num_cpus::get() as isize;
                let configs = whitebox_common::configs::get_configs()?;
                let max_procs = configs.max_procs;
                if max_procs > 0 && max_procs < num_procs {
                    num_procs = max_procs;
                }
                let (tx, rx) = mpsc::channel();
                for tid in 0..num_procs {
                    let input = input.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let (mut z, mut zn1, mut zn2): (f64, f64, f64);
                        let dx = [0, 0, -1, 1];
                        let dy = [-1, 1, 0, 0];
                        for row in (0..rows).filter(|r| r % num_procs == tid) {
                            let mut data = vec![nodata; columns as usize];
                            for col in 0..columns {
                                z = input[(row, col)];
                                if z != nodata {
                                    zn1 = input[(row + dy[0], col + dx[0])];
                                    zn2 = input[(row + dy[1], col + dx[1])];
                                    if zn1 != nodata && zn2 != nodata && zn1 > z && zn2 > z {
                                        data[col as usize] = 1f64;
                                    } else {
                                        zn1 = input[(row + dy[2], col + dx[2])];
                                        zn2 = input[(row + dy[3], col + dx[3])];
                                        if zn1 != nodata && zn2 != nodata && zn1 > z && zn2 > z {
                                            data[col as usize] = 1f64;
                                        } else {
                                            data[col as usize] = 0f64;
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
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
            _ => {
                // "PandD"
                // This one can't easily be performed conccurently because a cell can be
                // modified while a row other than the row containing the cell is being scanned.
                output.reinitialize_values(1f64);
                let mut z: f64;
                let mut maxz: f64;
                let mut which_cell: usize;
                let dx = [-1, 0, -1, 0];
                let dy = [-1, -1, 0, 0];
                let num_scan_cells = dx.len();
                for row in 0..rows {
                    for col in 0..columns {
                        z = input[(row, col)];
                        if z != nodata {
                            maxz = z;
                            which_cell = 3;
                            for n in 0..num_scan_cells {
                                z = input[(row + dy[n], col + dx[n])];
                                if z != nodata {
                                    if z > maxz {
                                        maxz = z;
                                        which_cell = n;
                                    }
                                }
                            }
                            output.set_value(row + dy[which_cell], col + dx[which_cell], 0f64);
                        } else {
                            output[(row, col)] = nodata;
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                        if progress != old_progress {
                            println!("Progress: {}%", progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        if line_thin {
            println!("Line thinning operation...");
            let mut did_something = true;
            let mut loop_num = 0;
            let dx = [1, 1, 1, 0, -1, -1, -1, 0];
            let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
            let elements = vec![
                vec![6, 7, 0, 4, 3, 2],
                vec![7, 0, 1, 3, 5],
                vec![0, 1, 2, 4, 5, 6],
                vec![1, 2, 3, 5, 7],
                vec![2, 3, 4, 6, 7, 0],
                vec![3, 4, 5, 7, 1],
                vec![4, 5, 6, 0, 1, 2],
                vec![5, 6, 7, 1, 3],
            ];

            let vals = vec![
                vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64, 1f64],
                vec![0f64, 0f64, 0f64, 1f64, 1f64],
            ];

            let mut neighbours = [0.0; 8];
            let mut pattern_match: bool;
            let mut z: f64;
            while did_something {
                loop_num += 1;
                did_something = false;
                for a in 0..8 {
                    for row in 0..rows {
                        for col in 0..columns {
                            z = output[(row, col)];
                            if z > 0.0 && z != nodata {
                                // fill the neighbours array
                                for i in 0..8 {
                                    neighbours[i] = output[(row + dy[i], col + dx[i])];
                                }

                                // scan through element
                                pattern_match = true;
                                for i in 0..elements[a].len() {
                                    if neighbours[elements[a][i]] != vals[a][i] {
                                        pattern_match = false;
                                    }
                                }
                                if pattern_match {
                                    output[(row, col)] = 0.0;
                                    did_something = true;
                                }
                            }
                        }
                    }
                    if verbose {
                        progress = (100.0_f64 * a as f64 / 7.0) as i32;
                        if progress != old_progress {
                            println!("Loop Number {}: {}%", loop_num, progress);
                            old_progress = progress;
                        }
                    }
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input DEM file: {}", input_file));
        output.add_metadata_entry(format!("Variant: {}", variant));
        if variant == String::from("lq") {
            output.add_metadata_entry(format!("Filter size: {}", filter_size));
        }
        output.add_metadata_entry(format!("Line thinning: {}", line_thin));
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
