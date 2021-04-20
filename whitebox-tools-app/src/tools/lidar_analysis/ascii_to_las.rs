/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 10/02/2019
Last Modified: 18/01/2020
License: MIT
*/

use whitebox_lidar::*;
use crate::tools::*;
use std;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{Error, ErrorKind};
use std::path;
extern crate byteorder;
use whitebox_common::spatial_ref_system::esri_wkt_from_epsg;

/// This tool can be used to convert one or more ASCII files, containing LiDAR point data, into LAS files. The user must
/// specify the name(s) of the input ASCII file(s) (`--inputs`). Each input file will have a correspondingly named
/// output file with a `.las` file extension. The output point data, each on a separate line, will take the format:
///
/// ```
/// x,y,z,intensity,class,return,num_returns"
/// ```
///
/// | Value | Interpretation    |
/// | :---- | :---------------- |
/// | x     | x-coordinate      |
/// | y     | y-coordinate      |
/// | z     | elevation         |
/// | i     | intensity value   |
/// | c     | classification    |
/// | rn    | return number     |
/// | nr    | number of returns |
/// | time  | GPS time          |
/// | sa    | scan angle        |
/// | r     | red               |
/// | b     | blue              |
/// | g     | green             |
///
/// The `x`, `y`, and `z` patterns must always be specified. If the `rn` pattern is used, the `nr` pattern must
/// also be specified. Examples of valid pattern string include:
///
/// ```
/// 'x,y,z,i'
/// 'x,y,z,i,rn,nr'
/// 'x,y,z,i,c,rn,nr,sa'
/// 'z,x,y,rn,nr'
/// 'x,y,z,i,rn,nr,r,g,b'
/// ```
///
/// Use the `LasToAscii` tool to convert a LAS file into a text file containing LiDAR point data.
///
/// # See Also
/// `LasToAscii`
pub struct AsciiToLas {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl AsciiToLas {
    pub fn new() -> AsciiToLas {
        // public constructor
        let name = "AsciiToLas".to_string();
        let toolbox = "LiDAR Tools".to_string();
        let description =
            "Converts one or more ASCII files containing LiDAR points into LAS files.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input LiDAR point ASCII Files (.csv)".to_owned(),
            flags: vec!["-i".to_owned(), "--inputs".to_owned()],
            description: "Input LiDAR  ASCII files (.csv).".to_owned(),
            parameter_type: ParameterType::FileList(ParameterFileType::Csv),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Pattern".to_owned(),
            flags: vec!["--pattern".to_owned()],
            description: "Input field pattern.".to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Well-known-text (WKT) string or EPSG code".to_owned(),
            flags: vec!["--proj".to_owned()],
            description: "Well-known-text string or EPSG code describing projection.".to_owned(),
            parameter_type: ParameterType::String,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" -i=\"file1.las, file2.las, file3.las\" -o=outfile.las\" --proj=2150", short_exe, name).replace("*", &sep);

