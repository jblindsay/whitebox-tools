/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Simon Gudim and Dr. John Lindsay
Created: 19/12/2019
Last Modified: 10/01/2020
License: MIT
*/

use crate::tools::*;
use whitebox_vector::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use statrs::distribution::{StudentsT, Univariate};
use std::cmp::Ordering::Equal;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;

/// This tool can be used to perform nieghbourhood-based (i.e. using roving search windows applied to each
/// grid cell) correlation analysis on two continuous attributes (`--field1` and `--field2`) of an input vector
/// (`--input`). The tool outputs correlation value and a significance (p-value) fields (`CORREL` and `PVALUE`) to
/// the input vector's attribute table. Additionally,the user must specify the size of the search window (`--filter`)
/// and the correlation statistic (`--stat`). Options for the correlation statistic include
/// [`pearson`](https://en.wikipedia.org/wiki/Pearson_correlation_coefficient),
/// [`kendall`](https://en.wikipedia.org/wiki/Kendall_rank_correlation_coefficient), and
/// [`spearman`](https://en.wikipedia.org/wiki/Spearman%27s_rank_correlation_coefficient). Notice that Pearson's *r* is the
/// most computationally efficient of the three correlation metrics but is unsuitable when the input distributions are
/// non-linearly associated, in which case, either Spearman's Rho or Kendall's tau-b correlations are more suited.
/// Both Spearman and Kendall correlations evaluate monotonic associations without assuming linearity in the relation.
/// Kendall's tau-b is by far the most computationally expensive of the three statistics and may not be suitable to
/// larger sized search windows.
///
/// # See Also
/// `AttributeCorrelation`, `ImageCorrelationNeighbourhoodAnalysis`
pub struct AttributeCorrelationNeighbourhoodAnalysis {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AttributeCorrelationNeighbourhoodAnalysis {
    pub fn new() -> AttributeCorrelationNeighbourhoodAnalysis {
        // public constructor
        let name = "AttributeCorrelationNeighbourhoodAnalysis".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Performs a correlation on two input vector attributes within a neighbourhood search windows.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Any,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name 1".to_owned(),
            flags: vec!["--field1".to_owned()],
            description: "First input field name (dependent variable) in attribute table."
                .to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name 2".to_owned(),
            flags: vec!["--field2".to_owned()],
            description: "Second input field name (independent variable) in attribute table."
                .to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Search Radius (map units)".to_owned(),
            flags: vec!["--radius".to_owned()],
            description: "Search Radius (in map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Min. Number of Points".to_owned(),
            flags: vec!["--min_points".to_owned()],
            description: "Minimum number of points.".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Correlation Statistic Type".to_owned(),
            flags: vec!["--stat".to_owned()],
            description: "Correlation type; one of 'pearson' (default) and 'spearman'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "pearson".to_owned(),
                "kendall".to_owned(),
                "spearman".to_owned(),
            ]),
            default_value: Some("pearson".to_owned()),
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=input.shp --field1=DEPEND --field2=INDEPEND --radius=4.0 --min_points=3 --stat=\"spearman\"",
                            short_exe,
                            name)
                .replace("*", &sep);

        AttributeCorrelationNeighbourhoodAnalysis {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AttributeCorrelationNeighbourhoodAnalysis {
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
        let mut field_name1 = String::new();
        let mut field_name2 = String::new();
        let mut radius = 0f64;
        let mut min_points = 0usize;
        let mut stat_type = String::from("pearson");

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
            } else if flag_val == "-field1" {
                field_name1 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-field2" {
                field_name2 = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-radius" {
                radius = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                radius *= radius; // the K-D tree structure actually needs the squared-radius because squared distances are used.
            } else if flag_val == "-min_points" {
                min_points = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val)) as usize
                };
            } else if flag_val == "-stat" {
                let val = if keyval {
                    vec[1].to_lowercase()
                } else {
                    args[i + 1].to_lowercase()
                };
                stat_type = if val.contains("son") {
                    "pearson".to_string()
                } else if val.contains("kendall") {
                    "kendall".to_string()
                } else {
                    "spearman".to_string()
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
            println!("Reading data...")
        };

        let mut input = Shapefile::read(&input_file)?;
        input.file_mode = "rw".to_string(); // we need to be able to modify the attributes table
        let num_records = input.num_records;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if input.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let mut points = vec![];
        let mut attr1_values = vec![];
        let mut attr2_values = vec![];
        let mut z: f64;
        const DIMENSIONS: usize = 2;
        const CAPACITY_PER_NODE: usize = 64;
        let mut tree = KdTree::with_capacity(DIMENSIONS, CAPACITY_PER_NODE);
        let mut p = 0;
        for record_num in 0..num_records {
            let record = input.get_record(record_num);
            if record.shape_type != ShapeType::Null {
                for i in 0..record.num_points as usize {
                    z = match input.attributes.get_value(record_num, &field_name1) {
                        FieldData::Int(val) => val as f64,
                        FieldData::Real(val) => val,
                        _ => {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "Error: Only vector fields of Int and Real data type may be used as inputs.",
                            ));
                        }
                    };
                    attr1_values.push(z);

                    z = match input.attributes.get_value(record_num, &field_name2) {
                        FieldData::Int(val) => val as f64,
                        FieldData::Real(val) => val,
                        _ => {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "Error: Only vector fields of Int and Real data type may be used as inputs.",
                            ));
                        }
                    };
                    attr2_values.push(z);
                    points.push((record.points[i].x, record.points[i].y));
                    tree.add([record.points[i].x, record.points[i].y], p)
                        .unwrap();
                    p += 1;
                }
            }

            if verbose {
                progress = (100.0_f64 * (record_num + 1) as f64 / num_records as f64) as usize;
                if progress != old_progress {
                    println!("Reading points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        input.attributes.add_field(&AttributeField::new(
            "CORREL",
            FieldDataType::Real,
            12u8,
            6u8,
        ));

        input.attributes.add_field(&AttributeField::new(
            "PVALUE",
            FieldDataType::Real,
            12u8,
            6u8,
        ));

        let (mut x, mut y): (f64, f64);
        for record_num in 0..points.len() {
            x = points[record_num].0;
            y = points[record_num].1;
            let mut ret = tree.within(&[x, y], radius, &squared_euclidean).unwrap();
            if ret.len() < min_points {
                ret = tree
                    .nearest(&[x, y], min_points, &squared_euclidean)
                    .unwrap();
            }
            if ret.len() > 0 {
                if stat_type == "pearson" {
                    // let (mut z1, mut z2): (f64, f64);
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let mut point_num: usize;

                    let mut num_vals = 0;
                    let mut sum1 = 0f64;
                    let mut sum2 = 0f64;
                    for k in 0..ret.len() {
                        point_num = *(ret[k].1);
                        z_n1 = attr1_values[point_num];
                        z_n2 = attr2_values[point_num];
                        sum1 += z_n1;
                        sum2 += z_n2;
                        num_vals += 1;
                    }
                    let mean1 = sum1 / num_vals as f64;
                    let mean2 = sum2 / num_vals as f64;

                    // Now calculate the total deviations and total cross-deviation.
                    let mut total_deviation1 = 0f64;
                    let mut total_deviation2 = 0f64;
                    let mut product_deviations = 0f64;
                    if num_vals > 2 {
                        for k in 0..ret.len() {
                            point_num = *(ret[k].1);
                            z_n1 = attr1_values[point_num];
                            z_n2 = attr2_values[point_num];
                            total_deviation1 += (z_n1 - mean1) * (z_n1 - mean1);
                            total_deviation2 += (z_n2 - mean2) * (z_n2 - mean2);
                            product_deviations += (z_n1 - mean1) * (z_n2 - mean2);
                        }
                    }

                    // Finally, calculate r for the neighbourhood.
                    let r = if total_deviation1 != 0f64 && total_deviation2 != 0f64 && num_vals > 2
                    {
                        product_deviations / (total_deviation1 * total_deviation2).sqrt()
                    } else {
                        // You can't divide by zero
                        0f64
                    };
                    input
                        .attributes
                        .set_value(record_num, "CORREL", FieldData::Real(r));

                    let df = num_vals - 2;
                    let pvalue = if df > 2 {
                        let tvalue = r * (df as f64 / (1f64 - r * r)).sqrt();
                        let t = StudentsT::new(0.0, 1.0, df as f64).unwrap();
                        2f64 * (1f64 - t.cdf(tvalue.abs()))
                    } else {
                        0f64
                    };
                    input
                        .attributes
                        .set_value(record_num, "PVALUE", FieldData::Real(pvalue));
                } else if stat_type == "kendall" {
                    // Perform Kendall's Tau-b correlation
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let mut rank2: f64;
                    let mut upper_range: usize;
                    let mut point_num: usize;
                    let mut num_tied_vals: f64;
                    let mut v1 = Vec::with_capacity(ret.len());
                    let mut v2 = Vec::with_capacity(ret.len());

                    let mut num_vals = 0;
                    for p in ret {
                        point_num = *(p.1);
                        z_n1 = attr1_values[point_num];
                        z_n2 = attr2_values[point_num];
                        num_vals += 1;
                        // tuple = (value, index, rank)
                        v1.push((z_n1, num_vals, 0f64));
                        v2.push((z_n2, num_vals, 0f64));
                    }
                    let num_vals_f64 = num_vals as f64;

                    // Sort both lists based on value
                    v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
                    v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));

                    let mut rank = 0f64;
                    let mut nt1 = 0f64;
                    for i in 0..num_vals {
                        if v1[i].2 == 0f64 {
                            rank += 1f64;
                            if i < num_vals - 1 {
                                // are there any ties above this one?
                                upper_range = i;
                                for j in i + 1..num_vals {
                                    if v1[i].0 == v1[j].0 {
                                        upper_range = j;
                                    } else {
                                        break;
                                    }
                                }
                                if upper_range != i {
                                    num_tied_vals = (upper_range - i + 1) as f64;
                                    nt1 += num_tied_vals * (num_tied_vals - 1f64) / 2f64;
                                    rank2 = rank + (upper_range - i) as f64;
                                    rank = (rank + rank2) / 2f64; // average rank
                                    for k in i..=upper_range {
                                        v1[k].2 = rank;
                                    }
                                    rank = rank2;
                                } else {
                                    v1[i].2 = rank;
                                }
                            } else {
                                v1[i].2 = rank;
                            }
                        }
                    }

                    let mut nt2 = 0f64;
                    rank = 0f64;
                    for i in 0..num_vals {
                        if v2[i].2 == 0f64 {
                            rank += 1f64;
                            if i < num_vals - 1 {
                                // are there any ties above this one?
                                upper_range = i;
                                for j in i + 1..num_vals {
                                    if v2[i].0 == v2[j].0 {
                                        upper_range = j;
                                    } else {
                                        break;
                                    }
                                }
                                if upper_range != i {
                                    num_tied_vals = (upper_range - i + 1) as f64;
                                    nt2 += num_tied_vals * (num_tied_vals - 1f64) / 2f64;
                                    rank2 = rank + (upper_range - i) as f64;
                                    rank = (rank + rank2) / 2f64; // average rank
                                    for k in i..=upper_range {
                                        v2[k].2 = rank;
                                    }
                                    rank = rank2;
                                } else {
                                    v2[i].2 = rank;
                                }
                            } else {
                                v2[i].2 = rank;
                            }
                        }
                    }

                    // Sort both lists based on index
                    v1.sort_by(|a, b| a.1.cmp(&b.1));
                    v2.sort_by(|a, b| a.1.cmp(&b.1));

                    ////////////////////////////////////////////////////////////////////////////
                    // This block of code is O(n^2) and is a serious performance killer. There
                    // is a O(nlogn) solution based on swaps in a merge-sort but I have yet to
                    // figure it out. As it stands, this solution is unacceptable for search
                    // windows larger than about 25, depending the number of cores in the
                    // system processor.
                    ////////////////////////////////////////////////////////////////////////////
                    let mut numer = 0f64;
                    for i in 0..num_vals {
                        for j in i + 1..num_vals {
                            if v1[i].2 != v1[j].2 && v2[i].2 != v2[j].2 {
                                numer +=
                                    (v1[i].2 - v1[j].2).signum() * (v2[i].2 - v2[j].2).signum();
                            }
                        }
                    }

                    let n0 = num_vals as f64 * (num_vals as f64 - 1f64) / 2f64;
                    let tau = numer / ((n0 - nt1) * (n0 - nt2)).sqrt();
                    input
                        .attributes
                        .set_value(record_num, "CORREL", FieldData::Real(tau));

                    let df = num_vals_f64 - 2f64;
                    let pvalue = if df > 2f64 {
                        let zvalue = 3f64 * numer
                            / (num_vals_f64 * (num_vals_f64 - 1f64) * (2f64 * num_vals_f64 + 5f64)
                                / 2f64)
                                .sqrt();
                        let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
                        2f64 * (1f64 - t.cdf(zvalue.abs()))
                    } else {
                        0f64
                    };
                    input
                        .attributes
                        .set_value(record_num, "PVALUE", FieldData::Real(pvalue));
                } else {
                    // Calculate Spearman's Rho correlation
                    let (mut z_n1, mut z_n2): (f64, f64);
                    let mut rank2: f64;
                    let mut upper_range: usize;
                    let mut point_num: usize;
                    let mut v1 = Vec::with_capacity(ret.len());
                    let mut v2 = Vec::with_capacity(ret.len());

                    let mut num_vals = 0;
                    for p in ret {
                        point_num = *(p.1);
                        z_n1 = attr1_values[point_num];
                        z_n2 = attr2_values[point_num];
                        num_vals += 1;
                        // tuple = (value, index, rank)
                        v1.push((z_n1, num_vals, 0f64));
                        v2.push((z_n2, num_vals, 0f64));
                    }
                    let num_vals_f64 = num_vals as f64;

                    // Sort both lists based on value
                    v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
                    v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));

                    let mut rank = 0f64;
                    for i in 0..num_vals {
                        if v1[i].2 == 0f64 {
                            rank += 1f64;
                            if i < num_vals - 1 {
                                // are there any ties above this one?
                                upper_range = i;
                                for j in i + 1..num_vals {
                                    if v1[i].0 == v1[j].0 {
                                        upper_range = j;
                                    } else {
                                        break;
                                    }
                                }
                                if upper_range != i {
                                    rank2 = rank + (upper_range - i) as f64;
                                    rank = (rank + rank2) / 2f64; // average rank
                                    for k in i..=upper_range {
                                        v1[k].2 = rank;
                                    }
                                    rank = rank2;
                                } else {
                                    v1[i].2 = rank;
                                }
                            } else {
                                v1[i].2 = rank;
                            }
                        }
                    }

                    rank = 0f64;
                    for i in 0..num_vals {
                        if v2[i].2 == 0f64 {
                            rank += 1f64;
                            if i < num_vals - 1 {
                                // are there any ties above this one?
                                upper_range = i;
                                for j in i + 1..num_vals {
                                    if v2[i].0 == v2[j].0 {
                                        upper_range = j;
                                    } else {
                                        break;
                                    }
                                }
                                if upper_range != i {
                                    rank2 = rank + (upper_range - i) as f64;
                                    rank = (rank + rank2) / 2f64; // average rank
                                    for k in i..=upper_range {
                                        v2[k].2 = rank;
                                    }
                                    rank = rank2;
                                } else {
                                    v2[i].2 = rank;
                                }
                            } else {
                                v2[i].2 = rank;
                            }
                        }
                    }

                    // Sort both lists based on index
                    v1.sort_by(|a, b| a.1.cmp(&b.1));
                    v2.sort_by(|a, b| a.1.cmp(&b.1));

                    let mut rank_diff_sqrd = 0f64;
                    for i in 0..num_vals {
                        rank_diff_sqrd += (v1[i].2 - v2[i].2) * (v1[i].2 - v2[i].2);
                    }

                    let rho = 1f64
                        - (6f64 * rank_diff_sqrd
                            / (num_vals_f64 * num_vals_f64 * num_vals_f64 - num_vals_f64));
                    input
                        .attributes
                        .set_value(record_num, "CORREL", FieldData::Real(rho));

                    let df = num_vals_f64 - 2f64; // calculate degrees of freedom (Anthony Comment)
                    let pvalue = if df > 2f64 {
                        let tvalue = rho * (df / (1f64 - rho * rho)).sqrt();
                        let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
                        2f64 * (1f64 - t.cdf(tvalue.abs()))
                    } else {
                        0f64
                    };
                    input
                        .attributes
                        .set_value(record_num, "PVALUE", FieldData::Real(pvalue));
                }
            }
            if verbose {
                progress = (100.0_f64 * (record_num + 1) as f64 / points.len() as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!("Saving data...")
        };
        let _ = match input.write() {
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

        // } else if stat_type == "kendall" { // Perform Kendall's Tau-b correlation
        //     let (tx, rx) = mpsc::channel();
        //     for tid in 0..num_procs {
        //         let image1 = image1.clone();
        //         let image2 = image2.clone();
        //         let tx = tx.clone();
        //         thread::spawn(move || {
        //             let mut num_cells: usize;
        //             let mut num_cells_f64: f64;
        //             let mut tau: f64;
        //             let mut df: f64;
        //             let mut zvalue: f64;
        //             let mut pvalue: f64;
        //             let (mut z1, mut z2): (f64, f64);
        //             let (mut z_n1, mut z_n2): (f64, f64);
        //             let num_pixels_in_filter = filter_size * filter_size;
        //             let mut dx = vec![0isize; num_pixels_in_filter];
        //             let mut dy = vec![0isize; num_pixels_in_filter];
        //             let (mut rank, mut rank2): (f64, f64);
        //             let mut upper_range: usize;

        //             let mut num_tied_vals: f64;
        //             let mut nt1: f64;
        //             let mut nt2: f64;
        //             let mut n0: f64;
        //             let mut numer: f64;

        //             let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        //             let mut a = 0;
        //             for row in 0..filter_size {
        //                 for col in 0..filter_size {
        //                     dx[a] = col as isize - midpoint;
        //                     dy[a] = row as isize - midpoint;
        //                     a += 1;
        //                 }
        //             }

        //             for row in (0..rows).filter(|r| r % num_procs == tid) {
        //                 let mut data1 = vec![nodata1; columns as usize];
        //                 let mut data2 = vec![nodata1; columns as usize];
        //                 for col in 0..columns {
        //                     z1 = image1.get_value(row, col);
        //                     z2 = image2.get_value(row, col);
        //                     if z1 != nodata1 && z2 != nodata2 {
        //                         let mut v1 = Vec::with_capacity(num_pixels_in_filter);
        //                         let mut v2 = Vec::with_capacity(num_pixels_in_filter);
        //                         num_cells = 0;
        //                         for i in 0..num_pixels_in_filter {
        //                             z_n1 = image1.get_value(row + dy[i], col + dx[i]);
        //                             z_n2 = image2.get_value(row + dy[i], col + dx[i]);
        //                             if z_n1 != nodata1 && z_n2 != nodata2 {
        //                                 num_cells += 1;
        //                                 // tuple = (value, index, rank)
        //                                 v1.push((z_n1, num_cells, 0f64));
        //                                 v2.push((z_n2, num_cells, 0f64));
        //                             }
        //                         }
        //                         num_cells_f64 = num_cells as f64;

        //                         // Sort both lists based on value
        //                         v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
        //                         v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));

        //                         // Now provide the rank data
        //                         rank = 0f64;
        //                         nt1 = 0f64;
        //                         for i in 0..num_cells {
        //                             if v1[i].2 == 0f64 {
        //                                 rank += 1f64;
        //                                 if i < num_cells - 1 {
        //                                     // are there any ties above this one?
        //                                     upper_range = i;
        //                                     for j in i+1..num_cells {
        //                                         if v1[i].0 == v1[j].0 {
        //                                             upper_range = j;
        //                                         } else {
        //                                             break;
        //                                         }
        //                                     }
        //                                     if upper_range != i {
        //                                         num_tied_vals = (upper_range - i + 1) as f64;
        //                                         nt1 += num_tied_vals * (num_tied_vals - 1f64) / 2f64;
        //                                         rank2 = rank + (upper_range - i) as f64;
        //                                         rank = (rank + rank2) / 2f64; // average rank
        //                                         for k in i..=upper_range {
        //                                             v1[k].2 = rank;
        //                                         }
        //                                         rank = rank2;
        //                                     } else {
        //                                         v1[i].2 = rank;
        //                                     }
        //                                 } else {
        //                                     v1[i].2 = rank;
        //                                 }
        //                             }
        //                         }

        //                         nt2 = 0f64;
        //                         rank = 0f64;
        //                         for i in 0..num_cells {
        //                             if v2[i].2 == 0f64 {
        //                                 rank += 1f64;
        //                                 if i < num_cells - 1 {
        //                                     // are there any ties above this one?
        //                                     upper_range = i;
        //                                     for j in i+1..num_cells {
        //                                         if v2[i].0 == v2[j].0 {
        //                                             upper_range = j;
        //                                         } else {
        //                                             break;
        //                                         }
        //                                     }
        //                                     if upper_range != i {
        //                                         num_tied_vals = (upper_range - i + 1) as f64;
        //                                         nt2 += num_tied_vals * (num_tied_vals - 1f64) / 2f64;
        //                                         rank2 = rank + (upper_range - i) as f64;
        //                                         rank = (rank + rank2) / 2f64; // average rank
        //                                         for k in i..=upper_range {
        //                                             v2[k].2 = rank;
        //                                         }
        //                                         rank = rank2;
        //                                     } else {
        //                                         v2[i].2 = rank;
        //                                     }
        //                                 } else {
        //                                     v2[i].2 = rank;
        //                                 }
        //                             }
        //                         }

        //                         // Sort both lists based on index
        //                         v1.sort_by(|a, b| a.1.cmp(&b.1));
        //                         v2.sort_by(|a, b| a.1.cmp(&b.1));

        //                         ////////////////////////////////////////////////////////////////////////////
        //                         // This block of code is O(n^2) and is a serious performance killer. There
        //                         // is a O(nlogn) solution based on swaps in a merge-sort but I have yet to
        //                         // figure it out. As it stands, this solution is unacceptable for search
        //                         // windows larger than about 25, depending the number of cores in the
        //                         // system processor.
        //                         ////////////////////////////////////////////////////////////////////////////
        //                         numer = 0f64;
        //                         for i in 0..num_cells {
        //                             for j in i+1..num_cells {
        //                                 if v1[i].2 != v1[j].2 && v2[i].2 != v2[j].2 {
        //                                     numer += (v1[i].2 - v1[j].2).signum() * (v2[i].2 - v2[j].2).signum();
        //                                 }
        //                             }
        //                         }

        //                         n0 = num_cells as f64 * (num_cells as f64 - 1f64) / 2f64;
        //                         tau = numer / ((n0 - nt1)*(n0 - nt2)).sqrt();
        //                         data1[col as usize] = tau;
        //                         df = num_cells_f64 - 2f64;

        //                         if df > 2f64 {
        //                             zvalue = 3f64 * numer / (num_cells_f64*(num_cells_f64-1f64)*(2f64*num_cells_f64+5f64) / 2f64).sqrt();
        //                             let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
        //                             pvalue =  2f64 * (1f64 - t.cdf(zvalue.abs())); // calculate the p-value (significance)
        //                             data2[col as usize] = pvalue;
        //                         } else {
        //                             data2[col as usize] = 0f64;
        //                         }
        //                     }
        //                 }
        //                 tx.send((row, data1, data2)).unwrap();
        //             }
        //         });
        //     }

        //     for r in 0..rows {
        //         let (row, data1, data2) = rx.recv().expect("Error receiving data from thread.");
        //         output_val.set_row_data(row, data1);
        //         output_sig.set_row_data(row, data2);

        //         if verbose {
        //             progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Performing Correlation: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }
        // } else { // Calculate Spearman's Rho correlation
        //     let (tx, rx) = mpsc::channel();
        //     for tid in 0..num_procs {
        //         let image1 = image1.clone();
        //         let image2 = image2.clone();
        //         let tx = tx.clone();
        //         thread::spawn(move || {
        //             let mut num_cells: usize;
        //             let mut num_cells_f64: f64;
        //             let mut rho: f64;
        //             let mut df: f64;
        //             let mut tvalue: f64;
        //             let mut pvalue: f64;
        //             let (mut z1, mut z2): (f64, f64);
        //             let (mut z_n1, mut z_n2): (f64, f64);
        //             let num_pixels_in_filter = filter_size * filter_size;
        //             let mut dx = vec![0isize; num_pixels_in_filter];
        //             let mut dy = vec![0isize; num_pixels_in_filter];
        //             let (mut rank, mut rank2): (f64, f64);
        //             let mut upper_range: usize;
        //             let mut num_ties = 0;
        //             let mut num_ties_test: isize;
        //             let mut max_num_ties: isize;
        //             let midpoint: isize = (filter_size as f64 / 2f64).floor() as isize; // + 1;
        //             let mut a = 0;
        //             for row in 0..filter_size {
        //                 for col in 0..filter_size {
        //                     dx[a] = col as isize - midpoint;
        //                     dy[a] = row as isize - midpoint;
        //                     a += 1;
        //                 }
        //             }

        //             for row in (0..rows).filter(|r| r % num_procs == tid) {
        //                 let mut data1 = vec![nodata1; columns as usize];
        //                 let mut data2 = vec![nodata1; columns as usize];
        //                 max_num_ties = -1;
        //                 for col in 0..columns {
        //                     z1 = image1.get_value(row, col);
        //                     z2 = image2.get_value(row, col);
        //                     if z1 != nodata1 && z2 != nodata2 {
        //                         let mut v1 = Vec::with_capacity(num_pixels_in_filter);
        //                         let mut v2 = Vec::with_capacity(num_pixels_in_filter);
        //                         num_cells = 0;
        //                         for i in 0..num_pixels_in_filter {
        //                             z_n1 = image1.get_value(row + dy[i], col + dx[i]);
        //                             z_n2 = image2.get_value(row + dy[i], col + dx[i]);
        //                             if z_n1 != nodata1 && z_n2 != nodata2 {
        //                                 num_cells += 1;
        //                                 // tuple = (value, index, rank)
        //                                 v1.push((z_n1, num_cells, 0f64));
        //                                 v2.push((z_n2, num_cells, 0f64));
        //                             }
        //                         }
        //                         num_cells_f64 = num_cells as f64;

        //                         // Sort both lists based on value
        //                         v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
        //                         v2.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));
        //                         num_ties_test = 0;
        //                         rank = 0f64;
        //                         for i in 0..num_cells {
        //                             if v1[i].2 == 0f64 {
        //                                 rank += 1f64;
        //                                 if i < num_cells - 1 {
        //                                     // are there any ties above this one?
        //                                     upper_range = i;
        //                                     for j in i+1..num_cells {
        //                                         if v1[i].0 == v1[j].0 {
        //                                             upper_range = j;
        //                                             num_ties += 1;
        //                                             num_ties_test += 1;
        //                                         } else {
        //                                             break;
        //                                         }
        //                                     }
        //                                     if upper_range != i {
        //                                         rank2 = rank + (upper_range - i) as f64;
        //                                         rank = (rank + rank2) / 2f64; // average rank
        //                                         for k in i..=upper_range {
        //                                             v1[k].2 = rank;
        //                                         }
        //                                         rank = rank2;
        //                                     } else {
        //                                         v1[i].2 = rank;
        //                                     }
        //                                 } else {
        //                                     v1[i].2 = rank;
        //                                 }
        //                             }
        //                         }

        //                         rank = 0f64;
        //                         for i in 0..num_cells {
        //                             if v2[i].2 == 0f64 {
        //                                 rank += 1f64;
        //                                 if i < num_cells - 1 {
        //                                     // are there any ties above this one?
        //                                     upper_range = i;
        //                                     for j in i+1..num_cells {
        //                                         if v2[i].0 == v2[j].0 {
        //                                             upper_range = j;
        //                                             num_ties += 1;
        //                                             num_ties_test += 1;
        //                                         } else {
        //                                             break;
        //                                         }
        //                                     }
        //                                     if upper_range != i {
        //                                         rank2 = rank + (upper_range - i) as f64;
        //                                         rank = (rank + rank2) / 2f64; // average rank
        //                                         for k in i..=upper_range {
        //                                             v2[k].2 = rank;
        //                                         }
        //                                         rank = rank2;
        //                                     } else {
        //                                         v2[i].2 = rank;
        //                                     }
        //                                 } else {
        //                                     v2[i].2 = rank;
        //                                 }
        //                             }
        //                         }

        //                         // Sort both lists based on index
        //                         v1.sort_by(|a, b| a.1.cmp(&b.1));
        //                         v2.sort_by(|a, b| a.1.cmp(&b.1));

        //                         let mut rank_diff_sqrd = 0f64;
        //                         for i in 0..num_cells {
        //                             rank_diff_sqrd += (v1[i].2 - v2[i].2) * (v1[i].2 - v2[i].2);
        //                         }

        //                         rho = 1f64 - (6f64 * rank_diff_sqrd / (num_cells_f64 * num_cells_f64 * num_cells_f64 - num_cells_f64));
        //                         data1[col as usize] = rho;
        //                         df = num_cells_f64 - 2f64; // calculate degrees of freedom (Anthony Comment)

        //                         if df > 2f64 {
        //                             tvalue = rho * (df / (1f64 - rho * rho)).sqrt();
        //                             let t = StudentsT::new(0.0, 1.0, df as f64).unwrap(); // create a student's t distribution
        //                             pvalue =  2f64 * (1f64 - t.cdf(tvalue.abs())); // calculate the p-value (significance)
        //                             data2[col as usize] = pvalue;
        //                         } else {
        //                             data2[col as usize] = 0f64;
        //                         }

        //                         if max_num_ties < num_ties_test { max_num_ties = num_ties_test; }
        //                     }
        //                 }
        //                 tx.send((row, data1, data2, num_ties, max_num_ties)).unwrap();
        //             }
        //         });
        //     }

        //     let mut max_ties = -1isize;
        //     let mut num_ties = 0;
        //     for r in 0..rows {
        //         let (row, data1, data2, ties, max_row_ties) = rx.recv().expect("Error receiving data from thread.");
        //         output_val.set_row_data(row, data1);
        //         output_sig.set_row_data(row, data2);
        //         num_ties += ties;
        //         if max_row_ties > max_ties { max_ties = max_row_ties; }

        //         if verbose {
        //             progress = (100.0_f64 * r as f64 / (rows - 1) as f64) as usize;
        //             if progress != old_progress {
        //                 println!("Performing Correlation: {}%", progress);
        //                 old_progress = progress;
        //             }
        //         }
        //     }

        //     if num_ties > 0 {
        //         println!("Warning: There was a maximum of {} ties in a test and as a result p-values \nmay be misleading. You may want to consider using Kendall's Tau instead.", max_ties);
        //     }
        // }

        Ok(())
    }
}
