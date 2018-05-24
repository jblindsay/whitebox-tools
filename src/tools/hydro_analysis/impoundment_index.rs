use time;
use num_cpus;
use rand;
use std::env;
use std::path;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::f64;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use raster::*;
use tools::*;
use self::rand::distributions::{Normal, IndependentSample};
use structures::Array2D;
use std::f64::consts::PI;

/// Preforms a stochastic analysis of depressions within a DEM.
pub struct ImpoundmentIndex {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ImpoundmentIndex {
    pub fn new() -> ImpoundmentIndex {
        // public constructor
        let name = "ImpoundmentIndex".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Calculates the impoundment size resulting from damming a DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input DEM File".to_owned(), 
            flags: vec!["-i".to_owned(), "--dem".to_owned()], 
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output Type".to_owned(), 
            flags: vec!["--out_type".to_owned()], 
            description: "Output type; one of 'area' (default) and 'volume'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec!["area".to_owned(), "volume".to_owned()]),
            default_value: Some("area".to_owned()),
            optional: true
        });

        parameters.push(ToolParameter{
            name: "Max dam height (z-units)".to_owned(), 
            flags: vec!["--damheight".to_owned()], 
            description: "Maximum height of the dam".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Max dam length (grid cells)".to_owned(), 
            flags: vec!["--damlength".to_owned()], 
            description: "Maximum length of thr dam.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
        });


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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=DEM.tif -o=out.tif --out_type=area --damheight --damlength", short_exe, name).replace("*", &sep);

        ImpoundmentIndex {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ImpoundmentIndex {
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

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut out_type = String::from("area");
        let mut dheight: f64;
        let mut dlength: f64;
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters."));
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
            } else if flag_val == "-out_type" {
                out_type = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i+1].to_lowercase()
                };
                if out_type.contains("v") {
                    out_type = String::from("volume");
                } else {
                    out_type = String::from("area");
                }
            } else if flag_val == "-damheight" {
                dheight = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i+1].to_string().parse::<f64>().unwrap()
                };
            } else if flag_val == "-damlength" {
                dlength = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i+1].to_string().parse::<f32>().unwrap() as usize
                };
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }


        let sep: String = path::MAIN_SEPARATOR.to_string();

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Arc::new(Raster::new(&input_file, "r")?);
        
        let start = time::now();
        let mut mode: f64
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let num_cells = rows * columns;
        let nodata = input.configs.nodata;
        let cell_size_x = input.configs.resoluion_x;
        let cell_size_y = input.configs.resolution_y;
        
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -1, -1)?;

        /*
        Find the data edges. This is complicated by the fact that DEMs frequently
        have nodata edges, whereby the DEM does not occupy the full extent of 
        the raster. One approach to doing this would be simply to scan the
        raster, looking for cells that neighbour nodata values. However, this
        assumes that there are no interior nodata holes in the dataset. Instead,
        the approach used here is to perform a region-growing operation, looking
        for nodata values along the raster's edges.
        */

        let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity((rows * columns) as usize);
        for row in 0..rows {
            /*
            Note that this is only possible because Whitebox rasters
            allow you to address cells beyond the raster extent but
            return the nodata value for these regions.
            */
            queue.push_back((row, -1));
            queue.push_back((row, columns));
        }

        for col in 0..columns {
            queue.push_back((-1, col));
            queue.push_back((rows, col));
        }


        /* 
        minheap is the priority queue. Note that I've tested using integer-based
        priority values, by multiplying the elevations, but this didn't result
        in a significant performance gain over the use of f64s.
        */
        let mut minheap = BinaryHeap::with_capacity((rows * columns) as usize);
        let mut num_solved_cells = 0;
        let mut zin_n: f64; // value of neighbour of row, col in input raster
        let mut zout: f64; // value of row, col in output raster
        let mut zout_n: f64; // value of neighbour of row, col in output raster
        let dx = [ 1, 1, 1, 0, -1, -1, -1, 0 ];
        let dy = [ -1, 0, 1, 1, 1, 0, -1, -1 ];
        let (mut row, mut col): (isize, isize);
        let (mut row_n, mut col_n): (isize, isize);
        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            row = cell.0;
            col = cell.1;
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                zin_n = input[(row_n, col_n)];
                zout_n = output[(row_n, col_n)];
                if zout_n == background_val {
                    if zin_n == nodata {
                        output[(row_n, col_n)] = nodata;
                        queue.push_back((row_n, col_n));
                    } else {
                        output[(row_n, col_n)] = zin_n;
                        // Push it onto the priority queue for the priority flood operation
                        minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                    }
                    num_solved_cells += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Perform the priority flood operation.
        let back_link = [ 4i8, 5i8, 6i8, 7i8, 0i8, 1i8, 2i8, 3i8 ];
        let (mut x, mut y): (isize, isize);
        let mut z_target: f64;
        let mut dir: i8;
        let mut flag: bool;

        while !minheap.is_empty() {
            let cell = minheap.pop().unwrap();
            row = cell.row;
            col = cell.column;
            zout = output[(row, col)];
            for n in 0..8 {
                row_n = row + dy[n];
                col_n = col + dx[n];
                zout_n = output[(row_n, col_n)];
                if zout_n == background_val {
                    zin_n = input[(row_n, col_n)];
                    if zin_n != nodata {
                        flow_dir[(row_n, col_n)] = back_link[n];
                        output[(row_n, col_n)] = zin_n;
                        minheap.push(GridCell{ row: row_n, column: col_n, priority: zin_n });
                        if zin_n < (zout + small_num) {
                            // Is it a pit cell?
                            // is_pit = true;
                            // for n2 in 0..8 {
                            //     row_n2 = row + dy[n2];
                            //     col_n2 = col + dx[n2];
                            //     zin_n2 = input[(row_n2, col_n2)];
                            //     if zin_n2 != nodata && zin_n2 < zin_n {
                            //         is_pit = false;
                            //         break;
                            //     }
                            // }
                            // if is_pit {
                                // Trace the flowpath back to a lower cell, if it exists.
                                x = col_n;
                                y = row_n;
                                z_target = output[(row_n, col_n)];
                                flag = true;
                                while flag {
                                    dir = flow_dir[(y, x)];
                                    if dir >= 0 {
                                        y += dy[dir as usize];
                                        x += dx[dir as usize];
                                        z_target -= small_num;
                                        if output[(y, x)] > z_target {
                                            output[(y, x)] = z_target;
                                        } else {
                                            flag = false;
                                        }
                                    } else {
                                        flag = false;
                                    }
                                }
                            // }
                        }
                    } else {
                        // Interior nodata cells are still treated as nodata and are not filled.
                        output[(row_n, col_n)] = nodata;
                        num_solved_cells += 1;
                    }
                }
            }

            if verbose {
                num_solved_cells += 1;
                progress = (100.0_f64 * num_solved_cells as f64 / (num_cells - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }



    }
}

