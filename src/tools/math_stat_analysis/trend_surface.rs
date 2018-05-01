/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 30/04/2018
Last Modified: 30/04/2018
License: MIT

HELP:
This tool can be used to interpolate a trend surface from a raster image. The 
technique uses a polynomial, least-squares regression analysis. The user must 
specify the name of the input raster file. In addition, the user must specify 
the polynomial order (1 to 10) for the analysis. A first-order polynomial is a 
planar surface with no curvature. As the polynomial order is increased, greater 
flexibility is allowed in the fitted surface. Although polynomial orders as high 
as 10 are accepted, numerical instability in the analysis often creates artifacts 
in trend surfaces of orders greater than 5. The operation will display a text 
report on completion, in addition to the output raster image. The report will 
list each of the coefficient values and the r-square value. Note that the entire 
raster image must be able to fit into computer memory, limiting the use of this 
tool to relatively small rasters. The Trend Surface (Vector Points) tool can be 
used instead if the input data is vector points contained in a shapefile.

Numerical stability is enhanced by transforming the x, y, z data by their minimum
values before performing the regression analysis. These transform parameters 
are also reported in the output report.
*/

use time;
use std::env;
use std::path;
use raster::*;
use std::io::{Error, ErrorKind};
use tools::*;
use na::{DMatrix, DVector};
use rendering::html::*;
use std::io::BufWriter;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

pub struct TrendSurface {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TrendSurface {
    pub fn new() -> TrendSurface { // public constructor
        let name = "TrendSurface".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Estimates the trend surface of an input raster file.".to_string();
        
        let mut parameters = vec![];
        parameters.push(ToolParameter{
            name: "Input File".to_owned(), 
            flags: vec!["-i".to_owned(), "--input".to_owned()], 
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Output File".to_owned(), 
            flags: vec!["-o".to_owned(), "--output".to_owned()], 
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false
        });

        parameters.push(ToolParameter{
            name: "Polynomial Order".to_owned(), 
            flags: vec!["--order".to_owned()], 
            description: "Polynomial order (1 to 10).".to_owned(),
            parameter_type: ParameterType::Integer,
            default_value: Some("1".to_string()),
            optional: true
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "").replace(".exe", "").replace(".", "").replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i='input.tif' -o='output.tif' --order=2", short_exe, name).replace("*", &sep);
    
        TrendSurface { 
            name: name, 
            description: description, 
            toolbox: toolbox,
            parameters: parameters, 
            example_usage: usage 
        }
    }
}

impl WhiteboxTool for TrendSurface {
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

