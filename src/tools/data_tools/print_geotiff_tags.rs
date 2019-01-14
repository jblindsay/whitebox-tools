/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: March 2, 2018
Last Modified: March 2, 2018
License: MIT
*/

use crate::raster::geotiff::*;
use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
// use crate::tools::ToolParameter;
// use crate::tools::ParameterType;
// use crate::tools::ParameterFileType;

/// This tool can be used to view the tags contained within a GeoTiff file. Viewing 
/// the tags of a GeoTiff file can be useful when trying to import the GeoTiff to
/// different software environments. The user must specify the name of a GeoTiff file 
/// and the tag information will be output to the StdOut output stream (e.g. console). 
/// Note that tags that contain greater than 100 values will be truncated in the output. 
/// GeoKeys will also be interpreted as per the GeoTIFF specification.
pub struct PrintGeoTiffTags {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PrintGeoTiffTags {
    pub fn new() -> PrintGeoTiffTags {
        // public constructor
        let name = "PrintGeoTiffTags".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Prints the tags within a GeoTIFF.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input GeoTIFF Raster File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input GeoTIFF file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        // parameters.push(ToolParameter{
        //     name: "Output HTML File".to_owned(),
        //     flags: vec!["-o".to_owned(), "--output".to_owned()],
        //     description: "Output HTML file.".to_owned(),
        //     parameter_type: ParameterType::NewFile(ParameterFileType::Html),
        //     default_value: None,
        //     optional: false
        // });

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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --input=DEM.tiff",
            short_exe, name
        )
        .replace("*", &sep);

        PrintGeoTiffTags {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PrintGeoTiffTags {
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
        // let mut output_file = String::new();

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
            if vec[0].to_lowercase() == "-i" || vec[0].to_lowercase() == "--input" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
                // } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                //     if keyval {
                //         output_file = vec[1].to_string();
                //     } else {
                //         output_file = args[i + 1].to_string();
                //     }
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

        // make sure that it is a tiff file
        if !input_file.to_lowercase().ends_with(".tiff")
            && !input_file.to_lowercase().ends_with(".tif")
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input file must be in a GeoTIFF format.",
            ));
        }

        match print_tags(&input_file) {
            Ok(_) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}
