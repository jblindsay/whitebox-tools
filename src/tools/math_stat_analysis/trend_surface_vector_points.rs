/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 01/05/2018
Last Modified: 13/10/2018
License: MIT
*/

use crate::na::{DMatrix, DVector};
use crate::raster::*;
use crate::rendering::html::*;
use crate::tools::*;
use crate::vector::{FieldData, ShapeType, Shapefile};
use std::env;
use std::f64;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::{Error, ErrorKind};
use std::path;
use std::process::Command;

/// This tool can be used to interpolate a trend surface from a vector points file. The
/// technique uses a polynomial, least-squares regression analysis. The user must specify
/// the name of the input shapefile, which must be of a 'Points' base ShapeType and select
/// the attribute in the shapefile's associated attribute table for which to base the trend
/// surface analysis. The attribute must be numerical. In addition, the user must specify
/// the polynomial order (1 to 10) for the analysis. A first-order polynomial is a planar
/// surface with no curvature. As the polynomial order is increased, greater flexibility is
/// allowed in the fitted surface. Although polynomial orders as high as 10 are accepted,
/// numerical instability in the analysis often creates artifacts in trend surfaces of orders
/// greater than 5. The operation will display a text report on completion, in addition to
/// the output raster image. The report will list each of the coefficient values and the
/// r-square value. The Trend Surface tool can be used instead if the input data is a raster
/// image.
///
/// Numerical stability is enhanced by transforming the x, y, z data by their minimum
/// values before performing the regression analysis. These transform parameters
/// are also reported in the output report.
pub struct TrendSurfaceVectorPoints {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TrendSurfaceVectorPoints {
    pub fn new() -> TrendSurfaceVectorPoints {
        // public constructor
        let name = "TrendSurfaceVectorPoints".to_string();
        let toolbox = "Math and Stats Tools".to_string();
        let description = "Estimates a trend surface from vector points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Vector Points File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input vector Points file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Field Name".to_owned(),
            flags: vec!["--field".to_owned()],
            description: "Input field name in attribute table.".to_owned(),
            parameter_type: ParameterType::VectorAttributeField(
                AttributeType::Number,
                "--input".to_string(),
            ),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Polynomial Order".to_owned(),
            flags: vec!["--order".to_owned()],
            description: "Polynomial order (1 to 10).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter{
            name: "Cell Size (optional)".to_owned(), 
            flags: vec!["--cell_size".to_owned()], 
            description: "Optionally specified cell size of output raster. Not used when base raster is specified.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.shp' --field=ELEV  -o='output.tif' --order=2 --cell_size=10.0", short_exe, name).replace("*", &sep);

        TrendSurfaceVectorPoints {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TrendSurfaceVectorPoints {
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
        let mut field_name = String::new();
        let mut output_file = String::new();
        let mut order = 1usize;
        let mut cell_size = 0f64;

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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-order" {
                order = if keyval {
                    vec[1].to_string().parse::<f32>().unwrap() as usize
                } else {
                    args[i + 1].to_string().parse::<f32>().unwrap() as usize
                };
            } else if flag_val == "-cell_size" {
                cell_size = if keyval {
                    vec[1].to_string().parse::<f64>().unwrap()
                } else {
                    args[i + 1].to_string().parse::<f64>().unwrap()
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

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if order < 1 {
            order = 1;
        }
        if order > 10 {
            order = 10;
        }

        if verbose {
            println!("Reading data...")
        };

        let vector_data = Shapefile::read(&input_file)?;

        let start = Instant::now();

        // make sure the input vector file is of points type
        if vector_data.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must either be of point base shape type.",
            ));
        }