        AsciiToLas {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for AsciiToLas {
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
        let mut input_files: String = String::new();
        let mut pattern_string = String::new();
        let mut proj_string = String::new();

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
            if flag_val == "-i" || flag_val == "-inputs" || flag_val == "-input" {
                input_files = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-pattern" {
                pattern_string = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };
            } else if flag_val == "-proj" {
                proj_string = if keyval {
                    vec[1].to_string().to_lowercase()
                } else {
                    args[i + 1].to_string().to_lowercase()
                };

                if proj_string.parse::<u16>().is_ok() {
                    proj_string = esri_wkt_from_epsg(
                        proj_string
                            .trim()
                            .parse::<u16>()
                            .expect("Error parsing EPSG code."),
                    );
                    if proj_string.to_lowercase() == "unknown epsg code" {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Error: The specified EPSG is unrecognized or unsupported. Please report this error.",
                        ));
                    }
                }
            }
        }

        if verbose {
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
            println!("* Welcome to {} *", self.get_tool_name());
            println!("***************{}", "*".repeat(self.get_tool_name().len()));
        }

        let sep = std::path::MAIN_SEPARATOR;

        let mut progress: usize;
        let mut old_progress: usize = 1;

        let start = Instant::now();

        if pattern_string.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: The interpretation pattern string (e.g. 'x,y,z,i,c,rn,nr,sa') was not specified.",
            ));
        }

        // Do some quality control on the pattern
        if !pattern_string.contains("x")
            || !pattern_string.contains("y")
            || !pattern_string.contains("z")
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: The interpretation pattern string (e.g. 'x,y,z,i,c,rn,nr,sa') must contain x, y, and z fields.",
            ));
        }

        if (pattern_string.contains("rn") && !pattern_string.contains("nr"))
            || (pattern_string.contains("nr") && !pattern_string.contains("rn"))
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Error: If the interpretation pattern string (e.g. 'x,y,z,i,rn,nr,time') contains a 'rn' field it must also contain a 'nr' field and vice versa.",
            ));
        }

        let pattern_vec = pattern_string.trim().split(",").collect::<Vec<&str>>();
        let num_fields = pattern_vec.len();

        // convert the string pattern to numeric for easier look-up
        let mut pattern_numeric = vec![0usize; num_fields];
        let mut pattern_has_time = false;
        let mut pattern_has_r = false;
        let mut pattern_has_g = false;
        let mut pattern_has_b = false;
        let mut pattern_has_i = false;
        for a in 0..num_fields {
            match pattern_vec[a] {
                "x" => pattern_numeric[a] = 0usize,
                "y" => pattern_numeric[a] = 1usize,
                "z" => pattern_numeric[a] = 2usize,
                "i" => {
                    pattern_has_i = true;
                    pattern_numeric[a] = 3usize
                }
                "c" => pattern_numeric[a] = 4usize,
                "rn" => pattern_numeric[a] = 5usize,
                "nr" => pattern_numeric[a] = 6usize,
                "time" => {
                    pattern_has_time = true;
                    pattern_numeric[a] = 7usize
                }
                "sa" => pattern_numeric[a] = 8usize,
                "r" => {
                    pattern_has_r = true;
                    pattern_numeric[a] = 9usize
                }
                "g" => {
                    pattern_has_g = true;
                    pattern_numeric[a] = 10usize
                }
                "b" => {
                    pattern_has_b = true;
                    pattern_numeric[a] = 11usize
                }
                _ => println!("Unrecognized pattern {}", pattern_vec[a]),
            }
        }

        // if the pattern contain any of 'r', 'g', or 'b', it must also contain all of 'r', 'g', and 'b'.
        let mut pattern_has_clr = false;
        if pattern_has_r && pattern_has_g && pattern_has_b {
            pattern_has_clr = true;
        } else if pattern_has_r || pattern_has_g || pattern_has_b {
            // you can't have one and not all
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "If any of r, g, or b are provided, each of r, g, and b must be provided.",
            ));
        }

        let mut cmd = input_files.split(";");
        let mut files_vec = cmd.collect::<Vec<&str>>();
        if files_vec.len() == 1 {
            cmd = input_files.split(",");
            files_vec = cmd.collect::<Vec<&str>>();
        }
        let mut i = 1;
        let num_files = files_vec.len();
        for value in files_vec {
            if !value.trim().is_empty() {
                let mut input_file = value.trim().to_owned();
                if !input_file.contains(sep) && !input_file.contains("/") {
                    input_file = format!("{}{}", working_directory, input_file);
                }

                // Initialize the output LAS file
                let file_extension = get_file_extension(&input_file);
                let output_file = input_file.replace(&format!(".{}", file_extension), ".las");
                let mut output = LasFile::new(&output_file, "w")?;
                let mut header: LasHeader = Default::default();
                let point_format = if !pattern_has_time && !pattern_has_clr {
                    header.point_format = 0;
                    0u8
                } else if pattern_has_time && !pattern_has_clr {
                    header.point_format = 1;
                    1u8
                } else if !pattern_has_time && pattern_has_clr {
                    header.point_format = 2;
                    2u8
                } else {
                    // if pattern_has_time && pattern_has_clr {
                    header.point_format = 3;
                    3u8
                };
                header.project_id_used = true;
                header.x_scale_factor = 0.0001;
                header.y_scale_factor = 0.0001;
                header.z_scale_factor = 0.0001;
                output.add_header(header);
                if !proj_string.is_empty() {
                    output.wkt = proj_string.clone();
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Error: Projection string was unspecified. Please specify either a WKT string or EPSG code.",
                    ));
                }
                output.use_point_intensity = pattern_has_i;
                output.use_point_userdata = true;

                if verbose {
                    println!("Parsing file {}...", output.get_short_filename());
                }
                let f = File::open(input_file.clone())?;
                let f = BufReader::new(f);

                // let mut point_num = 0;
                let mut read_first_point_x = false;
                let mut read_first_point_y = false;
                let mut read_first_point_z = false;
                let mut coord_val: f64;
                for line in f.lines() {
                    let line_unwrapped = line?;
                    let line_data = line_unwrapped.split(",").collect::<Vec<&str>>();
                    if line_data.len() >= num_fields {
                        // check to see if the first field contains a number; if not, it's likely a header row and should be ignored
                        let is_num = line_data[0].parse::<f64>().is_ok();
                        if is_num {
                            let mut point_data: PointData = Default::default();
                            let mut gps_time = 0f64;
                            let mut clr_data: ColourData = Default::default();
                            // now convert each of the specified fields based on the input pattern
                            for a in 0..num_fields {
                                match pattern_numeric[a] {
                                    0usize => {
                                        coord_val = line_data[a].trim().parse::<f64>().expect("Error parsing data.");
                                        if !read_first_point_x {
                                            output.header.x_offset = coord_val;
                                            read_first_point_x = true;
                                        }
                                        point_data.x = ((coord_val - output.header.x_offset) / output.header.x_scale_factor) as i32;
                                    }
                                    1usize => {
                                        coord_val = line_data[a].trim().parse::<f64>().expect("Error parsing data.");
                                        if !read_first_point_y {
                                            output.header.y_offset = coord_val;
                                            read_first_point_y = true;
                                        }
                                        point_data.y = ((coord_val - output.header.y_offset) / output.header.y_scale_factor) as i32;
                                    }
                                    2usize => {
                                        coord_val = line_data[a].trim().parse::<f64>().expect("Error parsing data.");
                                        if !read_first_point_z {
                                            output.header.z_offset = coord_val;
                                            read_first_point_z = true;
                                        }
                                        point_data.z = ((coord_val - output.header.z_offset) / output.header.z_scale_factor) as i32;
                                    }
                                    3usize => {
                                        point_data.intensity = line_data[a]
                                            .trim()
                                            .parse::<u16>()
                                            .expect("Error parsing data.")
                                    }
                                    4usize => point_data.set_classification(
                                        line_data[a]
                                            .trim()
                                            .parse::<u8>()
                                            .expect("Error parsing data."),
                                    ),
                                    5usize => point_data.set_return_number(
                                        line_data[a]
                                            .trim()
                                            .parse::<u8>()
                                            .expect("Error parsing data."),
                                    ),
                                    6usize => point_data.set_number_of_returns(
                                        line_data[a]
                                            .trim()
                                            .parse::<u8>()
                                            .expect("Error parsing data."),
                                    ),
                                    7usize => {
                                        gps_time = line_data[a]
                                            .trim()
                                            .parse::<f64>()
                                            .expect("Error parsing data.")
                                    }
                                    8usize => {
                                        point_data.scan_angle = line_data[a]
                                            .trim()
                                            .parse::<i16>()
                                            .expect("Error parsing data.")
                                    }
                                    9usize => {
                                        clr_data.red = line_data[a]
                                            .trim()
                                            .parse::<u16>()
                                            .expect("Error parsing data.")
                                    }
                                    10usize => {
                                        clr_data.green = line_data[a]
                                            .trim()
                                            .parse::<u16>()
                                            .expect("Error parsing data.")
                                    }
                                    11usize => {
                                        clr_data.blue = line_data[a]
                                            .trim()
                                            .parse::<u16>()
                                            .expect("Error parsing data.")
                                    }
                                    _ => println!("unrecognized pattern"),
                                }
                            }

                            point_data.point_source_id = 1;

                            // point_num += 1;
                            match point_format {
                                0 => {
                                    output.add_point_record(LidarPointRecord::PointRecord0 {
                                        point_data: point_data,
                                    });
                                }
                                1 => {
                                    output.add_point_record(LidarPointRecord::PointRecord1 {
                                        point_data: point_data,
                                        gps_data: gps_time,
                                    });
                                }
                                2 => {
                                    output.add_point_record(LidarPointRecord::PointRecord2 {
                                        point_data: point_data,
                                        colour_data: clr_data,
                                    });
                                }
                                3 => {
                                    // if pattern_has_time && pattern_has_clr {
                                    output.add_point_record(LidarPointRecord::PointRecord3 {
                                        point_data: point_data,
                                        gps_data: gps_time,
                                        colour_data: clr_data,
                                    });
                                }
                                _ => {
                                    panic!("Error: Unrecognized point format.");
                                }
                            };
                        }
                    } // else ignore the line.
                }

                let mut vlr1: Vlr = Default::default();
                // vlr1.reserved = 0u16;
                vlr1.user_id = String::from("LASF_Projection");
                vlr1.record_id = 2112u16;
                vlr1.description = String::from("OGC WKT Coordinate System");
                vlr1.binary_data = (format!("{}\0", proj_string)).as_bytes().to_vec();
                vlr1.record_length_after_header = vlr1.binary_data.len() as u16;
                output.add_vlr(vlr1);

                // let mut vlr1: Vlr = Default::default();
                // vlr1.reserved = 0u16;
                // vlr1.user_id = String::from("LASF_Projection");
                // vlr1.record_id = 34735u16;
                // vlr1.description = String::from("GeoTiff Projection Keys");
                // let raw_data = vec![1u16, 1, 0, 23, 1024, 0, 1, 1, 2048, 0, 1, 4269, 2049, 34737, 24, 64, 2050, 0, 1, 6269, 2051, 0, 1, 8901, 2054, 0, 1, 9102, 2055, 34736, 1, 9, 2056, 0, 1, 7019, 2057, 34736, 1, 6, 2059, 34736, 1, 7, 2061, 34736, 1, 8, 3072, 0, 1, 32145, 3073, 34737, 38, 0, 3075, 0, 1, 1, 3076, 0, 1, 9001, 3077, 34736, 1, 5, 3081, 34736, 1, 4, 3082, 34736, 1, 0, 3083, 34736, 1, 1, 3088, 34736, 1, 2, 3092, 34736, 1, 3, 4097, 34737, 26, 38, 4099, 0, 1, 9001];
                // let mut v8: Vec<u8> = Vec::new();
                // for n in raw_data {
                //     v8.write_u16::<LittleEndian>(n).unwrap();
                // }
                // vlr1.binary_data = v8;
                // vlr1.record_length_after_header = vlr1.binary_data.len() as u16; //192;
                // println!("vlr1 (192): {} {}", vlr1.binary_data.len(), vlr1.record_length_after_header);
                // output.add_vlr(vlr1);

                // let mut vlr2: Vlr = Default::default();
                // vlr2.reserved = 0u16;
                // vlr2.user_id = String::from("LASF_Projection");
                // vlr2.record_id = 34736u16;
                // vlr2.description = String::from("GeoTiff double parameters");
                // let raw_data = vec![500000f64, 0f64, -72.5f64, 0.9999642857142857f64, 42.5f64, 1f64, 6378137f64, 298.257222101f64, 0f64, 0.017453292519943295f64];
                // let mut v8: Vec<u8> = Vec::new();
                // for n in raw_data {
                //     v8.write_f64::<LittleEndian>(n).unwrap();
                // }
                // vlr2.binary_data = v8;
                // vlr2.record_length_after_header = vlr2.binary_data.len() as u16; //80;
                // println!("vlr2 (80): {} {}", vlr2.binary_data.len(), vlr2.record_length_after_header);
                // output.add_vlr(vlr2);

                // let mut vlr3: Vlr = Default::default();
                // vlr3.reserved = 0u16;
                // vlr3.user_id = String::from("LASF_Projection");
                // vlr3.record_id = 34737u16;
                // vlr3.description = String::from("GeoTiff ASCII parameters");
                // vlr3.binary_data = "NAD_1983_StatePlane_Vermont_FIPS_4400|NAVD88 - Geoid09 (Meters)|GCS_North_American_1983|".as_bytes().to_vec();
                // vlr3.record_length_after_header = vlr3.binary_data.len() as u16; //89;
                // println!("vlr3 (89): {} {}", vlr3.binary_data.len(), vlr3.record_length_after_header);
                // output.add_vlr(vlr3);

                if verbose {
                    println!("Writing output LAS file {}...", output.get_short_filename());
                }
                let _ = match output.write() {
                    Ok(_) => {
                        if verbose {
                            println!("Complete!")
                        }
                    }
                    Err(e) => println!("error while writing: {:?}", e),
                };
            }
            if verbose {
                progress = (100.0_f64 * (i + 1) as f64 / num_files as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
            i += 1;
        }

        if verbose {
            let elapsed_time = get_formatted_elapsed_time(start);
            println!("{}", &format!("Elapsed Time: {}", elapsed_time));
        }

        Ok(())
    }
}

/// Returns the file extension.
pub fn get_file_extension(file_name: &str) -> String {
    let file_path = std::path::Path::new(file_name);
    let extension = file_path.extension().unwrap();
    let e = extension.to_str().unwrap();
    e.to_string()
}
