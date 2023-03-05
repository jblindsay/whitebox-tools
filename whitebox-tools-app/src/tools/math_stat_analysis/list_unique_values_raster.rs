/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 12/04/2018
Last Modified: 09/02/2023
License: MIT
*/

use crate::tools::*;
use whitebox_raster::*;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use num_cpus;

/// This tool can be used to list each of the unique values contained within a categorical raster (`input`). The tool 
/// outputs an HTML formatted report (`--output`)
/// containing a table of the unique values and their frequency of occurrence within the data. The user must
/// specify the name of an input shapefile (`--input`) and the name of one of the fields (`--field`)
/// contained in the associated attribute table. The specified field *should not contained floating-point
/// numerical data*, since the number of categories will likely equal the number of records, which may be
/// quite large. The tool effectively provides tabular output that is similar to the graphical output
/// provided by the `AttributeHistogram` tool, which, however, can be applied to continuous data.
///
/// # See Also
/// `ListUniqueValues`
pub struct ListUniqueValuesRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ListUniqueValuesRaster {
    pub fn new() -> ListUniqueValuesRaster {
        // public constructor
        let name = "ListUniqueValuesRaster".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description =
            "Lists the unique values contained in a field within a vector's attribute table."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=lakes.shp --field=HEIGHT -o=outfile.html",
            short_exe, name
        )
        .replace("*", &sep);

        ListUniqueValuesRaster {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ListUniqueValuesRaster {
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
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if verbose {
            println!("Reading vector data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);
        let start = Instant::now();
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }

        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut freq_data = HashMap::new();
                let mut z: f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            let count = freq_data.entry(z as usize).or_insert(0);
                            *count += 1;
                        }
                    }
                }

                tx1.send(freq_data).unwrap();
            });
        }

        let mut freq_data = HashMap::new();
        for n in 0..num_procs {
            let data = rx.recv().expect("Error receiving data from thread.");
            for (category, count) in &data {
                let overall_count = freq_data.entry(*category).or_insert(0);
                *overall_count += *count;
            }

            if verbose {
                progress = (100.0_f64 * (n + 1) as f64 / num_procs as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut freq_data_vec = vec![];
        for (category, count) in &freq_data {
            freq_data_vec.push((category, *count as usize));
        }

        if freq_data.len() > 500 {
            if verbose {
                println!("Warning: There are a large number of categories. A continuous attribute variable may have been input incorrectly.");
            }
        }

        freq_data_vec.sort();

        let mut ret_str = String::from("Category,Frequency\n");
        for i in 0..freq_data_vec.len() {
            ret_str.push_str(&format!("{},{}\n", freq_data_vec[i].0, freq_data_vec[i].1));
        }

        println!("{ret_str}");

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