    fn run<'a>(&self, args: Vec<String>, working_directory: &'a str, verbose: bool) -> Result<(), Error> {
        let mut input_file = String::new();
        let mut output_file = String::new();
        let mut order = 1usize;
        
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
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i+1].to_string()
                };
            } else if flag_val == "-order" {
                order = if keyval {
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !input_file.contains(&sep) && !input_file.contains("/") {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if order < 1 { order = 1; }
        if order > 10 { order = 10; }

        if verbose { println!("Reading data...") };

        let input = Raster::new(&input_file, "r")?;
        let start = time::now();
        
        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let nodata = input.configs.nodata;

        let min_x = input.configs.west;
        let min_y = input.configs.south;
        let min_z = input.configs.minimum;


        // get the input data
        let total_cells = rows * columns;
        let mut x: Vec<f64> = Vec::with_capacity(total_cells as usize);
        let mut y: Vec<f64> = Vec::with_capacity(total_cells as usize);
        let mut z: Vec<f64> = Vec::with_capacity(total_cells as usize);
        let (mut x_val, mut y_val, mut z_val): (f64, f64, f64);
        for row in 0..rows {
            for col in 0..columns {
                z_val = input.get_value(row, col);
                if z_val != nodata {
                    x_val = input.get_x_from_column(col);
                    y_val = input.get_y_from_row(row);
                    x.push(x_val - min_x);
                    y.push(y_val - min_y);
                    z.push(z_val - min_z);
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // calculate the equation

        let n = z.len();

        // How many coefficients are there?
        let mut num_coefficients = 0;
        for j in 0..(order+1) {
            for _k in 0..(order - j + 1) {
                num_coefficients += 1;
            }
        }

        // Solve the forward transformation equations
        let mut forward_coefficient_matrix = vec![0f64; n * num_coefficients];
        for i in 0..n {
            let mut m = 0;
            for j in 0..(order+1) {
                for k in 0..(order - j + 1) {
                    forward_coefficient_matrix[i * num_coefficients + m] = x[i].powf(j as f64) * y[i].powf(k as f64);
                    m += 1;
                }
            }
        }
        
        let coefficients = DMatrix::from_row_slice(n, num_coefficients, &forward_coefficient_matrix);
        let qr = coefficients.clone().qr();
        let q  = qr.q();
        let r  = qr.r();
        if !r.is_invertible() {
            return Err(Error::new(ErrorKind::InvalidInput,  "Matrix is not invertible."));
        }
        
        let b = DVector::from_row_slice(n, &z);
        let regress_coefficents = (r.try_inverse().unwrap() * q.transpose() * b).as_slice().to_vec(); //inv(R).dot(Q.T).dot(y)
        
        let mut residuals = vec![0f64; n];
        let mut ss_resid = 0f64;
        let mut y_hat: f64;
        for i in 0..n {
            y_hat = 0f64;
            for j in 0..num_coefficients {
                y_hat += forward_coefficient_matrix[i * num_coefficients + j] * regress_coefficents[j];
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
            
        writer.write_all(&r#"
            </head>
            <body>
                <h1>Trend Surface Analysis Report</h1>
                "#.as_bytes())?;

        writer.write_all((format!("<p><strong>Input</strong>: {}</p>", input_file)).as_bytes())?;
        writer.write_all((format!("<p><strong>Polynomial Order</strong>: {}</p>", order)).as_bytes())?;
        writer.write_all((format!("<p><strong>R-sqr</strong>: {:.*}</p>", 5, r_sqr)).as_bytes())?;

        //////////////////////////
        // Transformation Table //
        //////////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>Pre-calculation Transformation Coefficients</caption>".as_bytes())?;
        writer.write_all("<tr><th>&Delta;X</th><th>&Delta;Y</th><th>&Delta;Z</th></tr>".as_bytes())?;
        writer.write_all(&format!("<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>", min_x, min_y, min_z).as_bytes())?;
        writer.write_all("</table></p>".as_bytes())?;

        ///////////////////////
        // Coefficient Table //
        ///////////////////////
        writer.write_all("<p><table>".as_bytes())?;
        writer.write_all("<caption>Regression Coefficients</caption>".as_bytes())?;
        writer.write_all("<tr><th>Coefficent Num.</th><th>Value</th></tr>".as_bytes())?;
        for j in 0..num_coefficients {
            let mut s = format!("<td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.*}</td>", (j+1), 12, regress_coefficents[j]);
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
        let mut output = Raster::initialize_using_file(&output_file, &input);
        let mut term: f64;
        let mut m: usize;   
        for row in 0..rows {
            for col in 0..columns {
                x_val = input.get_x_from_column(col) - min_x;
                y_val = input.get_y_from_row(row) - min_y;
                z_val = min_z; // 0f64;
                m = 0;
                for j in 0..(order+1) {
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

        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool", self.get_tool_name()));
        output.add_metadata_entry(format!("Input file: {}", input_file));
        output.add_metadata_entry(format!("Polynomial order: {}", order));
        output.add_metadata_entry(format!("r-squared: {}", r_sqr));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        if verbose { println!("Saving data...") };
        let _ = match output.write() {
            Ok(_) => if verbose { println!("Output file written") },
            Err(e) => return Err(e),
        };
           
        if verbose {
            println!("{}", &format!("Elapsed Time (including I/O): {}", elapsed_time).replace("PT", ""));
        }
        
        Ok(())
    }
}