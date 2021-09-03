/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/10/2018
Last Modified: 12/10/2018
License: MIT
*/

use crate::tools::*;
use whitebox_vector::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

/// Combines two or more input vectors of the same ShapeType creating a single, new output
/// vector. Importantly, the attribute table of the output vector will contain the ubiquitous
/// file-specific FID, the parent file name, the parent FID, and the list of attribute fields
/// that are shared among each of the input files. For a field to be considered common
/// between tables, it must have the same `name` and `field_type` (i.e. data type and
/// precision).
///
/// Overlapping features will not be identified nor handled in the merging. If you have
/// significant areas of overlap, it is advisable to use one of the vector overlay tools
/// instead.
///
/// The difference between `MergeVectors` and the `Append` tool is that merging takes two
/// or more files and creates one new file containing the features of all inputs, and
/// `Append` places the features of a single vector into another existing (appended) vector.
///
/// This tool only operates on vector files. Use the `Mosaic` tool to combine raster data.
///
/// # See Also
/// `Append`, `Mosaic`
pub struct MergeVectors {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl MergeVectors {
    pub fn new() -> MergeVectors {
        // public constructor
        let name = "MergeVectors".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Combines two or more input vectors of the same ShapeType creating a single, new output vector.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Files".to_string(),
            flags: vec!["-i".to_owned(), "--inputs".to_string()],
            description: "Input vector files.".to_string(),
            parameter_type: ParameterType::FileList(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Vector File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='polys1.shp;polys2.shp;polys3.shp' -o=out_file.shp",
            short_exe, name
        ).replace("*", &sep);

        MergeVectors {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for MergeVectors {
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
        let mut input_files = String::new();
        let mut output_file: String = "".to_string();

        // read the arguments
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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-inputs" {
                input_files = if keyval {
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

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

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

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let mut cmd = input_files.split(";");
        let mut vec = cmd.collect::<Vec<&str>>();
        if vec.len() == 1 {
            cmd = input_files.split(",");
            vec = cmd.collect::<Vec<&str>>();
        }
        let num_files = vec.len();
        if num_files < 2 {
            return Err(Error::new(ErrorKind::InvalidInput,
                    "There is something incorrect about the input files. At least two inputs are required to operate this tool."));
        }

        // We need to initialize output here, but in reality this can't be done
        // until we know the size of rows and columns, which occurs during the first loop.
        let mut output: Shapefile = Shapefile::new(&output_file, ShapeType::Null)?;
        let mut read_first_file = false;

        // It will be necessary to find which fields in the attribute tables are in
        // common among the input files.
        let mut in_files: Vec<String> = vec![];
        let mut atts: Vec<AttributeField> = vec![AttributeField {
            ..Default::default()
        }];

        // The current program structure is not ideal because each of the input files
        // are read twice; once to find the set of attributes shared among all of the
        // inputs and a second time to fill the output file. I can't currently think
        // of a way to avoid this doubling of IO that doesn't involve storing each
        // vector in memory for the second pass, which would also be suboptimal.
        for value in vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_string();
                if !input_file.contains(&sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                in_files.push(input_file.clone());

                if verbose {
                    println!("Reading '{}'", input_file);
                };

                let input = Shapefile::read(&input_file)?;

                if !read_first_file {
                    read_first_file = true;

                    // Add the attributes. The strategy is to
                    // initialize the attribute table with all
                    // of the attributes of the first vector
                    // file (excluding the FID), and then to
                    // remove any attribute field that is not
                    // found in any of the subsequent files.
                    // let a = input.attributes.get_fields();
                    for att in input.attributes.get_fields() {
                        if att.name.to_lowercase() != "fid" {
                            atts.push(att.clone());
                        }
                    }

                    // initialize the output
                    output = Shapefile::initialize_using_file(
                        &output_file,
                        &input,
                        input.header.shape_type,
                        false,
                    )?;
                } else {
                    atts.intersection(input.attributes.get_fields());
                }

                if input.header.shape_type != output.header.shape_type {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Each of the input files must be of the same ShapeType.",
                    ));
                }
            }
        }

        atts.insert(0, AttributeField::new("FID", FieldDataType::Int, 8u8, 0u8));
        atts.insert(
            1,
            AttributeField::new("PARENT", FieldDataType::Text, 25u8, 0u8),
        );
        atts.insert(
            2,
            AttributeField::new("PARENT_FID", FieldDataType::Int, 8u8, 0u8),
        );

        let num_atts = atts.len();
        for a in &atts {
            output.attributes.add_field(&(a.clone()));
        }

        let mut fid = 1i32;
        for input_file in in_files {
            let input = Shapefile::read(&input_file)?;
            let short_name = input.get_short_filename().replace(".shp", "");

            for record_num in 0..input.num_records {
                let record = input.get_record(record_num).clone();
                output.add_record(record);

                // attributes
                let mut out_atts: Vec<FieldData> = Vec::with_capacity(num_atts);
                out_atts.push(FieldData::Int(fid)); // the record FID
                fid += 1;
                out_atts.push(FieldData::Text(short_name.clone())); // parent file name.
                out_atts.push(FieldData::Int(record_num as i32 + 1)); // the record PARENT_FID

                // Now the list of shared attributes.
                for i in 3..num_atts {
                    out_atts.push(input.attributes.get_value(record_num, &(atts[i].name)))
                }

                output.attributes.add_record(out_atts, false);

                if verbose {
                    progress =
                        (100.0_f64 * (record_num + 1) as f64 / input.num_records as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }
        }

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

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
