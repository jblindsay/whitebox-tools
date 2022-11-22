use serde_json::Value;
// use duct;
use std::f32;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Default, Clone)]
pub struct ToolInfo {
    pub tool_name: String,
    pub toolbox: String,
    pub parameters: Vec<ToolParameter>,
    pub json_parameters: Value,
    pub cancel: Arc<Mutex<bool>>,
    pub tool_output: Arc<Mutex<String>>,
    pub exe_path: String,
    pub working_dir: String,
    pub output_command: bool,
    pub verbose_mode: bool,
    pub compress_rasters: bool,
    pub progress: Arc<Mutex<f32>>,
    pub progress_label: Arc<Mutex<String>>,
    pub continuous_mode: Arc<Mutex<bool>>,
}

impl ToolInfo {
    pub fn new(tool_name: &str, toolbox: &str, parameters: Value) -> Self {
        let parameter_values = parse_parameters(&parameters);
        ToolInfo {
            tool_name: tool_name.to_owned(),
            toolbox: toolbox.to_owned(),
            parameters: parameter_values,
            json_parameters: parameters,
            cancel: Arc::new(Mutex::new(false)),
            tool_output: Arc::new(Mutex::new(String::new())),
            exe_path: String::new(),
            working_dir: String::new(),
            output_command: false,
            verbose_mode: false,
            compress_rasters: false,
            progress: Arc::new(Mutex::new(0.0)),
            progress_label: Arc::new(Mutex::new("Progress".to_string())),
            continuous_mode: Arc::new(Mutex::new(false)),
        }
    }

