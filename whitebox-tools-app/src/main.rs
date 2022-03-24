/*
This code is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 21/06/2017
Last Modified: 30/01/2022
License: MIT
*/

/*!
WhiteboxTools is an advanced geospatial data analysis platform developed at
the University of Guelph's Geomorphometry and Hydrogeomatics Research Group (GHRG).

WhiteboxTools is a command-line program and can be run either by calling it,
with appropriate commands and arguments, from a terminal application, or, more
conveniently, by calling it from a script. The following commands are recognized
by the WhiteboxTools library:

| Command           | Description                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------- |
| --cd, --wd        | Changes the working directory; used in conjunction with --run flag.                               |
| -h, --help        | Prints help information.                                                                          |
| -l, --license     | Prints the whitebox-tools license. Tool names may also be used, --license=\"Slope\"               |
| --listtools       | Lists all available tools, with tool descriptions. Keywords may also be used, --listtools slope.  |
| -r, --run         | Runs a tool; used in conjunction with --cd flag; -r="LidarInfo".                                  |
| --toolbox         | Prints the toolbox associated with a tool; --toolbox=Slope.                                       |
| --toolhelp        | Prints the help associated with a tool; --toolhelp="LidarInfo".                                   |
| --toolparameters  | Prints the parameters (in json form) for a specific tool; --toolparameters=\"LidarInfo\".         |
| -v                | Verbose mode. Without this flag, tool outputs will not be printed.                                |
| --viewcode        | Opens the source code of a tool in a web browser; --viewcode=\"LidarInfo\".                       |
| --version         | Prints the version information.                                                                   |

*/

// pub mod algorithms;
// pub mod lidar;
// pub mod raster;
// pub mod rendering;
// pub mod spatial_ref_system;
// pub mod structures;
pub mod tools;
// pub mod utils;
// pub mod vector;

use crate::tools::ToolManager;
use nalgebra as na;
// use rstar;
use std::env;
use std::io::Error;
use std::path;

#[macro_use]
extern crate serde_derive;

// extern crate late_static;
// use late_static::LateStatic;

// pub static USE_COMPRESSION: LateStatic<bool> = LateStatic::new();

/// WhiteboxTools is an advanced geospatial data analysis engine.
///
/// # Examples
///
/// From the command line prompt, *WhiteboxTools* can be called to run a tool as follows:
///
/// ```
/// >>./whitebox_tools --wd='/Users/johnlindsay/Documents/data/' --run=DevFromMeanElev --input='DEM clipped.dep' --output='DEV raster.dep' -v
/// ```

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => panic!("{}", err),
    }
}

// // This is just used for testing new features.
// fn main() {
//     // let file_name = "/Users/johnlindsay/Documents/data/whitebox_cities.shp";
//     let file_name = "/Users/johnlindsay/Documents/data/world_map.shp";
//     // let file_name = "/Users/johnlindsay/Documents/data/Minnesota/HUC07inside08_buff25m.shp";

//     let sf = vector::Shapefile::new(file_name, "r").unwrap();
//     println!("{}", sf.header);
//     for i in 0..sf.num_records {
//         println!("id={}, {:?}", i, sf.get_record(i).points);
//     }
// }

