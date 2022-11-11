use serde_json::Value;
use crate::MyApp;
use crate::toggle;
use case::CaseExt;
// use duct;
use std::f32;
// use std::io::prelude::*;
// use std::io::BufReader;
// use std::process::{Command, Stdio};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use whitebox_vector::{ShapeType, Shapefile};

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
    file_type: ParameterFileType,
    geometry_type: VectorGeometryType,
}

#[derive(Default, Clone)]
pub struct ToolInfo {
    pub tool_name: String,
    pub toolbox: String,
    pub parameters: Vec<ToolParameter>,
    json_parameters: Value,
    cancel: Arc<Mutex<bool>>,
    tool_output: Arc<Mutex<String>>,
    exe_path: String,
    working_dir: String,
    output_command: bool,
    verbose_mode: bool,
    compress_rasters: bool,
    progress: Arc<Mutex<f32>>,
    progress_label: Arc<Mutex<String>>,
    continuous_mode: Arc<Mutex<bool>>,
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
                        if (parameter.parameter_type == ParameterType::Integer && parameter.str_value.trim().parse::<usize>().is_ok()) || 
                        (parameter.parameter_type == ParameterType::Float && parameter.str_value.trim().parse::<f32>().is_ok()){
                            param_str.push_str(&format!(" {flag}={}", parameter.str_value));
                            args.push(format!("{flag}={}", parameter.str_value));
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
                        let mut s = String::from("[");
                        for i in 0..files.len() {
                            let file = files[i].trim();
                            if !file.is_empty() && std::path::Path::new(file).exists() {
                                if i > 0 {
                                    s.push_str(&format!(",'{}'", file));
                                } else {
                                    s.push_str(&format!("'{}'", file));
                                }
                            }
                        }
                        s.push_str("]");
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
                .spawn().unwrap();

            let mut stdout = child.stdout.take().unwrap();

            let mut buf = [0u8; 200];
            let mut do_read = || -> usize {
                let read = stdout.read(&mut buf).unwrap_or(0);
                let line = std::str::from_utf8(&buf[0..read]).unwrap_or("");
                if let Ok(mut to) = tool_output.lock() {
                    if line.contains("%") {
                        let val1: Vec<&str> = line.split(":").collect::<Vec<&str>>();
                        let percent_val = val1[1].replace("%", "").trim().parse::<f32>().unwrap_or(0.0);
                        if let Ok(mut val) = pcnt.lock() {
                            *val = percent_val / 100.0;
                        }

                        if let Ok(mut val2) = progress_label.lock() {
                            *val2 = val1[0].to_string();
                        }
                    }
                    to.push_str(&format!("{line}"));
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

    fn get_tool_help(&self) -> Option<String> {
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

impl MyApp {

    pub fn tool_dialog(&mut self, ctx: &egui::Context, tool_idx: usize) {
        let mut close_dialog = false;
        _ = self.get_tool_parameters(&self.list_of_open_tools[tool_idx].tool_name);
        egui::Window::new(&format!("{}", &self.list_of_open_tools[tool_idx].tool_name))
        .id(egui::Id::new(format!("{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx)))
        .open(&mut self.open_tools[tool_idx])
        .resizable(true)
        .vscroll(false)
        .show(ctx, |ui| {

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Tool Parameters:").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔃").on_hover_text("Reset parameters").clicked() { // ⟲
                        self.list_of_open_tools[tool_idx].reset();
                    }
                });
            });
            // ui.separator();

            egui::ScrollArea::vertical()
                .min_scrolled_height(50.)
                .max_height(150.0)
                .auto_shrink([true; 2])
                .show(ui, |ui| {

            egui::Grid::new(&format!("grid{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx))
            .num_columns(2)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {

                // ui.label(egui::RichText::new("Tool Parameters:").strong());
                // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                //     if ui.button("⟲").on_hover_text("Reset parameters").clicked() {
                //         self.list_of_open_tools[tool_idx].reset();
                //     }
                // });
                // ui.end_row();
            

                for parameter in &mut (self.list_of_open_tools[tool_idx].parameters) {
                    let suffix = if parameter.optional { "*".to_string() } else { "".to_string() };
                    let parameter_label = if parameter.name.len() + suffix.len() < 25 {
                        format!("{}{}", &parameter.name, suffix)
                    } else {
                        format!("{}...{}", &parameter.name[0..(22-suffix.len())], suffix)
                    };
                    let param_nm = if !parameter.optional { parameter.name.clone() } else { format!("{} [Optional]", parameter.name) };
                    let hover_text = match parameter.file_type {
                        ParameterFileType::Vector | ParameterFileType::RasterAndVector => {
                            format!("{}:  {} (Geometry Type={:?})", param_nm, parameter.description, parameter.geometry_type)
                        },
                        _ => {
                            format!("{}:  {}", param_nm, parameter.description)
                        }
                    };
                    ui.label(&parameter_label)
                    .on_hover_text(&hover_text);

                    match parameter.parameter_type {
                        ParameterType::Boolean => {
                            ui.add(toggle(&mut parameter.bool_value));
                        },
                        ParameterType::Directory => {
                            if ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            ).double_clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(mut path) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_file() {
                                    parameter.str_value = path.display().to_string();
                                    // update the working directory
                                    path.pop();
                                    self.state.working_dir = path.display().to_string();
                                }
                            }
                            if ui.button("…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().set_directory(std::path::Path::new(&self.state.working_dir)).pick_folder() {
                                    parameter.str_value = path.display().to_string();
                                }
                            }
                        },
                        ParameterType::ExistingFile => {
                            if ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            ).double_clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(mut path) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_file() {
                                    parameter.str_value = path.display().to_string();
                                    
                                    if parameter.file_type == ParameterFileType::Vector && 
                                    parameter.geometry_type != VectorGeometryType::Any {
                                        // Read the file and make sure that it is the right geometry type.
                                        match Shapefile::read(&parameter.str_value) {
                                            Ok(vector_data) => {
                                                let base_shape_type = vector_data.header.shape_type.base_shape_type();
                                                // make sure the input vector file is of the right shape type
                                                let err_found = match parameter.geometry_type {
                                                    VectorGeometryType::Point => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::Point {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::Line => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::PolyLine {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::Polygon => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::Polygon {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::LineOrPolygon => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::PolyLine && base_shape_type != ShapeType::Polygon {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    _ => { false }
                                                };
                                                if err_found {
                                                    if rfd::MessageDialog::new()
                                                    .set_level(rfd::MessageLevel::Warning)
                                                    .set_title("Wrong Vector Geometry Type")
                                                    .set_description("The specified file does not have the correct vector geometry type for this parameter. Do you want to continue?")
                                                    .set_buttons(rfd::MessageButtons::YesNo)
                                                    .show() {
                                                        // do nothing
                                                    } else {
                                                        // Reset the parameter string value.
                                                        parameter.str_value = "".to_string();
                                                    }
                                                }
                                            },
                                            Err(_) => {} // do nothing
                                        }
                                    }

                                    // update the working directory
                                    path.pop();
                                    self.state.working_dir = path.display().to_string();
                                }
                            }

                            if ui.button("…").clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(mut path) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_file() {
                                    parameter.str_value = path.display().to_string();

                                    if parameter.file_type == ParameterFileType::Vector && 
                                    parameter.geometry_type != VectorGeometryType::Any {
                                        // Read the file and make sure that it is the right geometry type.
                                        match Shapefile::read(&parameter.str_value) {
                                            Ok(vector_data) => {
                                                let base_shape_type = vector_data.header.shape_type.base_shape_type();
                                                // make sure the input vector file is of the right shape type
                                                let err_found = match parameter.geometry_type {
                                                    VectorGeometryType::Point => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::Point {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::Line => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::PolyLine {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::Polygon => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::Polygon {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    VectorGeometryType::LineOrPolygon => {
                                                        let mut ret = false;
                                                        if base_shape_type != ShapeType::PolyLine && base_shape_type != ShapeType::Polygon {
                                                            ret = true;
                                                        }
                                                        ret
                                                    },
                                                    _ => { false }
                                                };
                                                if err_found {
                                                    if rfd::MessageDialog::new()
                                                    .set_level(rfd::MessageLevel::Warning)
                                                    .set_title("Wrong Vector Geometry Type")
                                                    .set_description("The specified file does not have the correct vector geometry type for this parameter. Do you want to continue?")
                                                    .set_buttons(rfd::MessageButtons::YesNo)
                                                    .show() {
                                                        // do nothing
                                                    } else {
                                                        // Reset the parameter string value.
                                                        parameter.str_value = "".to_string();
                                                    }
                                                }
                                            },
                                            Err(_) => {} // do nothing
                                        }
                                    }

                                    // update the working directory
                                    path.pop();
                                    self.state.working_dir = path.display().to_string();
                                }
                            }
                        },
                        ParameterType::ExistingFileOrFloat => {
                            ui.horizontal(|ui| {
                                if ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(self.state.textbox_width)
                                ).double_clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type); 
                                    if let Some(mut path) = fdialog
                                    .set_directory(std::path::Path::new(&self.state.working_dir))
                                    .pick_file() {
                                        parameter.str_value = path.display().to_string();
                                        // update the working directory
                                        path.pop();
                                        self.state.working_dir = path.display().to_string();
                                    }
                                }
                                if ui.button("…").clicked() {
                                    let fdialog = get_file_dialog(&parameter.file_type); 
                                    if let Some(mut path) = fdialog
                                    .set_directory(std::path::Path::new(&self.state.working_dir))
                                    .pick_file() {
                                        parameter.str_value = path.display().to_string();
                                        // update the working directory
                                        path.pop();
                                        self.state.working_dir = path.display().to_string();
                                    }
                                }

                                ui.label("OR");
                                
                                ui.add(
                                    egui::TextEdit::singleline(&mut parameter.str_value)
                                    .desired_width(50.0)
                                );
                            });
                        },
                        ParameterType::FileList => {
                            if ui.add(
                                egui::TextEdit::multiline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            ).double_clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(mut path) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_file() {
                                    parameter.str_value = path.display().to_string();
                                    // update the working directory
                                    path.pop();
                                    self.state.working_dir = path.display().to_string();
                                }
                            }
                            if ui.button("…").clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type);

                                if let Some(mut paths) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_files() {
                                    // let s = String::new();
                                    for path in &paths {
                                        parameter.str_value.push_str(&format!("{}\n", path.display().to_string()));
                                    }
                                    
                                    // update the working directory
                                    paths[0].pop();
                                    self.state.working_dir = paths[0].display().to_string();
                                }
                            }
                        }
                        ParameterType::Float | ParameterType::Integer => {
                            // ui.add(egui::DragValue::new(&mut parameter.float_value).speed(0).max_decimals(5));
                            ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(50.0) //self.state.textbox_width)
                            );

                            // let text_edit = egui::TextEdit::singleline(&mut parameter.str_value)
                            // .desired_width(50.0);
                            // let output = text_edit.show(ui);
                            // if output.response.double_clicked() {
                            //     // What to do here?
                            // }

                        },
                        ParameterType::NewFile => {
                            // ui.text_edit_singleline(&mut parameter.str_value);
                            if ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            ).double_clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(mut path) = fdialog
                                .set_directory(std::path::Path::new(&self.state.working_dir))
                                .pick_file() {
                                    parameter.str_value = path.display().to_string();
                                    // update the working directory
                                    path.pop();
                                    self.state.working_dir = path.display().to_string();
                                }
                            }
                            if ui.button("…").clicked() {
                                let fdialog = get_file_dialog(&parameter.file_type); 
                                if let Some(path) = fdialog.set_directory(std::path::Path::new(&self.state.working_dir)).save_file() {
                                    parameter.str_value = path.display().to_string();
                                }
                            }
                        },
                        ParameterType::OptionList => {
                            let alternatives = parameter.str_vec_value.clone();
                            egui::ComboBox::from_id_source(&parameter.name).show_index(
                                ui,
                                &mut parameter.int_value,
                                alternatives.len(),
                                |i| alternatives[i].to_owned()
                            );
                        }
                        ParameterType::String => {
                            ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            );
                        },
                        ParameterType::StringOrNumber => {
                            ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            );
                        },
                        ParameterType::VectorAttributeField => {
                            ui.add(
                                egui::TextEdit::singleline(&mut parameter.str_value)
                                .desired_width(self.state.textbox_width)
                            );
                        },
                    }
                    
                    ui.end_row();
                }
            });
        
            });

            if self.state.view_tool_output {
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_height(145.);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Tool Output:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖").on_hover_text("Clear tool output").clicked() {
                                if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                                    *tool_output = "".to_string();
                                }
                            }
                        });
                    });

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                            ui.add(
                                egui::TextEdit::multiline(&mut *tool_output)
                                    .id_source(&format!("out_{}-{}", &self.list_of_open_tools[tool_idx].tool_name, tool_idx))
                                    .cursor_at_end(true)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(8)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY)
                            );

                            if let Ok(cm) = self.list_of_open_tools[tool_idx].continuous_mode.lock() {
                                if *cm {
                                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                }
                            }
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        // ui.small(""); // just to add some vertical distance between it and the output text box.
                        if let Ok(progress) = self.list_of_open_tools[tool_idx].progress.lock() {
                            if let Ok(progress_label) = self.list_of_open_tools[tool_idx].progress_label.lock() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.add(egui::ProgressBar::new(*progress)
                                    .desired_width(100.0)
                                    .show_percentage());

                                    ui.label(&*progress_label);
                                });
                            }
                        }
                    });
                });
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Run").clicked() {
                    self.list_of_open_tools[tool_idx].update_working_dir(&self.state.working_dir);
                    self.list_of_open_tools[tool_idx].update_exe_path(&self.state.whitebox_exe);
                    self.list_of_open_tools[tool_idx].run();
                }
                if ui.button("Cancel").clicked() {
                    self.list_of_open_tools[tool_idx].cancel();
                }
                if ui.button("Help").clicked() {
                    let toolbox = self.list_of_open_tools[tool_idx]
                    .toolbox
                    .replace("GIS", "Gis")
                    .replace("TIN", "Tin")
                    .replace("LiDAR", "Lidar")
                    .replace("/", "")
                    .replace(" ", "")
                    .to_snake();

                    let tool_name = self.list_of_open_tools[tool_idx]
                    .tool_name
                    .replace("GIS", "Gis")
                    .replace("TIN", "Tin")
                    .replace("LiDAR", "Lidar")
                    .replace("/", "")
                    .replace(" ", "");
                    let url = format!("https://www.whiteboxgeo.com/manual/wbt_book/available_tools/{}.html#{}", toolbox, tool_name);
                    println!("URL: {url}");
                    if !webbrowser::open(&url).is_ok() {
                        if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                            tool_output.push_str("Could not navigate to help link in browser.\n");

                            let help_str = self.list_of_open_tools[tool_idx].get_tool_help();
                            if help_str.is_some() {
                                *tool_output = help_str.unwrap_or("".to_string());
                            }
                        }
                    }

                }
                if ui.button("View Code").clicked() {
                    // let url = self.view_code(&(self.list_of_open_tools[tool_idx].tool_name));
                    let output = std::process::Command::new(&self.state.whitebox_exe)
                            .args([&format!("--viewcode={}", self.list_of_open_tools[tool_idx].tool_name)])
                            .output()
                            .expect("Could not execute the WhiteboxTools binary");
                    
                    if output.status.success() {
                        let url = match std::str::from_utf8(&(output.stdout)) {
                            Ok(v) => v.to_string(),
                            Err(_) => "https://github.com/jblindsay/whitebox-tools".to_string(),
                        };
                        if !webbrowser::open(&url).is_ok() {
                            if let Ok(mut tool_output) = self.list_of_open_tools[tool_idx].tool_output.lock() {
                                tool_output.push_str("Could not navigate to code link in browser.\n");
                            }
                        }
                    } else {
                        println!("stdout: {}", std::str::from_utf8(output.stdout.as_slice()).unwrap_or("None"));
                        println!("stderr: {}", std::str::from_utf8(output.stderr.as_slice()).unwrap_or("None"));
                    }
                }
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }

                // let progress = *(self.list_of_open_tools[tool_idx].progress).lock().unwrap_or(0.);
                // let progress_label = &*(self.list_of_open_tools[tool_idx].progress_label).lock().unwrap_or("Progress");
                // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                //     ui.add(egui::ProgressBar::new(progress)
                //     .desired_width(100.0)
                //     .show_percentage());

                //     ui.label(progress_label);
                // });
            });

            if let Ok(cm) = self.list_of_open_tools[tool_idx].continuous_mode.lock() {
                if *cm {
                    ctx.request_repaint();
                }
            }
        });

        if close_dialog {
            self.open_tools[tool_idx] = false;
        }
    }
}