    pub fn run(&mut self) {
        if let Ok(mut cancel) = self.cancel.lock() {
            *cancel = false;
        };
        
        // self.animate_progress = true;
        if self.exe_path.trim().is_empty() {
            // we have an unspecified non-optional param
            rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning).set_title("Error Running Tool")
            .set_description("The WhiteboxTools executable path does not appear to be set.")
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
            return;
        }
        // Collect the parameter values
        let mut param_str = String::new(); // String::from(&format!("{} -r={} --wd={}", self.exe_path, self.tool_name, self.working_dir));
        let mut args: Vec<String> = vec![format!("-r={}", self.tool_name), format!("--wd={}", self.working_dir)];
        for parameter in &self.parameters {
            let flag = parameter.flags[parameter.flags.len()-1].clone();
            match parameter.parameter_type {
                ParameterType::Boolean => { 
                    if parameter.bool_value {
                        // param_str.push_str(&format!(" {flag}={}", parameter.bool_value));
                        param_str.push_str(&format!(" {flag}"));
                        // args.push(format!("{flag}=", parameter.bool_value));
                        args.push(format!("{flag}"));
                    } 
                },
                ParameterType::String => { 
                    if !parameter.str_value.trim().is_empty() {
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_value)); 
                        args.push(format!("{flag}='{}'", parameter.str_value));
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                // ParameterType::StringList => { param_str.push_str(&format!("{flag}={:?}", parameter.str_vec_value); },
                ParameterType::Integer | ParameterType::Float => { 
                    if !parameter.str_value.trim().is_empty() {
                        // if (parameter.parameter_type == ParameterType::Integer && parameter.str_value.trim().parse::<usize>().is_ok()) || 
                        // (parameter.parameter_type == ParameterType::Float && parameter.str_value.trim().parse::<f32>().is_ok()) {
                        if parameter.str_value.trim().parse::<f32>().is_ok() {
                            let mut arg = parameter.str_value.clone();
                            if parameter.parameter_type == ParameterType::Integer && arg.trim().contains(".") {
                                arg = arg.split(".").collect::<Vec<&str>>()[0].trim().to_string();
                            }
                            param_str.push_str(&format!(" {flag}={}", arg));
                            args.push(format!("{flag}={}", arg));
                        } else {
                            // we had an error parsing the user intput in a number.
                            rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                            .set_description(&format!("Error parsing a non-optional parameter {}.", parameter.name))
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                            return;
                        }
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::VectorAttributeField => { 
                    if !parameter.str_value.trim().is_empty() {
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_value)); 
                        args.push(format!("{flag}='{}'", parameter.str_value));
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::StringOrNumber => {
                    if !parameter.str_value.trim().is_empty() { 
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_value)); 
                        args.push(format!("{flag}='{}'", parameter.str_value));
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::ExistingFile => { 
                    if !parameter.str_value.trim().is_empty() {
                        // does the file exist?
                        if std::path::Path::new(parameter.str_value.trim()).exists() {
                            param_str.push_str(&format!(" {flag}='{}'", parameter.str_value.trim()));
                            args.push(format!("{flag}='{}'", parameter.str_value));
                        } else {
                            // maybe we just need to append the working directory...
                            if std::path::Path::new(&self.working_dir).join(&parameter.str_value.trim()).exists() {
                                param_str.push_str(&format!(" {flag}='{}'", parameter.str_value.trim()));
                                args.push(format!("{flag}='{}'", parameter.str_value));
                            } else {
                                // we have an incorrect param
                                rfd::MessageDialog::new()
                                .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                                .set_description(&format!("The specified file path does not exist. ({}).", parameter.name))
                                .set_buttons(rfd::MessageButtons::Ok)
                                .show();

                                return;
                            }
                        } 
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::ExistingFileOrFloat => {
                    if !parameter.str_value.trim().is_empty() {
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_value));
                        args.push(format!("{flag}='{}'", parameter.str_value));
                    } else if !parameter.str_vec_value[0].trim().is_empty() {
                        match parameter.str_vec_value[0].trim().parse::<f32>() {
                            Ok(_) => {
                                param_str.push_str(&format!(" {flag}='{}'", parameter.str_vec_value[0]));
                                args.push(format!("{flag}='{}'", parameter.str_vec_value[0]));
                            },
                            Err(_) => {
                                // we had an error parsing the user intput in a number.
                                rfd::MessageDialog::new()
                                .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                                .set_description(&format!("Error parsing a non-optional parameter {}.", parameter.name))
                                .set_buttons(rfd::MessageButtons::Ok)
                                .show();
                                return;
                            }
                        }
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::NewFile => { 
                    if !parameter.str_value.trim().is_empty() {
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_value.trim())); 
                        args.push(format!("{flag}='{}'", parameter.str_value));
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::FileList => { 
                    if !parameter.str_value.trim().is_empty() {
                        let files: Vec<&str> = parameter.str_value.split("\n").collect();
                        let mut s = String::from("\"");
                        for i in 0..files.len() {
                            let file = files[i].trim();
                            if !file.is_empty() && std::path::Path::new(file).exists() {
                                if i > 0 {
                                    s.push_str(&format!(";{}", file));
                                } else {
                                    s.push_str(&format!("{}", file));
                                }
                            }
                        }
                        s.push_str("\"");
                        param_str.push_str(&format!(" {flag}={}", s)); 
                        args.push(format!("{flag}={}", s));
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::Directory => { 
                    if !parameter.str_value.trim().is_empty() {
                        if std::path::Path::new(parameter.str_value.trim()).exists() {
                            param_str.push_str(&format!(" {flag}='{}'", parameter.str_value));
                            args.push(format!("{flag}='{}'", parameter.str_value));
                        } else {
                            // we have an unspecified non-optional param
                            rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                            .set_description(&format!("The specified directory does not exist. ({}).", parameter.name.trim()))
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();

                            return;
                        }  
                    } else if !parameter.optional {
                        // we have an unspecified non-optional param
                        rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                        .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();

                        return;
                    }
                },
                ParameterType::OptionList => { 
                    // if !parameter.str_value.trim().is_empty() {
                        param_str.push_str(&format!(" {flag}='{}'", parameter.str_vec_value[parameter.int_value])); 
                        args.push(format!("{flag}='{}'", parameter.str_vec_value[parameter.int_value]));
                    // } else if !parameter.optional {
                    //     // we have an unspecified non-optional param
                    //     rfd::MessageDialog::new()
                    //     .set_level(rfd::MessageLevel::Warning).set_title("Error Parsing Parameter")
                    //     .set_description(&format!("Unspecified non-optional parameter {}.", parameter.name))
                    //     .set_buttons(rfd::MessageButtons::Ok)
                    //     .show();

                    //     return;
                    // }
                },
            }
        }
        

        if self.verbose_mode {
            param_str.push_str(" -v=true");
            args.push("-v=true".to_string());
        } else {
            param_str.push_str(" -v=false");
            args.push("-v=false".to_string());
        }

        if self.compress_rasters {
            param_str.push_str(" --compress_rasters=true");
            args.push("--compress_rasters=true".to_string());
        } else {
            param_str.push_str(" --compress_rasters=false");
            args.push("--compress_rasters=false".to_string());
        }

        let continuous_mode = Arc::clone(&self.continuous_mode);
        if let Ok(mut cm) = continuous_mode.lock() {
            *cm = true;
        }

        let tool_output = Arc::clone(&self.tool_output);
        if let Ok(mut to) = tool_output.lock() {
            if self.output_command {
                to.push_str(
                    &format!(
                        "{} -r={} --wd=\"{}\" {}\n", 
                        &self.exe_path, 
                        self.tool_name, 
                        self.working_dir, 
                        param_str
                    )
                );
            }
        }

        let exe_path = Arc::new(self.exe_path.clone());
        let exe = Arc::clone(&exe_path);
        let pcnt = Arc::clone(&self.progress);
        let progress_label = Arc::clone(&self.progress_label);
        let continuous_mode = Arc::clone(&self.continuous_mode);
        let tool_output = Arc::clone(&self.tool_output);
        let cancel = Arc::clone(&self.cancel);
        thread::spawn(move || {
            let mut child = Command::new(&*exe)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn().unwrap();

            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();

            let mut buf = [0u8; 200];
            let mut out_str = String::new();
            let mut do_read = || -> usize {
                let read = stdout.read(&mut buf).unwrap_or(0);
                let line = std::str::from_utf8(&buf[0..read]).unwrap_or("");
                if let Ok(mut to) = tool_output.lock() {
                    if line.contains("\n") {
                        let a = line.split("\n").collect::<Vec<&str>>();

                        for m in 0..a.len()-1 {
                            out_str.push_str(&format!("{}\n", a[m]));
                        
                            if out_str.contains("%") {
                                let val1: Vec<&str> = out_str.split(":").collect::<Vec<&str>>();
                                let percent_val = val1[1].replace("%", "").replace("\n", "").trim().parse::<f32>().unwrap_or(0.0);
                                if let Ok(mut val) = pcnt.lock() {
                                    *val = percent_val / 100.0;
                                }
        
                                if let Ok(mut val2) = progress_label.lock() {
                                    *val2 = val1[0].replace("\n", "").to_string();
                                }
                            } else {
                                if to.len() >= 10000 {
                                    to.clear();
                                }
                                to.push_str(&format!("{out_str}"));
                            }

                            out_str = "".to_string();
                        }

                        out_str.push_str(&format!("{}", a[a.len()-1]));
                    } else {
                        out_str.push_str(&format!("{line}"));
                    }

                    // if line.contains("%") {
                    //     let val1: Vec<&str> = line.split(":").collect::<Vec<&str>>();
                    //     let percent_val = val1[1].replace("%", "").trim().parse::<f32>().unwrap_or(0.0);
                    //     if let Ok(mut val) = pcnt.lock() {
                    //         *val = percent_val / 100.0;
                    //     }

                    //     if let Ok(mut val2) = progress_label.lock() {
                    //         *val2 = val1[0].to_string();
                    //     }
                    // }
                    // if to.len() > 10000 {
                    //     to.clear();
                    // }
                    // to.push_str(&format!("{line}"));
                }
                
                std::io::stdout().flush().unwrap();
                read
            };

            let mut last;
            while child.try_wait().unwrap().is_none() {
                if let Ok(mut cancel2) = cancel.lock() {
                    if *cancel2 {
                        // cancel the process
                        if let Ok(mut to) = tool_output.lock() {
                            to.push_str("\nCancelling process...\n");

                            match child.kill() {
                                Ok(_) => {
                                    *cancel2 = false; // reset the cancel.
                                    to.push_str("\nProcess cancelled\n");
                                },
                                Err(_) => {
                                    to.push_str("\nError encountered while killing process\n");
                                }
                            }
                        }
                    }
                }

                let _last = do_read();
            }

            // make sure we try at least one more read in case there's new data in the pipe after the child exited
            last = 1;

            while last > 0 {
                last = do_read();
            }

            // Was anything written to stderr?
            let mut s = String::new();
            let read = stderr.read_to_string(&mut s).unwrap_or(0);
            if read > 0 {
                println!("Error: {s}");
                if let Ok(mut to) = tool_output.lock() {
                    to.push_str(&s);
                }
            }

            if let Ok(mut val) = pcnt.lock() {
                *val = 0.0;
            }

            if let Ok(mut val2) = progress_label.lock() {
                *val2 = "Progress".to_string();
            }

            if let Ok(mut cm) = continuous_mode.lock() {
                *cm = false;
            }
        });

        // self.animate_progress = false;
    }

    pub fn cancel(&mut self) {
        if let Ok(mut cancel) = self.cancel.lock() {
            *cancel = true;
        }
    }

    pub fn reset(&mut self) {
        self.parameters = parse_parameters(&self.json_parameters);
        if let Ok(mut cancel) = self.cancel.lock() {
            *cancel = false;
        }

        if let Ok(mut tool_output) = self.tool_output.lock() {
            *tool_output = String::new();
        }
        
        if let Ok(mut val) = self.progress.lock() {
            *val = 0.0;
        }

        if let Ok(mut val2) = self.progress_label.lock() {
            *val2 = "Progress".to_string();
        }
    }

    pub fn update_exe_path(&mut self, exe_path: &str) {
        self.exe_path = exe_path.to_string();
    }

    pub fn update_working_dir(&mut self, working_dir: &str) {
        self.working_dir = working_dir.to_string();
    }

    pub fn update_output_command(&mut self, value: bool) {
        self.output_command = value;
    }

    pub fn update_verbose_mode(&mut self, value: bool) {
        self.verbose_mode = value;
    }

    pub fn update_compress_rasters(&mut self, value: bool) {
        self.compress_rasters = value;
    }

    pub fn get_tool_help(&self) -> Option<String> {
        let output = Command::new(&self.exe_path)
                .args([format!("--toolhelp={}", self.tool_name)])
                .output()
                .expect("Could not execute the WhiteboxTools binary");
        
        if output.status.success() {
            let s = match std::str::from_utf8(&(output.stdout)) {
                Ok(v) => v,
                Err(_) => return Some("https://www.whiteboxgeo.com/manual/wbt_book/intro.html".to_string()),
            };
            return Some(s.to_string());
        }
        Some("https://www.whiteboxgeo.com/manual/wbt_book/intro.html".to_string())
    }
}


#[derive(Default, Debug, PartialEq, Clone)]
pub enum ParameterType {
    Boolean,
    #[default]
    String,
    // StringList, // I don't think there are any tools that use this type
    Integer,
    Float,
    VectorAttributeField,
    StringOrNumber,
    ExistingFile,
    ExistingFileOrFloat,
    NewFile,
    FileList,
    Directory,
    OptionList,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub enum ParameterFileType {
    #[default]
    Any,
    Lidar,
    Raster,
    RasterAndVector,
    Vector,
    Text,
    Html,
    Csv,
    Dat,
}


#[derive(Default, Debug, PartialEq, Clone)]
pub enum VectorGeometryType {
    #[default]
    Any,
    Point,
    Line,
    Polygon,
    LineOrPolygon,
}

#[derive(Default, Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub flags: Vec<String>,
    pub description: String,
    pub parameter_type: ParameterType,
    pub default_value: Option<String>,
    pub optional: bool,
    pub str_value: String,
    pub bool_value: bool,
    pub int_value: usize,
    // pub float_value: f32,
    pub str_vec_value: Vec<String>,
    pub file_type: ParameterFileType,
    pub geometry_type: VectorGeometryType,
}

fn parse_parameters(parameters: &Value) -> Vec<ToolParameter> {
    let mut ret = vec![];
    let empty: Vec<Value> = vec![];
    let tool_parameters = parameters["parameters"].as_array().unwrap_or(&empty);
    for j in 0..tool_parameters.len() {
        let name = tool_parameters[j]["name"].as_str().unwrap_or("").to_string();
        let empty_arr: Vec<Value> = vec![];
        let flags: Vec<String> = tool_parameters[j]["flags"]
            .as_array()
            .unwrap_or(&empty_arr)
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_owned())
            .collect();
        let description = tool_parameters[j]["description"].as_str().unwrap_or("").to_string();
        let default_value = if tool_parameters[j]["default_value"].is_string() {
            Some(tool_parameters[j]["default_value"].as_str().unwrap_or("").to_string())
        } else {
            None
        };
        let optional = tool_parameters[j]["optional"].as_bool().unwrap_or(false);

        let mut str_vec_value: Vec<String> = vec![];
        let mut str_value = "".to_string();
        let mut bool_value = false;
        // let mut int_value = 0usize;
        // let mut float_value = f32::NAN;
        let mut file_type = ParameterFileType::Any;
        let mut geometry_type = VectorGeometryType::Any;

        let parameter_type = tool_parameters[j]["parameter_type"].clone();
        let pt = if parameter_type.is_string() {
            let s = parameter_type.as_str().unwrap_or("").to_lowercase();
            if s == "boolean" {
                if default_value.is_some() {
                    bool_value = default_value.clone().unwrap_or("false".to_string()).trim().to_lowercase().parse().unwrap_or(false);
                }
                ParameterType::Boolean
            } else if s == "float" {
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string()).trim().to_owned();
                }
                ParameterType::Float
            } else if s == "integer" {
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string()).trim().to_owned();
                }
                ParameterType::Integer
            } else if s == "string" {
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string());
                }
                ParameterType::String
            } else if s == "directory" {
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string());
                }
                ParameterType::Directory
            } else if s == "stringornumber" {
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string());
                }
                ParameterType::StringOrNumber
            } else {
                println!("Unknown String: {:?}", parameter_type);
                ParameterType::String
            }
        } else if parameter_type.is_object() {
            if !parameter_type["ExistingFile"].is_null() {
                if parameter_type["ExistingFile"].is_string() {
                    let s = parameter_type["ExistingFile"].as_str().unwrap_or("").trim().to_lowercase();
                    if s == "lidar" {
                        file_type = ParameterFileType::Lidar;
                    } else if s == "raster" {
                        file_type = ParameterFileType::Raster;
                    } else if s == "text" {
                        file_type = ParameterFileType::Text;
                    } else if s == "html" {
                        file_type = ParameterFileType::Html;
                    } else if s == "csv" {
                        file_type = ParameterFileType::Csv;
                    } else if s == "dat" {
                        file_type = ParameterFileType::Dat;
                    } else {
                        file_type = ParameterFileType::Any;
                    }
                } else if parameter_type["ExistingFile"].is_object() {
                    let o = parameter_type["ExistingFile"].as_object().unwrap();
                    if o.contains_key("Vector") {
                        if o["Vector"].is_string() {
                            file_type = ParameterFileType::Vector;
                            let s = o["Vector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    } else if o.contains_key("RasterAndVector") {
                        if o["RasterAndVector"].is_string() {
                            file_type = ParameterFileType::RasterAndVector;
                            let s = o["RasterAndVector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    }
                }
                ParameterType::ExistingFile
            } else if !parameter_type["NewFile"].is_null() {
                if parameter_type["NewFile"].is_string() {
                    let s = parameter_type["NewFile"].as_str().unwrap_or("").trim().to_lowercase();
                    if s == "lidar" {
                        file_type = ParameterFileType::Lidar;
                    } else if s == "raster" {
                        file_type = ParameterFileType::Raster;
                    } else if s == "text" {
                        file_type = ParameterFileType::Text;
                    } else if s == "html" {
                        file_type = ParameterFileType::Html;
                    } else if s == "csv" {
                        file_type = ParameterFileType::Csv;
                    } else if s == "dat" {
                        file_type = ParameterFileType::Dat;
                    } else {
                        file_type = ParameterFileType::Any;
                    }
                } else if parameter_type["NewFile"].is_object() {
                    let o = parameter_type["NewFile"].as_object().unwrap();
                    if o.contains_key("Vector") {
                        if o["Vector"].is_string() {
                            file_type = ParameterFileType::Vector;
                            let s = o["Vector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    } else if o.contains_key("RasterAndVector") {
                        if o["RasterAndVector"].is_string() {
                            file_type = ParameterFileType::RasterAndVector;
                            let s = o["RasterAndVector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    }
                }
                ParameterType::NewFile
            } else if !parameter_type["OptionList"].is_null() {
                str_vec_value = parameter_type["OptionList"]
                .as_array()
                .unwrap_or(&empty_arr)
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect();
                if default_value.is_some() {
                    str_value = default_value.clone().unwrap_or("".to_string());
                }
                ParameterType::OptionList
            } else if !parameter_type["FileList"].is_null() {
                if parameter_type["FileList"].is_string() {
                    let s = parameter_type["FileList"].as_str().unwrap_or("").trim().to_lowercase();
                    if s == "lidar" {
                        file_type = ParameterFileType::Lidar;
                    } else if s == "raster" {
                        file_type = ParameterFileType::Raster;
                    } else if s == "text" {
                        file_type = ParameterFileType::Text;
                    } else if s == "html" {
                        file_type = ParameterFileType::Html;
                    } else if s == "csv" {
                        file_type = ParameterFileType::Csv;
                    } else if s == "dat" {
                        file_type = ParameterFileType::Dat;
                    } else {
                        file_type = ParameterFileType::Any;
                    }
                } else if parameter_type["FileList"].is_object() {
                    let o = parameter_type["FileList"].as_object().unwrap();
                    if o.contains_key("Vector") {
                        if o["Vector"].is_string() {
                            file_type = ParameterFileType::Vector;
                            let s = o["Vector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    } else if o.contains_key("RasterAndVector") {
                        if o["RasterAndVector"].is_string() {
                            file_type = ParameterFileType::RasterAndVector;
                            let s = o["RasterAndVector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    }
                }
                ParameterType::FileList
            } else if !parameter_type["ExistingFileOrFloat"].is_null() {
                str_vec_value = vec!["".to_string()];
                if parameter_type["ExistingFileOrFloat"].is_string() {
                    let s = parameter_type["ExistingFileOrFloat"].as_str().unwrap_or("").trim().to_lowercase();
                    if s == "lidar" {
                        file_type = ParameterFileType::Lidar;
                    } else if s == "raster" {
                        file_type = ParameterFileType::Raster;
                    } else if s == "text" {
                        file_type = ParameterFileType::Text;
                    } else if s == "html" {
                        file_type = ParameterFileType::Html;
                    } else if s == "csv" {
                        file_type = ParameterFileType::Csv;
                    } else if s == "dat" {
                        file_type = ParameterFileType::Dat;
                    } else {
                        file_type = ParameterFileType::Any;
                    }
                } else if parameter_type["ExistingFileOrFloat"].is_object() {
                    let o = parameter_type["ExistingFileOrFloat"].as_object().unwrap();
                    if o.contains_key("Vector") {
                        if o["Vector"].is_string() {
                            file_type = ParameterFileType::Vector;
                            let s = o["Vector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    } else if o.contains_key("RasterAndVector") {
                        if o["RasterAndVector"].is_string() {
                            file_type = ParameterFileType::RasterAndVector;
                            let s = o["RasterAndVector"].as_str().unwrap_or("").trim().to_lowercase();
                            if s == "any" {
                                geometry_type =VectorGeometryType::Any;
                            } else if s == "point" {
                                geometry_type =VectorGeometryType::Point;
                            } else if s == "line" {
                                geometry_type =VectorGeometryType::Line;
                            } else if s == "polygon" {
                                geometry_type =VectorGeometryType::Polygon;
                            } else if s == "lineorpolygon" {
                                geometry_type =VectorGeometryType::LineOrPolygon;
                            }
                        }
                    }
                }
                ParameterType::ExistingFileOrFloat
            } else if !parameter_type["VectorAttributeField"].is_null() {
                str_vec_value = parameter_type["VectorAttributeField"]
                .as_array()
                .unwrap_or(&empty_arr)
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect();
                ParameterType::VectorAttributeField
            } else {
                println!("Object: {:?}", parameter_type);
                ParameterType::String
            }
        } else {
            println!("Something Else: {:?}", parameter_type);
            ParameterType::String
        };

        let tp = ToolParameter {
            name: name,
            flags: flags,
            description: description,
            parameter_type: pt,
            default_value: default_value,
            optional: optional,
            str_value: str_value,
            bool_value: bool_value,
            int_value: 0usize,
            // float_value: float_value,
            str_vec_value: str_vec_value,
            file_type: file_type,
            geometry_type: geometry_type,
            // attribute_type: AttributeType::Any,
        };
        ret.push(tp);
    }
    ret
}