fn run() -> Result<(), Error> {
    let sep: &str = &path::MAIN_SEPARATOR.to_string();
    let mut working_dir = String::new();
    let mut tool_name = String::new();
    let mut run_tool = false;
    let mut tool_help = false;
    let mut tool_parameters = false;
    let mut toolbox = false;
    let mut list_tools = false;
    let mut keywords: Vec<String> = vec![];
    let mut view_code = false;
    let mut tool_args_vec: Vec<String> = vec![];
    // let mut verbose = false;
    let mut finding_working_dir = false;
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        version();
        // print help
        help();
        // list tools
        let tm = ToolManager::new(&working_dir, &false)?;
        tm.list_tools();

        return Ok(());
    }

    let mut configs = whitebox_common::configs::get_configs()?;
    let mut configs_modified = false;

    // if args.contains(&String::from("--compress_rasters")) {
    //     // unsafe {
    //     //     LateStatic::assign(&USE_COMPRESSION, true);
    //     // }
    //     configs.compress_raster = true;
    // } else {
    //     // unsafe {
    //     //     LateStatic::assign(&USE_COMPRESSION, false);
    //     // }
    //     configs.compress_raster = false;
    // }

    // if args.contains(&String::from("-v")) {
    //     configs.verbose_mode = true;
    //     verbose = true;
    // } else {
    //     configs.verbose_mode = false;
    // }

    for arg in args {
        let flag_val = arg.to_lowercase().replace("--", "-");
        if flag_val == "-h" || flag_val == "-help" {
            help();
            return Ok(());
        } else if flag_val.starts_with("-cd") || flag_val.starts_with("-wd") || flag_val.starts_with("-working_directory") {
            let mut v = arg
                .replace("--cd", "")
                .replace("--wd", "")
                .replace("--working_directory", "")
                .replace("-cd", "")
                .replace("-wd", "")
                .replace("-working_directory", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            if v.trim().is_empty() {
                finding_working_dir = true;
            }
            if !v.ends_with(sep) {
                v.push_str(sep);
            }
            working_dir = v.to_string();

            let sep = path::MAIN_SEPARATOR;
            if !working_dir.ends_with(sep) {
                working_dir.push_str(&(sep.to_string()));
                configs.working_directory = working_dir.clone();
            }
            if configs.working_directory != working_dir { // update the value
                configs.working_directory = working_dir.clone();
                configs_modified = true;
            }
        } else if arg.starts_with("-run") || arg.starts_with("--run") || arg.starts_with("-r") {
            let mut v = arg
                .replace("--run", "")
                .replace("-run", "")
                .replace("-r", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            run_tool = true;
        } else if arg.starts_with("-toolhelp") || arg.starts_with("--toolhelp") {
            let mut v = arg
                .replace("--toolhelp", "")
                .replace("-toolhelp", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            tool_help = true;
        } else if arg.starts_with("-toolparameters") || arg.starts_with("--toolparameters") {
            let mut v = arg
                .replace("--toolparameters", "")
                .replace("-toolparameters", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            tool_parameters = true;
        } else if arg.starts_with("-toolbox") || arg.starts_with("--toolbox") {
            let mut v = arg
                .replace("--toolbox", "")
                .replace("-toolbox", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            toolbox = true;
        } else if arg.starts_with("-listtools")
            || arg.starts_with("--listtools")
            || arg.starts_with("-list_tools")
            || arg.starts_with("--list_tools")
        {
            list_tools = true;
        } else if arg.starts_with("-viewcode") || arg.starts_with("--viewcode") {
            let mut v = arg
                .replace("--viewcode", "")
                .replace("-viewcode", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            view_code = true;
        } else if arg.starts_with("-license")
            || arg.starts_with("-licence")
            || arg.starts_with("--license")
            || arg.starts_with("--licence")
            || arg.starts_with("-l")
        {
            tool_name = arg
                .replace("--license", "")
                .replace("-license", "")
                .replace("--licence", "")
                .replace("-licence", "")
                .replace("\"", "")
                .replace("\'", "");
            if tool_name.starts_with("=") {
                tool_name = tool_name[1..tool_name.len()].to_string();
                if !tool_name.is_empty() {
                    let tm = ToolManager::new(&configs.working_directory, &configs.verbose_mode)?;
                    return tm.tool_license(tool_name);
                }
            } else {
                license();
            }
            return Ok(());
        } else if arg.starts_with("-compress_raster") || arg.starts_with("--compress_raster") {
            let mut v = arg
                .replace("--compress_rasters", "")
                .replace("-compress_rasters", "")
                .replace("--compress_raster", "")
                .replace("-compress_raster", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            if v.to_lowercase().contains("t") || v.is_empty() {
                if !configs.compress_rasters { // update value
                    configs.compress_rasters = true;
                    configs_modified = true;
                }
            } else {
                if configs.compress_rasters { // update value
                    configs.compress_rasters = false;
                    configs_modified = true;
                }
            }
        } else if arg.starts_with("-v") || arg.starts_with("--verbose") {
            let mut v = arg
                .replace("-v", "")
                .replace("--verbose", "")
                .replace("-verbose", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            if v.to_lowercase().contains("t") || v.is_empty() {
                if !configs.verbose_mode {
                    configs.verbose_mode = true;
                    configs_modified = true;
                }
            } else {
                if configs.verbose_mode {
                    configs.verbose_mode = false;
                    configs_modified = true;
                }
            }
        } else if arg.starts_with("-max_procs") || arg.starts_with("--max_procs") {
            let mut v = arg
                .replace("--max_procs", "")
                .replace("-max_procs", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            let val = v.parse::<isize>().expect(&format!("Error parsing {}", v));
            if val != configs.max_procs {
                configs.max_procs = val;
                configs_modified = true;
            }
        } else if arg.starts_with("-version") || arg.starts_with("--version") {
            version();
            return Ok(());
        // } else if arg.trim() == "-v" {
        //     verbose = true;
        } else if arg.starts_with("-") {
            // it's an arg to be fed to the tool
            if !arg.contains("-17976931348623157") {
                // The QGIS plugin doesn't seem to handle numerical arguments that don't supply default values very well.
                // When this is the case, it will use an extremely large negative value, starting with the sequence above,
                // as the default. So if this number occurs in the argument, it means that the value was unspecified. If
                // it's an optional parameter, the tool will be able to handle this situation. If not, an error will likely
                // be thrown by the absence of the parameter.
                tool_args_vec.push(arg.trim().to_string().clone());
            }
        } else if !arg.contains("whitebox_tools") {
            // add it to the keywords list
            keywords.push(
                arg.trim()
                    .replace("\"", "")
                    .replace("\'", "")
                    .to_string()
                    .clone(),
            );
            if finding_working_dir {
                working_dir = arg.trim().to_string().clone();
                finding_working_dir = false;
                configs.working_directory = working_dir.clone();
                configs_modified = true;
            } else if tool_args_vec.len() > 0 {
                tool_args_vec.push(arg.trim().to_string().clone());
            }
        }
    }

    // let sep = path::MAIN_SEPARATOR;
    // if !working_dir.ends_with(sep) {
    //     working_dir.push_str(&(sep.to_string()));
    //     configs.working_directory = working_dir.clone();
    // }

    if configs_modified {
        whitebox_common::configs::save_configs(&configs)?;
    }

    let tm = ToolManager::new(&configs.working_directory, &configs.verbose_mode)?;
    if run_tool {
        if tool_name.is_empty() && keywords.len() > 0 {
            tool_name = keywords[0].clone();
        }
        return tm.run_tool(tool_name, tool_args_vec);
    } else if tool_help {
        if tool_name.is_empty() && keywords.len() > 0 {
            tool_name = keywords[0].clone();
        }
        return tm.tool_help(tool_name);
    } else if tool_parameters {
        if tool_name.is_empty() && keywords.len() > 0 {
            tool_name = keywords[0].clone();
        }
        return tm.tool_parameters(tool_name);
    } else if toolbox {
        if tool_name.is_empty() && keywords.len() > 0 {
            tool_name = keywords[0].clone();
        }
        if tool_name.is_empty() {
            tool_name = String::new();
        }
        return tm.toolbox(tool_name);
    } else if list_tools {
        if keywords.len() == 0 {
            tm.list_tools();
        } else {
            tm.list_tools_with_keywords(keywords);
        }
    } else if view_code {
        if tool_name.is_empty() && keywords.len() > 0 {
            tool_name = keywords[0].clone();
        }
        return tm.get_tool_source_code(tool_name);
    }

    Ok(())
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("whitebox_tools{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = "WhiteboxTools Help

The following commands are recognized:
--cd, --wd          Changes the working directory; used in conjunction with --run flag.
--compress_rasters  Sets the compress_raster option in the settings.json file; determines if newly created rasters are compressed. e.g. --compress_rasters=true
-h, --help          Prints help information.
-l, --license       Prints the whitebox-tools license. Tool names may also be used, --license=\"Slope\"
--listtools         Lists all available tools. Keywords may also be used, --listtools slope.
--max_procs         Sets the maximum number of processors used. -1 = all available processors. e.g. --max_procs=2
-r, --run           Runs a tool; used in conjunction with --wd flag; -r=\"LidarInfo\".
--toolbox           Prints the toolbox associated with a tool; --toolbox=Slope.
--toolhelp          Prints the help associated with a tool; --toolhelp=\"LidarInfo\".
--toolparameters    Prints the parameters (in json form) for a specific tool; --toolparameters=\"LidarInfo\".
-v                  Verbose mode. Without this flag, tool outputs will not be printed.
--viewcode          Opens the source code of a tool in a web browser; --viewcode=\"LidarInfo\".
--version           Prints the version information.

Example Usage:
>> .*EXE_NAME -r=lidar_info --cd=\"*path*to*data*\" -i=input.las --vlr --geokeys
"
            .replace("*", &sep)
            .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn license() {
    let license_text = "WhiteboxTools License
Copyright 2017-2021 John Lindsay

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the \"Software\"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.";
    println!("{}", license_text);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "WhiteboxTools v{} by Dr. John B. Lindsay (c) 2017-2022

WhiteboxTools is an advanced geospatial data analysis platform developed at
the University of Guelph's Geomorphometry and Hydrogeomatics Research 
Group (GHRG). See www.whiteboxgeo.com for more details.",
        VERSION.unwrap_or("unknown")
    );
}
