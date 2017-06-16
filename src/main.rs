extern crate byteorder;

pub mod io_utils;
pub mod lidar;
pub mod raster;
pub mod tools;
pub mod structures;

use std::io::Error; //{Error, ErrorKind};
use std::env;
use std::path;
use tools::ToolManager;

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => panic!("{}", err),
    }
}

fn run() -> Result<(), Error> {
    let sep: &str = &path::MAIN_SEPARATOR.to_string();
    let mut working_dir = String::new();
    let mut tool_name = String::new();
    let mut run_tool = false;
    let mut tool_help = false;
    let mut list_tools = false;
    let mut tool_args_vec: Vec<String> = vec![];
    let mut verbose = false;
        let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        // return Err(Error::new(ErrorKind::InvalidInput,
        //                       "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
        help();
        return Ok(());
    }
    for arg in args {
        if arg.starts_with("-h") || arg.starts_with("--help") {
            help();
            return Ok(());
        } else if arg.starts_with("-cd") || arg.starts_with("--cd") || arg.starts_with("--wd") {
            let mut v = arg.replace("--cd", "")
                .replace("--wd", "")
                .replace("-cd", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            if !v.ends_with(sep) {
                v.push_str(sep);
            }
            // working_dir = format!("\"{}\"", v);
            working_dir = v.to_string();
        } else if arg.starts_with("-run") || arg.starts_with("--run") || arg.starts_with("-r") {
            let mut v = arg.replace("--run", "")
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
            let mut v = arg.replace("--toolhelp", "")
                .replace("-toolhelp", "")
                .replace("\"", "")
                .replace("\'", "");
            if v.starts_with("=") {
                v = v[1..v.len()].to_string();
            }
            tool_name = v;
            tool_help = true;
        } else if arg.starts_with("-listtools") || arg.starts_with("--listtools") {
            list_tools = true;
        } else if arg.starts_with("-license") || arg.starts_with("-licence") ||
                  arg.starts_with("--license") ||
                  arg.starts_with("--licence") || arg.starts_with("-l") {
            license();
            return Ok(());
        } else if arg.starts_with("-version") || arg.starts_with("--version") {
            version();
            return Ok(());
        } else if arg.starts_with("-v") {
            verbose = true;
        } else if arg.starts_with("-") {
            // it's an arg to be fed to the tool
            // println!("arg: {}", arg); //temp
            tool_args_vec.push(arg.trim().to_string().clone());
        }
    }

    let sep = path::MAIN_SEPARATOR;
    if !working_dir.ends_with(sep) {
        working_dir.push_str(&(sep.to_string()));
    }
    let tm = ToolManager::new(&working_dir, &verbose)?;
    if run_tool {
        return tm.run_tool(tool_name, tool_args_vec);
    } else if tool_help {
        return tm.tool_help(tool_name);
    } else if list_tools {
        tm.list_tools();
    }

    Ok(())
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("whitebox-tools{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = "whitebox-tools Help

The following commands are recognized:
--cd, --wd       Changes the working directory; used in conjunction with --run flag.
-l, --license    Prints the whitebox-tools license.
--listtools      Lists all available tools.
-r, --run        Runs a tool; used in conjuction with --args and --cd flags; -r=\"lidar_info\".
--toolhelp       Prints the help associated with a tool; --toolhelp=\"lidar_info\".
-h, --help       Prints help information.

Example Usage:
>> .*EXE_NAME -r=lidar_info --cd=\"*path*to*data*\" -i=input.las --vlr --geokeys
"
            .replace("*", &sep)
            .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn license() {
    let license_text = "whitebox-tools License
Copyright 2017 John Lindsay

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
    println!("whitebox-tools v{}", VERSION.unwrap_or("unknown"));
}
