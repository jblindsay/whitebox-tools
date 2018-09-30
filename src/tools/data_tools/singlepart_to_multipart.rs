/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/09/2018
Last Modified: 30/09/2018
License: MIT
*/

use algorithms::{is_clockwise_order, poly_in_poly};
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use time;
use tools::*;
use vector::*;

/// This tool can be used to convert a vector file containing single-part features into a vector
/// containing multi-part features. The user has the option to either group features based on an
/// ID Field (`--field` flag), which is a categorical field within the vector's attribute table.
/// The ID Field should either be of String (text) or Integer type. Fields containing decimal values
/// are not good candidates for the ID Field. **If no `--field` flag is specified, all features will
/// be grouped together into one large multi-part vector**.
///
/// This tool works for vectors containing either point, line, or polygon features.
/// Since vectors of a POINT ShapeType cannot represent multi-part features, the ShapeType of the
/// output file will be modified to a MULTIPOINT ShapeType if the input file is of a POINT ShapeType.
/// If the input vector is of a POLYGON ShapeType, the user can optionally set the algorithm to search
/// for polygons that should be represented as hole parts. In the case of grouping based on an ID Field,
/// hole parts are polygon features contained within larger polygons of the same ID Field value. Please
/// note that searching for polygon holes may significantly increase processing time for larger polygon
/// coverages.
///
/// **See Also**: `MultiPartToSinglePart`
pub struct SinglePartToMultiPart {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl SinglePartToMultiPart {
    pub fn new() -> SinglePartToMultiPart {
        // public constructor
        let name = "SinglePartToMultiPart".to_string();
        let toolbox = "Data Tools".to_string();
        let description = "Converts a vector file containing multi-part features into a vector containing only single-part features.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Line or Polygon File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector line or polygon file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Grouping ID Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Grouping ID field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Line or Polygon File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output vector line or polygon file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.shp -o=output.shp --field='COUNTRY'",
            short_exe, name
        ).replace("*", &sep);

        SinglePartToMultiPart {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for SinglePartToMultiPart {
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
        let mut input_file: String = "".to_string();
        let mut output_file: String = "".to_string();
        let mut field_name = String::new();
        let mut use_field = false;

        // read the arguments
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
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-field" {
                field_name = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                use_field = true;
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

        let start = time::now();

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        if !input_file.contains(path::MAIN_SEPARATOR) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }

        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let input = Shapefile::read(&input_file)?;

        if !use_field {
            match input.header.shape_type.base_shape_type() {
                ShapeType::Point => {
                    // Points cannot handle multipart features. Have to use multipoints instead.

                    // create output file
                    let mut output = match input.header.shape_type.dimension() {
                        ShapeTypeDimension::XY => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPoint,
                            false,
                        )?,
                        ShapeTypeDimension::Measure => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPointM,
                            false,
                        )?,
                        ShapeTypeDimension::Z => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPointZ,
                            false,
                        )?,
                    };

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        3u8,
                        0u8,
                    ));

                    let mut sfg = ShapefileGeometry::new(output.header.shape_type);
                    let mut points: Vec<Point2D> = Vec::with_capacity(input.num_records);
                    let mut measures: Vec<f64> = Vec::with_capacity(input.num_records);
                    let mut z_values: Vec<f64> = Vec::with_capacity(input.num_records);
                    for record_num in 0..input.num_records {
                        let record = input.get_record(record_num);
                        points.push(record.points[0]);
                        if output.header.shape_type.dimension() == ShapeTypeDimension::Measure {
                            measures.push(record.m_array[0]);
                        } else if output.header.shape_type.dimension() == ShapeTypeDimension::Z {
                            measures.push(record.m_array[0]);
                            z_values.push(record.z_array[0]);
                        }

                        if verbose {
                            progress = (100.0_f64 * (record_num + 1) as f64
                                / input.num_records as f64)
                                as usize;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    match output.header.shape_type.dimension() {
                        ShapeTypeDimension::XY => {
                            sfg.add_part(&points);
                        }
                        ShapeTypeDimension::Measure => {
                            sfg.add_partm(&points, &measures);
                        }
                        ShapeTypeDimension::Z => {
                            sfg.add_partz(&points, &measures, &z_values);
                        }
                    }

                    output.add_record(sfg);
                    output
                        .attributes
                        .add_record(vec![FieldData::Int(1i32)], false);

                    if verbose {
                        println!("Saving data...")
                    };
                    let _ = match output.write() {
                        Ok(_) => if verbose {
                            println!("Output file written")
                        },
                        Err(e) => return Err(e),
                    };
                }
                ShapeType::PolyLine => {
                    let mut output = Shapefile::initialize_using_file(
                        &output_file,
                        &input,
                        input.header.shape_type,
                        false,
                    )?;

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        1u8,
                        0u8,
                    ));