fn get_file_dialog(pft: &ParameterFileType) -> rfd::FileDialog {
    match pft {
        ParameterFileType::Lidar => {
            rfd::FileDialog::new()
            .add_filter("LAS Files", &["las"])
            .add_filter("LAZ Files", &["laz"])
            .add_filter("zLidar Files", &["zLidar"])
            .add_filter("Lidar Files", &["las", "laz", "zLidar"])
        },
        ParameterFileType::Raster => {
            rfd::FileDialog::new()
            .add_filter("Raster Files", &["tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"])
            .add_filter("GeoTIFF Files", &["tif", "tiff"])
        },
        ParameterFileType::Vector => {
            rfd::FileDialog::new()
            .add_filter("Vector Files", &["shp"])
        },
        ParameterFileType::RasterAndVector => {
            rfd::FileDialog::new()
            .add_filter("Raster Files", &["tif", "tiff", "bil", "hdr", "flt", "sdat", "sgrd", "rdc", "rst", "grd", "txt", "asc", "tas", "dep"])
            .add_filter("Vector Files", &["shp"])
        },
        ParameterFileType::Text => {
            rfd::FileDialog::new()
            .add_filter("Test Files", &["txt"])
        },
        ParameterFileType::Html => {
            rfd::FileDialog::new()
            .add_filter("HTML Files", &["html"])
        },
        ParameterFileType::Csv => {
            rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
        },
        ParameterFileType::Dat => {
            rfd::FileDialog::new()
            .add_filter("DAT Files", &["dat"])
        },
        _ => { 
            rfd::FileDialog::new()
        }
    }
}
