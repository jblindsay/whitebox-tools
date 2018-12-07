/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Prof. John Lindsay
Created: 07/10/2018
Last Modified: 22/11/2018
License: MIT
*/

use crate::tools::*;
use crate::vector::{FieldData, Shapefile};
use std::collections::HashMap;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to join (i.e. merge) a vector's attribute table with a second table. The
/// user must specify the name of the vector file (and associated attribute file) as well as the
/// *primary key* within the table. The *primary key* (`--pkey` flag) is the field
/// within the table that is being appended to that serves as the identifier. Additionally, the user
/// must specify the name of a second vector from which the data appended into the first table will be
/// derived. The *foreign key* (`--fkey` flag), the identifying field within the
/// second table that corresponds with the data contained within the primary key in the table, must be
/// specified. Both the primary and foreign keys should either be strings (text) or integer values.
/// *Fields containing decimal values are not good candidates for keys.* Lastly, the names of the field
/// within the second file to include in the merge operation can also be input (`--import_field`). If the
/// `--import_field` field is not input, all fields in the attribute table of the second file, that are not
/// the foreign key nor FID, will be imported to the first table.
///
/// Merging works for one-to-one and many-to-one database relations. A *one-to-one* relations exists when
/// each record in the attribute table corresponds to one record in the second table and each primary
/// key is unique. Since each record in the attribute table is associated with a geospatial feature in
/// the vector, an example of a one-to-one relation may be where the second file contains AREA and
/// PERIMETER fields for each polygon feature in the vector. This is the most basic type of relation.
/// A many-to-one relation would exist when each record in the first attribute table corresponds to one
/// record in the second file and the primary key is NOT unique. Consider as an example a vector and
/// attribute table associated with a world map of countries. Each country has one or more more polygon
/// features in the shapefile, e.g. Canada has its mainland and many hundred large islands. You may want
/// to append a table containing data about the population and area of each country. In this case, the
/// COUNTRY columns in the attribute table and the second file serve as the primary and foreign keys
/// respectively. While there may be many duplicate primary keys (all of those Canadian polygons) each
/// will correspond to only one foreign key containing the population and area data. This is a
/// *many-to-one* relation. The `JoinTables` tool does not support one-to-many nor many-to-many relations.
///
/// # See Also
/// `MergeTableWithCsv`, `ReinitializeAttributeTable`, `ExportTableToCsv`
pub struct JoinTables {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl JoinTables {
    /// public constructor
    pub fn new() -> JoinTables {
        let name = "JoinTables".to_string();
        let toolbox = "Data Tools".to_string();
        let description =
            "Merge a vector's attribute table with another table based on a common field."
                .to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Primary Vector File".to_owned(),
            flags: vec!["--i1".to_owned(), "--input1".to_owned()],
            description: "Input primary vector file (i.e. the table to be modified).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Primary Key Field".to_owned(),
            flags: vec!["--pkey".to_owned()],
            description: "Primary key field.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input1".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Foreign Vector File".to_owned(),
            flags: vec!["--i2".to_owned(), "--input2".to_owned()],
            description: "Input foreign vector file (i.e. source of data to be imported)."
                .to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Foreign Key Field".to_owned(),
            flags: vec!["--fkey".to_owned()],
            description: "Foreign key field.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input2".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Imported Field".to_owned(),
            flags: vec!["--import_field".to_owned()],
            description: "Imported field (all fields will be imported if not specified)."
                .to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Any,
                "--input2".to_string(),
            ),
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
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=properties.shp --pkey=TYPE --i2=land_class.shp --fkey=VALUE --import_field=NEW_VALUE",
            short_exe, name
        ).replace("*", &sep);

        JoinTables {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for JoinTables {
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
        let mut input1_file = String::new();
        let mut primary_key = String::new();
        let mut input2_file = String::new();
        let mut foreign_key = String::new();
        let mut import_field = String::new();

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
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i1" || flag_val == "-input1" {
                input1_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-primary_key" || flag_val == "-pkey" {
                primary_key = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-i2" || flag_val == "-input2" {
                input2_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-foreign_key" || flag_val == "-fkey" {
                foreign_key = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-import_field" {
                import_field = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
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

        if !input1_file.contains(&sep) && !input1_file.contains("/") {
            input1_file = format!("{}{}", working_directory, input1_file);
        }
        if !input2_file.contains(&sep) && !input2_file.contains("/") {
            input2_file = format!("{}{}", working_directory, input2_file);
        }

        if verbose {
            println!("Reading data...")
        };
        let input1 = Shapefile::read(&input1_file)?;
        let input2 = Shapefile::read(&input2_file)?;

        let start = Instant::now();

        // create output file
        let mut output = Shapefile::initialize_using_file(
            &input1_file,
            &input1,
            input1.header.shape_type,
            true,
        )?;

        // What is the index of the foreign field?
        let fkey_index = match input2.attributes.get_field_num(&foreign_key) {
            Some(i) => i,
            None => {
                // Field not found
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Foreign Key was not located in table.",
                ));
            }
        };

        let fields_to_append = if import_field.is_empty() {
            // append all fields except the fkey and any FID
            let mut ret = vec![];
            for a in 0..input2.attributes.get_num_fields() {
                let f = input2.attributes.get_field(a);
                if a != fkey_index && f.name.to_lowercase() != "fid" {
                    ret.push(f.clone());
                }
            }
            ret
        } else {
            // just append the import field
            let import_index = match input2.attributes.get_field_num(&import_field) {
                Some(i) => i,
                None => {
                    // Field not found
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Import field was not located in table.",
                    ));
                }
            };
            vec![input2.attributes.get_field(import_index).clone()]
        };

        // update the vector1 attribute table
        for f in &fields_to_append {
            output.attributes.add_field(&f);
        }

        // read the second file into a hashmap
        let (mut pkey_value, mut fkey_value): (String, String);
        let mut data_map = HashMap::new();
        for record_num in 0..input2.num_records {
            fkey_value = input2
                .attributes
                .get_value(record_num, &foreign_key)
                .to_string();
            let mut imported_data = vec![];
            for a in &fields_to_append {
                imported_data.push(input2.attributes.get_value(record_num, &(a.name)));
            }
            data_map.insert(fkey_value, imported_data.clone());
        }

        // print the attribute data
        for record_num in 0..input1.num_records {
            // geometries
            let record = input1.get_record(record_num);
            output.add_record(record.clone());
            // attributes
            let mut atts = input1.attributes.get_record(record_num);

            pkey_value = input1
                .attributes
                .get_value(record_num, &primary_key)
                .to_string();
            match data_map.get(&pkey_value) {
                Some(v) => {
                    for a in v {
                        atts.push(a.clone());
                    }
                }
                None => {
                    // return Err(Error::new(
                    //     ErrorKind::InvalidInput,
                    //     "Error mapping primary key value to foreign key value.",
                    // ));

                    // add nulls to the att table
                    for _ in 0..fields_to_append.len() {
                        atts.push(FieldData::Null);
                    }
                }
            }

            output.attributes.add_record(atts, false);

            if verbose {
                progress =
                    (100.0_f64 * (record_num + 1) as f64 / input1.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
            Ok(_) => if verbose {
                println!("Output file written")
            },
            Err(e) => return Err(e),
        };

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}