                    let mut sfg = ShapefileGeometry::new(output.header.shape_type);

                    for record_num in 0..input.num_records {
                        let record = input.get_record(record_num);

                        match output.header.shape_type.dimension() {
                            ShapeTypeDimension::XY => {
                                sfg.add_part(&record.points.clone());
                            }
                            ShapeTypeDimension::Measure => {
                                sfg.add_partm(&record.points.clone(), &record.m_array.clone());
                            }
                            ShapeTypeDimension::Z => {
                                sfg.add_partz(
                                    &record.points.clone(),
                                    &record.m_array.clone(),
                                    &record.z_array.clone(),
                                );
                            }
                        }

                        if verbose {
                            progress = (100.0_f64 * (record_num + 1) as f64
                                / input.num_records as f64)
                                as usize;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    output.add_record(sfg);
                    output
                        .attributes
                        .add_record(vec![FieldData::Int(1i32)], false);

                    if verbose {
                        println!("Saving data...")
                    };
                    let _ = match output.write() {
                        Ok(_) => if verbose {
                            println!("Output file written")
                        },
                        Err(e) => return Err(e),
                    };
                }
                ShapeType::Polygon => {
                    let mut output = Shapefile::initialize_using_file(
                        &output_file,
                        &input,
                        input.header.shape_type,
                        false,
                    )?;

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        3u8,
                        0u8,
                    ));

                    // polygons contained within other polygons will be considered holes
                    let mut is_contained = vec![false; input.num_records];
                    for record_num in 0..input.num_records {
                        let record1 = input.get_record(record_num);
                        for i in 0..input.num_records {
                            if i != record_num {
                                let record2 = input.get_record(i);
                                if poly_in_poly(&(record1.points), &(record2.points)) {
                                    is_contained[record_num] = true;
                                    break;
                                }
                            }
                        }
                    }

                    let mut sfg = ShapefileGeometry::new(output.header.shape_type);

                    for record_num in 0..input.num_records {
                        let record = input.get_record(record_num);
                        let mut points = record.points.clone();
                        let mut measures: Vec<f64> = Vec::with_capacity(record.num_points as usize);
                        let mut z_values: Vec<f64> = Vec::with_capacity(record.num_points as usize);
                        let mut reverse_pnts = false;
                        if (is_contained[record_num] && is_clockwise_order(&points))
                            || (!is_contained[record_num] && !is_clockwise_order(&points))
                        {
                            points.reverse();
                            reverse_pnts = true;
                        }
                        if output.header.shape_type.dimension() == ShapeTypeDimension::Measure {
                            measures = record.m_array.clone();
                            if reverse_pnts {
                                measures.reverse();
                            }
                        } else if output.header.shape_type.dimension() == ShapeTypeDimension::Z {
                            measures = record.m_array.clone();
                            if reverse_pnts {
                                measures.reverse();
                            }

                            z_values = record.z_array.clone();
                            if reverse_pnts {
                                z_values.reverse();
                            }
                        }

                        match output.header.shape_type.dimension() {
                            ShapeTypeDimension::XY => {
                                sfg.add_part(&points);
                            }
                            ShapeTypeDimension::Measure => {
                                sfg.add_partm(&points, &measures);
                            }
                            ShapeTypeDimension::Z => {
                                sfg.add_partz(&points, &measures, &z_values);
                            }
                        }

                        if verbose {
                            progress = (100.0_f64 * (record_num + 1) as f64
                                / input.num_records as f64)
                                as usize;
                            if progress != old_progress {
                                println!("Progress: {}%", progress);
                                old_progress = progress;
                            }
                        }
                    }

                    output.add_record(sfg);
                    output
                        .attributes
                        .add_record(vec![FieldData::Int(1i32)], false);

