/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 25/07/2017
Last Modified: 22/10/2019
License: MIT
*/

use crate::raster::*;
use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool transforms three intensity, hue, and saturation (IHS; sometimes HSI or HIS) raster images into three 
/// equivalent multispectral images corresponding with the red, green, and blue channels of an RGB composite. Intensity 
/// refers to the brightness of a color, hue is related to the dominant wavelength of light and is perceived as color, 
/// and saturation is the purity of the color (Koutsias et al., 2000). There are numerous algorithms for performing a 
/// red-green-blue (RGB) to IHS transformation. This tool uses the transformation described by Haydn (1982). Note that, 
/// based on this transformation, the input IHS values must follow the ranges:
/// 
/// > 0 < I < 1
/// > 
/// > 0 < H < 2PI 
/// > 
/// > 0 < S < 1
/// 
/// The output red, green, and blue images will have values ranging from 0 to 255. The user must specify the names of the 
/// intensity, hue, and saturation images (`--intensity`, `--hue`, `--saturation`). These images will generally be created using 
/// the `RgbToIhs` tool. The user must also specify the names of the output red, green, and blue images (`--red`, `--green`, 
/// `--blue`). Image enhancements, such as contrast stretching, are often performed on the individual IHS components, which are 
/// then inverse transformed back in RGB components using this tool. The output RGB components can then be used to create an 
/// improved color composite image.
/// 
/// # References
/// Haydn, R., Dalke, G.W. and Henkel, J. (1982) Application of the IHS color transform to the processing of multisensor 
/// data and image enhancement. Proc. of the Inter- national Symposium on Remote Sensing of Arid and Semiarid Lands, 
/// Cairo, 599-616.
/// 
/// Koutsias, N., Karteris, M., and Chuvico, E. (2000). The use of intensity-hue-saturation transformation of Landsat-5 Thematic 
/// Mapper data for burned land mapping. Photogrammetric Engineering and Remote Sensing, 66(7), 829-840.
/// 
/// # See Also
/// `RgbToIhs`, `BalanceContrastEnhancement`, `DirectDecorrelationStretch`
pub struct IhsToRgb {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl IhsToRgb {
    /// Public constructor.
    pub fn new() -> IhsToRgb {
        let name = "IhsToRgb".to_string();
        let toolbox = "Image Processing Tools".to_owned();
        let description = "Converts intensity, hue, and saturation (IHS) images into red, green, and blue (RGB) images.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input Intensity File".to_owned(),
            flags: vec!["--intensity".to_owned()],
            description: "Input intensity file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Hue File".to_owned(),
            flags: vec!["--hue".to_owned()],
            description: "Input hue file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Saturation File".to_owned(),
            flags: vec!["--saturation".to_owned()],
            description: "Input saturation file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Red Band File (optional; only if colour-composite not specified)"
                .to_owned(),
            flags: vec!["--red".to_owned()],
            description:
                "Output red band file. Optionally specified if colour-composite not specified."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Green Band File (optional; only if colour-composite not specified)"
                .to_owned(),
            flags: vec!["--green".to_owned()],
            description:
                "Output green band file. Optionally specified if colour-composite not specified."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Blue Band File (optional; only if colour-composite not specified)"
                .to_owned(),
            flags: vec!["--blue".to_owned()],
            description:
                "Output blue band file. Optionally specified if colour-composite not specified."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Colour-Composite File (optional; only if individual bands not specified)"
                .to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description:
                "Output colour-composite file. Only used if individual bands are not specified."
                    .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --intensity=intensity.tif --hue=hue.tif --saturation=saturation.tif --red=band3.tif --green=band2.tif --blue=band1.tif
>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --intensity=intensity.tif --hue=hue.tif --saturation=saturation.tif --composite=image.tif", short_exe, name).replace("*", &sep);

        IhsToRgb {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for IhsToRgb {
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
        let mut red_file = String::new();
        let mut green_file = String::new();
        let mut blue_file = String::new();
        let mut intensity_file = String::new();
        let mut hue_file = String::new();
        let mut saturation_file = String::new();
        let mut composite_file = String::new();
        let mut use_composite = false;
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
            if flag_val == "-r" || flag_val == "-red" {
                if keyval {
                    red_file = vec[1].to_string();
                } else {
                    red_file = args[i + 1].to_string();
                }
            } else if flag_val == "-g" || flag_val == "-green" {
                if keyval {
                    green_file = vec[1].to_string();
                } else {
                    green_file = args[i + 1].to_string();
                }
            } else if flag_val == "-b" || flag_val == "-blue" {
                if keyval {
                    blue_file = vec[1].to_string();
                } else {
                    blue_file = args[i + 1].to_string();
                }
            } else if flag_val == "-i" || flag_val == "-intensity" {
                if keyval {
                    intensity_file = vec[1].to_string();
                } else {
                    intensity_file = args[i + 1].to_string();
                }
            } else if flag_val == "-h" || flag_val == "-hue" {
                if keyval {
                    hue_file = vec[1].to_string();
                } else {
                    hue_file = args[i + 1].to_string();
                }
            } else if flag_val == "-s" || flag_val == "-saturation" {
                if keyval {
                    saturation_file = vec[1].to_string();
                } else {
                    saturation_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-composite" || flag_val == "-output" {
                if keyval {
                    composite_file = vec[1].to_string();
                } else {
                    composite_file = args[i + 1].to_string();
                }
                use_composite = true;
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

        if !use_composite {
            if !red_file.contains(&sep) && !red_file.contains("/") {
                red_file = format!("{}{}", working_directory, red_file);
            }
            if !green_file.contains(&sep) && !green_file.contains("/") {
                green_file = format!("{}{}", working_directory, green_file);
            }
            if !blue_file.contains(&sep) && !blue_file.contains("/") {
                blue_file = format!("{}{}", working_directory, blue_file);
            }
        } else {
            if !composite_file.contains(&sep) && !composite_file.contains("/") {
                composite_file = format!("{}{}", working_directory, composite_file);
            }
        }
        if !intensity_file.contains(&sep) && !intensity_file.contains("/") {
            intensity_file = format!("{}{}", working_directory, intensity_file);
        }
        if !hue_file.contains(&sep) && !hue_file.contains("/") {
            hue_file = format!("{}{}", working_directory, hue_file);
        }
        if !saturation_file.contains(&sep) && !saturation_file.contains("/") {
            saturation_file = format!("{}{}", working_directory, saturation_file);
        }

        if verbose {
            println!("Reading intensity band data...")
        };
        let input_i = Arc::new(Raster::new(&intensity_file, "r")?);
        if verbose {
            println!("Reading hue band data...")
        };
        let input_h = Arc::new(Raster::new(&hue_file, "r")?);
        if verbose {
            println!("Reading saturation band data...")
        };
        let input_s = Arc::new(Raster::new(&saturation_file, "r")?);

        let rows = input_i.configs.rows as isize;
        let columns = input_i.configs.columns as isize;
        let nodata_i = input_i.configs.nodata;
        let nodata_h = input_h.configs.nodata;
        let nodata_s = input_s.configs.nodata;

        let start = Instant::now();

        // make sure the input files have the same size
        if input_i.configs.rows != input_h.configs.rows
            || input_i.configs.columns != input_h.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }
        if input_i.configs.rows != input_s.configs.rows
            || input_i.configs.columns != input_s.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let num_procs = num_cpus::get() as isize;
        if !use_composite {
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_i = input_i.clone();
                let input_h = input_h.clone();
                let input_s = input_s.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    // let (mut r, mut g, mut b): (f64, f64, f64);
                    let (mut i, mut h, mut s): (f64, f64, f64);
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut red_data = vec![nodata_i; columns as usize];
                        let mut green_data = vec![nodata_i; columns as usize];
                        let mut blue_data = vec![nodata_i; columns as usize];
                        for col in 0..columns {
                            i = input_i[(row, col)];
                            h = input_h[(row, col)];
                            s = input_s[(row, col)];
                            if i != nodata_i && h != nodata_h && s != nodata_s {
                                let (r, g, b) = hsi2rgb(h, s, i);

                                red_data[col as usize] = r as f64;
                                green_data[col as usize] = g as f64;
                                blue_data[col as usize] = b as f64;
                            }
                        }
                        tx.send((row, red_data, green_data, blue_data)).unwrap();
                    }
                });
            }

            let mut output_r = Raster::initialize_using_file(&red_file, &input_i);
            output_r.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_r.configs.data_type = DataType::F32;

            let mut output_g = Raster::initialize_using_file(&green_file, &input_i);
            output_g.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_g.configs.data_type = DataType::F32;

            let mut output_b = Raster::initialize_using_file(&blue_file, &input_i);
            output_b.configs.photometric_interp = PhotometricInterpretation::Continuous;
            output_b.configs.data_type = DataType::F32;

            for row in 0..rows {
                let data = rx.recv().unwrap();
                output_r.set_row_data(data.0, data.1);
                output_g.set_row_data(data.0, data.2);
                output_b.set_row_data(data.0, data.3);
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Progress: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time = get_formatted_elapsed_time(start);

            output_r.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output_r.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_r.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_r
                .add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_r.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

            if verbose {
                println!("Saving red data...")
            };
            let _ = match output_r.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };

            output_g.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output_g.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_g.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_g
                .add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_g.add_metadata_entry(
                format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
            );

            if verbose {
                println!("Saving green data...")
            };
            let _ = match output_g.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };

