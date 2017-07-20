/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: July 10, 2017
Last Modified: July 17, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use lidar::*;
// use lidar::point_data::*;
use raster::*;
use structures::FixedRadiusSearch2D;
use tools::WhiteboxTool;

pub struct LidarPointDensity {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl LidarPointDensity {
    pub fn new() -> LidarPointDensity {
        // public constructor
        let name = "LidarPointDensity".to_string();

        let description = "Calculates the spatial pattern of point density for a LiDAR data set."
            .to_string();

        //let mut parameters = "-i, --input    Optional input LAS file; if excluded, all LAS files in working directory will be processed.\n".to_owned();
        let mut parameters = "-i, --input    Input LAS file (including extension).\n".to_owned();
        parameters.push_str("-o, --output   Output raster file (including extension).\n");
        parameters.push_str("--returns      Point return types to include; options are 'all' (default), 'last', 'first'.\n");
        parameters.push_str("--resolution   Output raster's grid resolution.\n");
        parameters.push_str("--radius       Search radius; default is 2.5.\n");
        parameters.push_str("--exclude_cls  Optional exclude classes from interpolation; Valid class values range from 0 to 18, based on LAS specifications. Example, --exclude_cls='3,4,5,6,7,18'");
        parameters.push_str("--palette      Optional palette name (for use with Whitebox raster files).\n");
        parameters.push_str("--minz         Optional minimum elevation for inclusion in interpolation.\n");
        parameters.push_str("--maxz         Optional maximum elevation for inclusion in interpolation.\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.dep --resolution=2.0 --radius=5.0\"
.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=file.las -o=outfile.dep --resolution=5.0 --radius=2.0 --exclude_cls='3,4,5,6,7,18' --palette=light_quant.plt", short_exe, name).replace("*", &sep);

        LidarPointDensity {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for LidarPointDensity {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut return_type = "all".to_string();
        let mut grid_res: f64 = 1.0;
        let mut search_radius = 2.5;
        let mut include_class_vals = vec![true; 256];
        let mut palette = "default".to_string();
        let mut exclude_cls_str = String::new();
        let mut max_z = f64::INFINITY;
        let mut min_z = f64::NEG_INFINITY;

        // read the arguments
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-returns" || vec[0].to_lowercase() == "--returns" {
                if keyval {
                    return_type = vec[1].to_string();
                } else {
                    return_type = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-resolution" ||
                      vec[0].to_lowercase() == "--resolution" {
                if keyval {
                    grid_res = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    grid_res = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-radius" || vec[0].to_lowercase() == "--radius" {
                if keyval {
                    search_radius = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    search_radius = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-palette" || vec[0].to_lowercase() == "--palette" {
                if keyval {
                    palette = vec[1].to_string();
                } else {
                    palette = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-exclude_cls" ||
                      vec[0].to_lowercase() == "--exclude_cls" {
                if keyval {
                    exclude_cls_str = vec[1].to_string();
                } else {
                    exclude_cls_str = args[i + 1].to_string();
                }
                let mut cmd = exclude_cls_str.split(",");
                let mut vec = cmd.collect::<Vec<&str>>();
                if vec.len() == 1 {
                    cmd = exclude_cls_str.split(";");
                    vec = cmd.collect::<Vec<&str>>();
                }
                for value in vec {
                    if !value.trim().is_empty() {
                        let c = value.trim().parse::<usize>().unwrap();
                        include_class_vals[c] = false;
                    }
                }
            } else if vec[0].to_lowercase() == "-minz" || vec[0].to_lowercase() == "--minz" {
                if keyval {
                    min_z = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    min_z = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            } else if vec[0].to_lowercase() == "-maxz" || vec[0].to_lowercase() == "--maxz" {
                if keyval {
                    max_z = vec[1].to_string().parse::<f64>().unwrap();
                } else {
                    max_z = args[i + 1].to_string().parse::<f64>().unwrap();
                }
            }
        }

        let (all_returns, late_returns, early_returns): (bool, bool, bool);
        if return_type.contains("last") {
            all_returns = false;
            late_returns = true;
            early_returns = false;
        } else if return_type.contains("first") {
            all_returns = false;
            late_returns = false;
            early_returns = true;
        } else {
            // all
            all_returns = true;
            late_returns = false;
            early_returns = false;
        }

        if !input_file.contains(path::MAIN_SEPARATOR) {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(path::MAIN_SEPARATOR) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if verbose {
            println!("Reading input LAS file...");
        }
        let input = match LasFile::new(&input_file, "r") {
            Ok(lf) => lf,
            Err(err) => panic!("Error reading file {}: {}", input_file, err),
        };

        let start = time::now();

        if verbose {
            println!("Performing analysis...");
        }

        let n_points = input.header.number_of_points as usize;
        let num_points: f64 = (input.header.number_of_points - 1) as f64; // used for progress calculation only

        let mut progress: i32;
        let mut old_progress: i32 = -1;
        let mut frs: FixedRadiusSearch2D<usize> = FixedRadiusSearch2D::new(search_radius);
        for i in 0..n_points {
            let p: PointData = input[i];
            if !p.class_bit_field.withheld() {
                if all_returns || (p.is_late_return() & late_returns) ||
                   (p.is_early_return() & early_returns) {
                    if include_class_vals[p.classification() as usize] {
                        if p.z >= min_z && p.z <= max_z {
                            frs.insert(p.x, p.y, i);
                        }
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * i as f64 / num_points) as i32;
                if progress != old_progress {
                    println!("Binning points: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        let west: f64 = input.header.min_x;
        let north: f64 = input.header.max_y;
        let rows: isize = (((north - input.header.min_y) / grid_res).ceil()) as isize;
        let columns: isize = (((input.header.max_x - west) / grid_res).ceil()) as isize;
        let south: f64 = north - rows as f64 * grid_res;
        let east = west + columns as f64 * grid_res;
        let nodata = -32768.0f64;

        let mut configs = RasterConfigs { ..Default::default() };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = grid_res;
        configs.resolution_y = grid_res;
        configs.nodata = nodata;
        configs.data_type = DataType::F64;
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        configs.palette = palette;

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        let frs = Arc::new(frs); // wrap FRS in an Arc
        let search_area = f64::consts::PI * search_radius * search_radius;
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let frs = frs.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let (mut x, mut y): (f64, f64);
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![nodata; columns as usize];
                    for col in 0..columns {
                        x = west + col as f64 * grid_res + 0.5;
                        y = north - row as f64 * grid_res - 0.5;
                        let ret = frs.search(x, y);
                        if ret.len() > 0 {
                            data[col as usize] = ret.len() as f64 / search_area;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().unwrap();
            output.set_row_data(data.0, data.1);
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as i32;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Grid resolution: {}", grid_res));
        output.add_metadata_entry(format!("Search radius: {}", search_radius));
        output.add_metadata_entry(format!("Returns: {}", return_type));
        output.add_metadata_entry(format!("Excluded classes: {}", exclude_cls_str));
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

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