                    if verbose {
                        println!("Saving data...")
                    };
                    let _ = match output.write() {
                        Ok(_) => if verbose {
                            println!("Output file written")
                        },
                        Err(e) => return Err(e),
                    };
                }
                _ => {
                    // Null and Multipoint
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The input ShapeType cannot be represented as a multipart geometry",
                    ));
                }
            }
        } else {
            // What is the index of the field to be analyzed?
            let field_index = match input.attributes.get_field_num(&field_name) {
                Some(i) => i,
                None => {
                    // Field not found
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Attribute not found in table.",
                    ));
                }
            };

            // Is the field numeric?
            if !input.attributes.is_field_numeric(field_index) {
                if input.attributes.fields[field_index].decimal_count > 0 {
                    println!(
                        "WARNING: The attribute field does not appear to be categorical. This may produce unexpected results."
                    )
                }
            }

            match input.header.shape_type.base_shape_type() {
                ShapeType::Point => {
                    // Points cannot handle multipart features. Have to use multipoints instead.

                    // create output file
                    let mut output = match input.header.shape_type.dimension() {
                        ShapeTypeDimension::XY => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPoint,
                            false,
                        )?,
                        ShapeTypeDimension::Measure => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPointM,
                            false,
                        )?,
                        ShapeTypeDimension::Z => Shapefile::initialize_using_file(
                            &output_file,
                            &input,
                            ShapeType::MultiPointZ,
                            false,
                        )?,
                    };

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        7u8,
                        0u8,
                    ));

                    output
                        .attributes
                        .add_field(&(input.attributes.get_field(field_index).clone()));

                    let mut feature_num = vec![input.num_records + 1; input.num_records];
                    let mut id = 0usize;
                    for record_num in 0..input.num_records {
                        if feature_num[record_num] == input.num_records + 1 {
                            feature_num[record_num] = id;
                            let record1 = input.attributes.get_value(record_num, &field_name);
                            for record_num2 in record_num..input.num_records {
                                let record2 = input.attributes.get_value(record_num2, &field_name);
                                if record2 == record1 {
                                    feature_num[record_num2] = id;
                                }
                            }
                            id += 1;
                        }
                    }

                    let mut max_id = id - 1;

                    for id in 0..=max_id {
                        let mut sfg = ShapefileGeometry::new(output.header.shape_type);
                        let mut i = input.num_records;
                        for record_num in 0..input.num_records {
                            if feature_num[record_num] == id {
                                i = record_num;
                                let record = input.get_record(record_num);
                                let mut points = record.points.clone();
                                let mut measures: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);
                                let mut z_values: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);

                                if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Measure
                                {
                                    measures = record.m_array.clone();
                                } else if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Z
                                {
                                    measures = record.m_array.clone();

                                    z_values = record.z_array.clone();
                                }

                                match output.header.shape_type.dimension() {
                                    ShapeTypeDimension::XY => {
                                        sfg.add_part(&points);
                                    }
                                    ShapeTypeDimension::Measure => {
                                        sfg.add_partm(&points, &measures);
                                    }
                                    ShapeTypeDimension::Z => {
                                        sfg.add_partz(&points, &measures, &z_values);
                                    }
                                }
                            }
                        }

                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(id as i32 + 1i32),
                                input.attributes.get_value(i, &field_name).clone(),
                            ],
                            false,
                        );

                        if verbose {
                            progress = (100.0_f64 * (id + 1) as f64 / max_id as f64) as usize;
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
                }
                ShapeType::PolyLine => {
                    let mut output = Shapefile::initialize_using_file(
                        &output_file,
                        &input,
                        input.header.shape_type,
                        false,
                    )?;

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        7u8,
                        0u8,
                    ));

                    output
                        .attributes
                        .add_field(&(input.attributes.get_field(field_index).clone()));

                    let mut feature_num = vec![input.num_records + 1; input.num_records];
                    let mut id = 0usize;
                    for record_num in 0..input.num_records {
                        if feature_num[record_num] == input.num_records + 1 {
                            feature_num[record_num] = id;
                            let record1 = input.attributes.get_value(record_num, &field_name);
                            for record_num2 in record_num..input.num_records {
                                let record2 = input.attributes.get_value(record_num2, &field_name);
                                if record2 == record1 {
                                    feature_num[record_num2] = id;
                                }
                            }
                            id += 1;
                        }
                    }

                    let mut max_id = id - 1;

                    for id in 0..=max_id {
                        let mut sfg = ShapefileGeometry::new(output.header.shape_type);
                        let mut i = input.num_records;
                        for record_num in 0..input.num_records {
                            if feature_num[record_num] == id {
                                i = record_num;
                                let record = input.get_record(record_num);
                                let mut points = record.points.clone();
                                let mut measures: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);
                                let mut z_values: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);

                                if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Measure
                                {
                                    measures = record.m_array.clone();
                                } else if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Z
                                {
                                    measures = record.m_array.clone();

                                    z_values = record.z_array.clone();
                                }

                                match output.header.shape_type.dimension() {
                                    ShapeTypeDimension::XY => {
                                        sfg.add_part(&points);
                                    }
                                    ShapeTypeDimension::Measure => {
                                        sfg.add_partm(&points, &measures);
                                    }
                                    ShapeTypeDimension::Z => {
                                        sfg.add_partz(&points, &measures, &z_values);
                                    }
                                }
                            }
                        }

                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(id as i32 + 1i32),
                                input.attributes.get_value(i, &field_name).clone(),
                            ],
                            false,
                        );

                        if verbose {
                            progress = (100.0_f64 * (id + 1) as f64 / max_id as f64) as usize;
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
                }
                ShapeType::Polygon => {
                    let mut output = Shapefile::initialize_using_file(
                        &output_file,
                        &input,
                        input.header.shape_type,
                        false,
                    )?;

                    // add the attributes
                    output.attributes.add_field(&AttributeField::new(
                        "FID",
                        FieldDataType::Int,
                        7u8,
                        0u8,
                    ));

                    output
                        .attributes
                        .add_field(&(input.attributes.get_field(field_index).clone()));

                    let mut feature_num = vec![input.num_records + 1; input.num_records];
                    let mut id = 0usize;
                    for record_num in 0..input.num_records {
                        if feature_num[record_num] == input.num_records + 1 {
                            feature_num[record_num] = id;
                            let record1 = input.attributes.get_value(record_num, &field_name);
                            for record_num2 in record_num..input.num_records {
                                let record2 = input.attributes.get_value(record_num2, &field_name);
                                if record2 == record1 {
                                    feature_num[record_num2] = id;
                                }
                            }
                            id += 1;
                        }
                    }

                    let mut max_id = id - 1;

                    // polygons contained within other polygons with the same field id will be considered holes
                    let mut is_contained = vec![false; input.num_records];
                    for record_num in 0..input.num_records {
                        let record1 = input.get_record(record_num);
                        for record_num2 in 0..input.num_records {
                            if record_num2 != record_num
                                && feature_num[record_num] == feature_num[record_num2]
                            {
                                let record2 = input.get_record(record_num2);
                                if poly_in_poly(&(record1.points), &(record2.points)) {
                                    is_contained[record_num] = true;
                                    break;
                                }
                            }
                        }
                    }

                    for id in 0..=max_id {
                        let mut sfg = ShapefileGeometry::new(output.header.shape_type);
                        let mut i = input.num_records;
                        for record_num in 0..input.num_records {
                            if feature_num[record_num] == id {
                                i = record_num;
                                let record = input.get_record(record_num);
                                let mut points = record.points.clone();
                                let mut measures: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);
                                let mut z_values: Vec<f64> =
                                    Vec::with_capacity(record.num_points as usize);
                                let mut reverse_pnts = false;
                                if (is_contained[record_num] && is_clockwise_order(&points))
                                    || (!is_contained[record_num] && !is_clockwise_order(&points))
                                {
                                    points.reverse();
                                    reverse_pnts = true;
                                }
                                if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Measure
                                {
                                    measures = record.m_array.clone();
                                    if reverse_pnts {
                                        measures.reverse();
                                    }
                                } else if output.header.shape_type.dimension()
                                    == ShapeTypeDimension::Z
                                {
                                    measures = record.m_array.clone();
                                    if reverse_pnts {
                                        measures.reverse();
                                    }

                                    z_values = record.z_array.clone();
                                    if reverse_pnts {
                                        z_values.reverse();
                                    }
                                }

                                match output.header.shape_type.dimension() {
                                    ShapeTypeDimension::XY => {
                                        sfg.add_part(&points);
                                    }
                                    ShapeTypeDimension::Measure => {
                                        sfg.add_partm(&points, &measures);
                                    }
                                    ShapeTypeDimension::Z => {
                                        sfg.add_partz(&points, &measures, &z_values);
                                    }
                                }
                            }
                        }

                        output.add_record(sfg);
                        output.attributes.add_record(
                            vec![
                                FieldData::Int(id as i32 + 1i32),
                                input.attributes.get_value(i, &field_name).clone(),
                            ],
                            false,
                        );

                        if verbose {
                            progress = (100.0_f64 * (id + 1) as f64 / max_id as f64) as usize;
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
                }
                _ => {
                    // Null and Multipoint
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "The input ShapeType cannot be represented as a multipart geometry",
                    ));
                }
            }
        }

        let end = time::now();
        let elapsed_time = end - start;

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time: {}", elapsed_time).replace("PT", "")
            );
        }

        Ok(())
    }
}