            output_b.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output_b.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output_b.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output_b
                .add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
            output_b.add_metadata_entry(
                format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""),
            );

            if verbose {
                println!("Saving blue data...")
            };
            let _ = match output_b.write() {
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
                    &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", "")
                );
            }
        } else {
            if verbose {
                println!("Creating a colour composite output...");
            }
            let (tx, rx) = mpsc::channel();
            for tid in 0..num_procs {
                let input_i = input_i.clone();
                let input_h = input_h.clone();
                let input_s = input_s.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let (mut i, mut h, mut s): (f64, f64, f64);
                    let mut value: f64;
                    for row in (0..rows).filter(|r| r % num_procs == tid) {
                        let mut data = vec![0f64; columns as usize];
                        for col in 0..columns {
                            i = input_i[(row, col)];
                            h = input_h[(row, col)];
                            s = input_s[(row, col)];
                            if i != nodata_i && h != nodata_h && s != nodata_s {
                                value = hsi2value(h, s, i);
                                data[col as usize] = value;
                            }
                        }
                        tx.send((row, data)).unwrap();
                    }
                });
            }

            let mut output = Raster::initialize_using_file(&composite_file, &input_i);
            output.configs.photometric_interp = PhotometricInterpretation::RGB;
            output.configs.nodata = 0f64;
            output.configs.data_type = DataType::RGBA32;
            for row in 0..rows {
                let data = rx.recv().unwrap();
                output.set_row_data(data.0, data.1);
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
            output.add_metadata_entry(format!("Input intensity image file: {}", intensity_file));
            output.add_metadata_entry(format!("Input hue image file: {}", hue_file));
            output.add_metadata_entry(format!("Input saturation image file: {}", saturation_file));
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
                    &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                );
            }
        }

        Ok(())
    }
}