        // What is the index of the field to be analyzed?
        let field_index = match vector_data.attributes.get_field_num(&field_name) {
            Some(i) => i,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "ERROR: The input field could not be located within the attribute table.",
                ));
            }
        };

        // Is the field numeric?
        if !vector_data.attributes.is_field_numeric(field_index) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: The input field is non-numeric.",
            ));
        }

        // base the output raster on the cell_size and the
        // extent of the input vector.
        if cell_size <= 0f64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ERROR: The input cell-size must be >= 0.0.",
            ));
        }
        let west: f64 = vector_data.header.x_min;
        let north: f64 = vector_data.header.y_max;
        let rows: isize = (((north - vector_data.header.y_min) / cell_size).ceil()) as isize;
        let columns: isize = (((vector_data.header.x_max - west) / cell_size).ceil()) as isize;
        let south: f64 = north - rows as f64 * cell_size;
        let east = west + columns as f64 * cell_size;
        let nodata = -32768.0f64;

        let mut configs = RasterConfigs {
            ..Default::default()
        };
        configs.rows = rows as usize;
        configs.columns = columns as usize;
        configs.north = north;
        configs.south = south;
        configs.east = east;
        configs.west = west;
        configs.resolution_x = cell_size;
        configs.resolution_y = cell_size;
        configs.nodata = nodata;
        configs.data_type = DataType::F32;
        configs.photometric_interp = PhotometricInterpretation::Continuous;

        let mut output = Raster::initialize_using_config(&output_file, &configs);

        // get the input data
        let num_recs = vector_data.num_records as usize;
        let min_x = vector_data.header.x_min;
        let min_y = vector_data.header.y_min;
        let mut min_z = f64::INFINITY;
        let mut x: Vec<f64> = Vec::with_capacity(num_recs);
        let mut y: Vec<f64> = Vec::with_capacity(num_recs);
        let mut z: Vec<f64> = Vec::with_capacity(num_recs);
        let (mut x_val, mut y_val, mut z_val): (f64, f64, f64);

        for record_num in 0..num_recs {
            let record = vector_data.get_record(record_num);
            x_val = record.points[0].x - min_x;
            y_val = record.points[0].y - min_y;
            z_val = match vector_data.attributes.get_value(record_num, &field_name) {
                FieldData::Int(val) => val as f64,
                // FieldData::Int64(val) => {
                //     val as f64
                // },
                FieldData::Real(val) => val,
                _ => nodata,
            };
            if z_val < min_z {
                min_z = z_val;
            }
            x.push(x_val);
            y.push(y_val);
            z.push(z_val);

            if verbose {
                progress =
                    (100.0_f64 * record_num as f64 / (vector_data.num_records - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Reading attributes: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        for record_num in 0..num_recs {
            z[record_num] -= min_z;
        }

        // calculate the equation
        let n = z.len();

        // How many coefficients are there?
        let mut num_coefficients = 0;
        for j in 0..(order + 1) {
            for _k in 0..(order - j + 1) {
                num_coefficients += 1;
            }
        }

        // Solve the forward transformation equations
        let mut forward_coefficient_matrix = vec![0f64; n * num_coefficients];
        for i in 0..n {
            let mut m = 0;
            for j in 0..(order + 1) {
                for k in 0..(order - j + 1) {
                    forward_coefficient_matrix[i * num_coefficients + m] =
                        x[i].powf(j as f64) * y[i].powf(k as f64);
                    m += 1;
                }
            }
        }

        let coefficients =
            DMatrix::from_row_slice(n, num_coefficients, &forward_coefficient_matrix);
        let qr = coefficients.clone().qr();
        let q = qr.q();
        let r = qr.r();
        if !r.is_invertible() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Matrix is not invertible.",
            ));
        }

        let b = DVector::from_row_slice(n, &z);
        let regress_coefficents = (r.try_inverse().unwrap() * q.transpose() * b)
            .as_slice()
            .to_vec(); //inv(R).dot(Q.T).dot(y)

        let mut residuals = vec![0f64; n];
        let mut ss_resid = 0f64;
        let mut y_hat: f64;
        for i in 0..n {
            y_hat = 0f64;
            for j in 0..num_coefficients {
                y_hat +=
                    forward_coefficient_matrix[i * num_coefficients + j] * regress_coefficents[j];
            }
            residuals[i] = z[i] - y_hat;
            ss_resid += residuals[i] * residuals[i];
        }

        let mut sum = 0f64;
        let mut ss = 0f64;
        for i in 0..n {
            ss += z[i] * z[i];
            sum += z[i];
        }
        let variance = (ss - (sum * sum) / n as f64) / n as f64;
        let ss_total = (n - 1) as f64 * variance;
        let r_sqr = 1f64 - ss_resid / ss_total;

        // create the output trend-surface report
        let p = path::Path::new(&output_file);
        let mut extension = String::from(".");
        let ext = p.extension().unwrap().to_str().unwrap();
        extension.push_str(ext);
        let output_html_file = output_file.replace(&extension, ".html");

        let f = File::create(output_html_file.clone())?;
        let mut writer = BufWriter::new(f);

        writer.write_all(&r#"<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">
        <html>
            <head>
                <meta content=\"text/html; charset=iso-8859-1\" http-equiv=\"content-type\">
                <title>Trend Surface Analysis Report</title>"#.as_bytes())?;

        // get the style sheet
        writer.write_all(&get_css().as_bytes())?;

        writer.write_all(
            &r#"
            </head>
            <body>
                <h1>Trend Surface Analysis Report</h1>
                "#
            .as_bytes(),
        )?;

        writer.write_all((format!("<p><strong>Input</strong>: {}</p>", input_file)).as_bytes())?;
        writer.write_all(
            (format!("<p><strong>Polynomial Order</strong>: {}</p>", order)).as_bytes(),
        )?;
        writer.write_all((format!("<p><strong>R-sqr</strong>: {:.*}</p>", 5, r_sqr)).as_bytes())?;

        //////////////////////////
        // Transformation Table //
        //////////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all(
            "<caption>Pre-calculation Transformation Coefficients</caption>".as_bytes(),
        )?;
        writer
            .write_all("<tr><th>&Delta;X</th><th>&Delta;Y</th><th>&Delta;Z</th></tr>".as_bytes())?;
        writer.write_all(&format!("<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>", min_x, min_y, min_z).as_bytes())?;
        writer.write_all("</table></p>".as_bytes())?;

        //////////////
        // Equation //
        //////////////
        let mut s = "z = ".to_string();
        let mut b_val = 1;
        for j in 0..(order + 1) {
            for k in 0..(order - j + 1) {
                let x_exp = if j > 1 {
                    format!("<sup>{}</sup>", j)
                } else {
                    "".to_string()
                };
                let y_exp = if k > 1 {
                    format!("<sup>{}</sup>", k)
                } else {
                    "".to_string()
                };
                if j != 0 && k != 0 {
                    s.push_str(&format!("b<sub>{}</sub>x{}y{} + ", b_val, x_exp, y_exp));
                } else if j != 0 {
                    s.push_str(&format!("b<sub>{}</sub>x{} + ", b_val, x_exp));
                } else if k != 0 {
                    s.push_str(&format!("b<sub>{}</sub>y{} + ", b_val, y_exp));
                } else {
                    s.push_str(&format!("b<sub>{}</sub> + ", b_val));
                }
                b_val += 1;
            }
        }
        s = s.trim().trim_right_matches(" +").to_string();
        writer.write_all(&format!("<p>{}</p>", s).as_bytes())?;

        ///////////////////////
        // Coefficient Table //
        ///////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>Regression Coefficients</caption>".as_bytes())?;
        writer.write_all("<tr><th>Coefficent Num.</th><th>Value</th></tr>".as_bytes())?;
        for j in 0..num_coefficients {
            let s = format!(
                "<td class=\"numberCell\">b<sub>{}</sub></td><td class=\"numberCell\">{:.*}</td>",
                (j + 1),
                12,
                regress_coefficents[j]
            );
            writer.write_all(&format!("<tr>{}</tr>", s).as_bytes())?;
        }
        writer.write_all("</table></p>".as_bytes())?;

        writer.write_all("</body>".as_bytes())?;

        let _ = writer.flush();

        if verbose {
            if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
                let output = Command::new("open")
                    .arg(output_html_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "windows") {
                // let output = Command::new("cmd /c start")
                let output = Command::new("explorer.exe")
                    .arg(output_html_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            } else if cfg!(target_os = "linux") {
                let output = Command::new("xdg-open")
                    .arg(output_html_file.clone())
                    .output()
                    .expect("failed to execute process");

                let _ = output.stdout;
            }

            println!("Please see {} for output report.", output_html_file);
        }

        // create the output trend-surface raster
        let mut term: f64;
        let mut m: usize;
        for row in 0..rows {
            for col in 0..columns {
                x_val = output.get_x_from_column(col) - min_x;
                y_val = output.get_y_from_row(row) - min_y;
                z_val = min_z; // 0f64;
                m = 0;
                for j in 0..(order + 1) {
                    for k in 0..(order - j + 1) {
                        term = x_val.powf(j as f64) * y_val.powf(k as f64);
                        z_val += term * regress_coefficents[m];
                        m += 1;
                    }
                }
                output.set_value(row, col, z_val);
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Polynomial order: {}", order));
        output.add_metadata_entry(format!("r-squared: {}", r_sqr));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

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

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