// fn value2hsi(value: f64) -> (f64, f64, f64) {
//     let r = (value as u32 & 0xFF) as f64 / 255f64;
//     let g = ((value as u32 >> 8) & 0xFF) as f64 / 255f64;
//     let b = ((value as u32 >> 16) & 0xFF) as f64 / 255f64;

//     let i = (r + g + b) / 3f64;

//     let rn = r / (r + g + b);
//     let gn = g / (r + g + b);
//     let bn = b / (r + g + b);

//     let mut h = if rn != gn || rn != bn {
//         ((0.5 * ((rn - gn) + (rn - bn))) / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt())
//             .acos()
//     } else {
//         0f64
//     };
//     if b > g {
//         h = 2f64 * PI - h;
//     }

//     let s = 1f64 - 3f64 * rn.min(gn).min(bn);

//     (h, s, i)
// }

fn hsi2value(h: f64, s: f64, i: f64) -> f64 {
    let mut r: u32;
    let mut g: u32;
    let mut b: u32;

    let x = i * (1f64 - s);

    if h < 2f64 * PI / 3f64 {
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (y * 255f64).round() as u32;
        g = (z * 255f64).round() as u32;
        b = (x * 255f64).round() as u32;
    } else if h < 4f64 * PI / 3f64 {
        let h = h - 2f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (x * 255f64).round() as u32;
        g = (y * 255f64).round() as u32;
        b = (z * 255f64).round() as u32;
    } else {
        let h = h - 4f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (z * 255f64).round() as u32;
        g = (x * 255f64).round() as u32;
        b = (y * 255f64).round() as u32;
    }

    if r > 255u32 {
        r = 255u32;
    }
    if g > 255u32 {
        g = 255u32;
    }
    if b > 255u32 {
        b = 255u32;
    }

    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}

fn hsi2rgb(h: f64, s: f64, i: f64) -> (u32, u32, u32) {
    let mut r: u32;
    let mut g: u32;
    let mut b: u32;

    let x = i * (1f64 - s);

    if h < 2f64 * PI / 3f64 {
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (y * 255f64).round() as u32;
        g = (z * 255f64).round() as u32;
        b = (x * 255f64).round() as u32;
    } else if h < 4f64 * PI / 3f64 {
        let h = h - 2f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (x * 255f64).round() as u32;
        g = (y * 255f64).round() as u32;
        b = (z * 255f64).round() as u32;
    } else {
        let h = h - 4f64 * PI / 3f64;
        let y = i * (1f64 + (s * h.cos()) / ((PI / 3f64 - h).cos()));
        let z = 3f64 * i - (x + y);
        r = (z * 255f64).round() as u32;
        g = (x * 255f64).round() as u32;
        b = (y * 255f64).round() as u32;
    }

    if r > 255u32 {
        r = 255u32;
    }
    if g > 255u32 {
        g = 255u32;
    }
    if b > 255u32 {
        b = 255u32;
    }

    (r, g, b)
}