/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 09/07/2020
Last Modified: 14/07/2020
License: MIT
*/

use crate::raster::*;
use crate::structures::Array2D;
use crate::tools::*;
use num_cpus;
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand_distr::Uniform;
use std::env;
use std::f64;
use std::f64::consts::PI;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

/// This tool creates a colour shaded relief (Swiss hillshading) image from an input digital elevation model (DEM).
/// The tool combines a colourized version of the DEM with varying illumination provided by a hillshade image, to
/// produce a composite relief model that can be used to visual topography for more effective interpretation of
/// landscapes. The output (`--output`) of the tool is a 24-bit red-green-blue (RGB) colour image.
///
/// The user must specify the name of the input DEM and the output image name. Other parameters that must
/// be specified include the illumination source azimuth (`--azimuth`), or sun direction (0-360 degrees), the
/// illumination source altitude (`--altitude`; i.e. the elevation of the sun above the horizon, measured as an angle
/// from 0 to 90 degrees), the hillshade weight (`--hs_weight`; 0-1), image brightness (`--brightness`; 0-1), and atmospheric
/// effects (`--atmospheric`; 0-1). The hillshade weight can be used to increase or subdue the relative prevalence of the
/// hillshading effect in the output image. The image brightness parameter is used to create an overall brighter or
/// darker version of the terrain rendering; note however, that very high values may over-saturate the well-illuminated
/// portions of the terrain. The atmospheric effects parameter can be used to introduce a haze or atmosphere effect to 
/// the output image. It is intended to reproduce the effect of viewing mountain valley bottoms through a thicker and 
/// more dense atmosphere. Values greater than zero will introduce a slightly blue tint, particularly at lower altitudes,
/// blur the hillshade edges slightly, and create a random haze-like speckle in lower areas. The user must also specify 
/// the Z conversion factor (`--zfactor`). The *Z conversion factor* is only important when the vertical and horizontal 
/// units are not the same in the DEM. When this is the case, the algorithm will multiply each elevation in the DEM by the 
/// Z conversion factor. If the DEM is in the geographic coordinate system (latitude and longitude), the following equation
/// is used:
///
/// > zfactor = 1.0 / (113200.0 x cos(mid_lat))
///
/// where `mid_lat` is the latitude of the centre of the raster, in radians.
///
/// # See Also
/// `Hillshade`, `Aspect`, `Slope`
pub struct ColourShadedRelief {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ColourShadedRelief {
    pub fn new() -> ColourShadedRelief {
        // public constructor
        let name = "ColourShadedRelief".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description = "Creates an colour shaded relief image from an input DEM.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
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

        // parameters.push(ToolParameter {
        //     name: "Illumination Source Azimuth (degrees)".to_owned(),
        //     flags: vec!["--azimuth".to_owned()],
        //     description: "Illumination source azimuth in degrees.".to_owned(),
        //     parameter_type: ParameterType::Float,
        //     default_value: Some("315.0".to_owned()),
        //     optional: true,
        // });

        parameters.push(ToolParameter {
            name: "Illumination Source Altitude (degrees)".to_owned(),
            flags: vec!["--altitude".to_owned()],
            description: "Illumination source altitude in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("45.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Hillshade Weight".to_owned(),
            flags: vec!["--hs_weight".to_owned()],
            description: "Weight given to hillshade relative to relief (0.0-1.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Brightness".to_owned(),
            flags: vec!["--brightness".to_owned()],
            description: "Brightness factor (0.0-1.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.5".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Atmospheric Effects".to_owned(),
            flags: vec!["--atmospheric".to_owned()],
            description: "Atmospheric effects weight (0.0-1.0).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("0.0".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Palette".to_owned(),
            flags: vec!["--palette".to_owned()],
            description:
                "Options include 'atlas', 'high_relief', 'arid', 'soft', 'muted', 'purple', 'viridi', 'gn_yl', 'pi_y_g', 'bl_yl_rd', and 'deep."
                    .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "atlas".to_owned(),
                "high_relief".to_owned(),
                "arid".to_owned(),
                "soft".to_owned(),
                "muted".to_owned(),
                "purple".to_owned(),
                "viridi".to_owned(),
                "gn_yl".to_owned(),
                "pi_y_g".to_owned(),
                "bl_yl_rd".to_owned(),
                "deep".to_owned()
            ]),
            default_value: Some("atlas".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Reverse palette?".to_owned(),
            flags: vec!["--reverse".to_owned()],
            description: "Optional flag indicating whether to use reverse the palette."
                .to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Z Conversion Factor".to_owned(),
            flags: vec!["--zfactor".to_owned()],
            description:
                "Optional multiplier for when the vertical and horizontal units are not the same."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("1.0".to_owned()),
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
        let usage = format!(">>.*{} -r={} -v --wd=\"*path*to*data*\" -i=DEM.tif -o=output.tif --azimuth=315.0 --altitude=45.0 --hs_weight=0.3 --brightness=0.6 --atmospheric=0.2 --palette=arid", short_exe, name).replace("*", &sep);

        ColourShadedRelief {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ColourShadedRelief {
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
        let mut output_file = String::new();
        // let mut azimuth = 315.0f64;
        let mut altitude = 45.0f64;
        let mut z_factor = 1f64;
        let mut hs_alpha = 0.5f32;
        let mut brightness = 0.5f32;
        let mut atmospheric_alpha = 0.0f32;
        let mut palette = String::from("atlas");
        let mut reverse_palette = false;

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
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                if keyval {
                    input_file = vec[1].to_string();
                } else {
                    input_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            // } else if flag_val == "-azimuth" {
            //     if keyval {
            //         azimuth = vec[1]
            //             .to_string()
            //             .parse::<f64>()
            //             .expect(&format!("Error parsing {}", flag_val));
            //     } else {
            //         azimuth = args[i + 1]
            //             .to_string()
            //             .parse::<f64>()
            //             .expect(&format!("Error parsing {}", flag_val));
            //     }
            } else if flag_val == "-altitude" {
                if keyval {
                    altitude = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    altitude = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
            } else if flag_val == "-hs_weight" {
                hs_alpha = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                if hs_alpha < 0.0 {
                    hs_alpha = 0.0;
                }
                if hs_alpha > 1.0 {
                    hs_alpha = 1.0;
                }
            } else if flag_val == "-brightness" {
                brightness = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                if brightness < 0.0 {
                    brightness = 0.0;
                }
                if brightness > 1.0 {
                    brightness = 1.0;
                }
            } else if flag_val == "-atmospheric" {
                atmospheric_alpha = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f32>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                if atmospheric_alpha < 0.0 {
                    atmospheric_alpha = 0.0;
                }
                if atmospheric_alpha > 1.0 {
                    atmospheric_alpha = 1.0;
                }
            } else if flag_val == "-palette" {
                palette = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                palette = palette.to_lowercase();
            } else if flag_val == "-reverse" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    reverse_palette = true;
                }
            } else if flag_val == "-zfactor" {
                if keyval {
                    z_factor = vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                } else {
                    z_factor = args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val));
                }
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

        let relief_alpha = 1f32 - hs_alpha;

        if verbose {
            println!("Reading data...")
        };

        let input = Arc::new(Raster::new(&input_file, "r")?);

        let start = Instant::now();

        // azimuth = (azimuth - 90f64).to_radians();
        altitude = altitude.to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        let eight_grid_res = input.configs.resolution_x * 8.0;

        if input.is_in_geographic_coordinates() {
            // calculate a new z-conversion factor
            let mut mid_lat = (input.configs.north - input.configs.south) / 2.0;
            if mid_lat <= 90.0 && mid_lat >= -90.0 {
                mid_lat = mid_lat.to_radians();
                z_factor = 1.0 / (113200.0 * mid_lat.cos());
            }
        }

        let rows = input.configs.rows as isize;
        let columns = input.configs.columns as isize;
        let mut hs: Array2D<i16> = Array2D::new(rows, columns, -32768i16, -32768i16)?;
        let multidirection360mode = true;
        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let nodata = input.configs.nodata;
                let out_nodata = -32768i16;
                let columns = input.configs.columns as isize;
                let dx = [1, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

                let azimuths = if multidirection360mode {
                    vec![
                        (0f64 - 90f64).to_radians(), 
                        (45f64 - 90f64).to_radians(), 
                        (90f64 - 90f64).to_radians(), 
                        (135f64 - 90f64).to_radians(),
                        (180f64 - 90f64).to_radians(),
                        (225f64 - 90f64).to_radians(), 
                        (270f64 - 90f64).to_radians(), 
                        (315f64 - 90f64).to_radians(),
                    ]
                } else { 
                    vec![
                        (225f64 - 90f64).to_radians(), 
                        (270f64 - 90f64).to_radians(), 
                        (315f64 - 90f64).to_radians(), 
                        (360f64 - 90f64).to_radians()
                    ]
                };

                let weights = if multidirection360mode {
                    vec![
                        0.15f64, 
                        0.125f64, 
                        0.1f64, 
                        0.05f64,
                        0.1f64, 
                        0.125f64, 
                        0.15f64, 
                        0.20f64,
                    ]
                } else {
                    vec![
                        0.1f64, 
                        0.4f64, 
                        0.4f64, 
                        0.1f64
                    ]
                };

                let mut n: [f64; 8] = [0.0; 8];
                let mut z: f64;
                let mut azimuth: f64;
                let (mut term1, mut term2, mut term3): (f64, f64, f64);
                let (mut fx, mut fy): (f64, f64);
                let mut tan_slope: f64;
                let mut aspect: f64;
                let half_pi = PI / 2f64;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![out_nodata; columns as usize];
                    for col in 0..columns {
                        z = input.get_value(row, col);
                        if z != nodata {
                            z = z * z_factor;
                            for c in 0..8 {
                                n[c] = input.get_value(row + dy[c], col + dx[c]);
                                if n[c] != nodata {
                                    n[c] = n[c] * z_factor;
                                } else {
                                    n[c] = z;
                                }
                            }
                            // calculate slope and aspect
                            fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                            fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                            tan_slope = (fx * fx + fy * fy).sqrt();
                            if tan_slope < 0.00017 {
                                tan_slope = 0.00017;
                            }
                            aspect = if fx != 0f64 {
                                PI - ((fy / fx).atan()) + half_pi * (fx / (fx).abs())
                            } else {
                                PI
                            };
                            term1 = tan_slope / (1f64 + tan_slope * tan_slope).sqrt();
                            term2 = sin_theta / tan_slope;
                            z = 0f64;
                            for a in 0..azimuths.len() {
                                azimuth = azimuths[a];
                                term3 = cos_theta * (azimuth - aspect).sin();
                                z += term1 * (term2 - term3) * weights[a];
                            }
                            z = z * 32767.0;
                            if z < 0.0 {
                                z = 0.0;
                            }
                            data[col as usize] = z.round() as i16;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut histo: [f64; 32768] = [0.0; 32768];
        let mut num_cells = 0.0;
        let mut histo_elev: [f64; 32768] = [0.0; 32768];
        let elev_min = input.configs.minimum;
        let elev_max = input.configs.maximum;
        let elev_range = elev_max - elev_min;
        let mut elev: f64;
        let mut bin: usize;
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            for col in 0..data.1.len() {
                if data.1[col] != -32768 {
                    bin = data.1[col] as usize;
                    histo[bin] += 1.0;
                    num_cells += 1.0;
                    elev = input.get_value(data.0, col as isize);
                    bin = (((elev - elev_min) / elev_range) * 32767f64).round() as usize;
                    histo_elev[bin] += 1.0;
                }
            }
            hs.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut new_min = 0i16;
        let mut new_max = 32767i16;
        let clip_percent = 0.005;
        let mut target_cell_num = num_cells * clip_percent;
        let mut sum = 0.0;
        for c in 0..32768 {
            sum += histo[c];
            if sum >= target_cell_num {
                new_min = c as i16;
                break;
            }
        }

        target_cell_num = num_cells as f64 * 0.10f64 * brightness as f64;
        sum = 0.0;
        for c in (0..32768).rev() {
            sum += histo[c];
            if sum >= target_cell_num {
                new_max = c as i16;
                break;
            }
        }

        // let mu = (new_min + new_max) as f32 / 2f32;
        // let s = 0.33f32 * ((new_max - new_min) as f32 / 2f32);

        let mut new_elev_min = 0f64;
        let mut new_elev_max = 0f64;
        target_cell_num = num_cells * clip_percent;
        sum = 0.0;
        for c in 0..32768 {
            sum += histo_elev[c];
            if sum >= target_cell_num {
                new_elev_min = elev_min + (c as f64 / 32768f64) * elev_range;
                break;
            }
        }

        sum = 0.0;
        for c in (0..32768).rev() {
            sum += histo_elev[c];
            if sum >= target_cell_num {
                new_elev_max = elev_min + (c as f64 / 32768f64) * elev_range;
                break;
            }
        }

        let new_elev_range = new_elev_max - new_elev_min;

        let mut output = Raster::initialize_using_file(&output_file, &input);
        let out_nodata = 0f64;
        output.configs.nodata = out_nodata;
        output.configs.photometric_interp = PhotometricInterpretation::RGB;
        output.configs.data_type = DataType::RGB24;

        let hs = Arc::new(hs);
        let palette = Arc::new(palette);
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input = input.clone();
            let hs = hs.clone();
            let palette = palette.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let mut elev_palette = if palette.contains("atlas") {
                    vec![
                        (72f32, 135f32, 55f32),
                        (226f32, 219f32, 171f32),
                        (228f32, 180f32, 123f32),
                    ]
                } else if palette.contains("high") {
                    vec![
                        (72f32, 135f32, 55f32),
                        (226f32, 219f32, 171f32),
                        (228f32, 180f32, 123f32),
                        (182f32, 156f32, 144f32),
                        (255f32, 255f32, 255f32),
                    ]
                } else if palette.contains("arid") {
                    vec![
                        (119f32, 101f32, 91f32), // (152f32, 148f32, 120f32),
                        (254f32, 213f32, 132f32),
                        (254f32, 252f32, 231f32),
                    ]
                } else if palette.contains("soft") {
                    vec![
                        (154f32, 206f32, 111f32),
                        (255f32, 254f32, 211f32),
                        (255f32, 160f32, 100f32),
                    ]
                } else if palette.contains("muted") {
                    // muted
                    vec![
                        (72f32, 136f32, 184f32),
                        (142f32, 199f32, 167f32),
                        (255f32, 254f32, 198f32),
                        (228f32, 116f32, 79f32),
                        (197f32, 74f32, 82f32),
                    ]
                } else if palette.contains("purple") {
                    vec![
                        (118f32, 42f32, 131f32),
                        (153f32, 112f32, 171f32),
                        (194f32, 165f32, 207f32),
                        (231f32, 212f32, 232f32),
                        (247f32, 247f32, 247f32),
                        (217f32, 240f32, 211f32),
                        (166f32, 219f32, 160f32),
                        (90f32, 174f32, 97f32),
                        (27f32, 120f32, 55f32),
                    ]
                } else if palette.contains("viridi") {
                    vec![
                        (68f32, 1f32, 84f32),
                        (68f32, 2f32, 85f32),
                        (68f32, 3f32, 87f32),
                        (69f32, 5f32, 88f32),
                        (69f32, 6f32, 90f32),
                        (69f32, 8f32, 91f32),
                        (70f32, 9f32, 92f32),
                        (70f32, 11f32, 94f32),
                        (70f32, 12f32, 95f32),
                        (70f32, 14f32, 97f32),
                        (71f32, 15f32, 98f32),
                        (71f32, 17f32, 99f32),
                        (71f32, 18f32, 101f32),
                        (71f32, 20f32, 102f32),
                        (71f32, 21f32, 103f32),
                        (71f32, 22f32, 105f32),
                        (71f32, 24f32, 106f32),
                        (72f32, 25f32, 107f32),
                        (72f32, 26f32, 108f32),
                        (72f32, 28f32, 110f32),
                        (72f32, 29f32, 111f32),
                        (72f32, 30f32, 112f32),
                        (72f32, 32f32, 113f32),
                        (72f32, 33f32, 114f32),
                        (72f32, 34f32, 115f32),
                        (72f32, 35f32, 116f32),
                        (71f32, 37f32, 117f32),
                        (71f32, 38f32, 118f32),
                        (71f32, 39f32, 119f32),
                        (71f32, 40f32, 120f32),
                        (71f32, 42f32, 121f32),
                        (71f32, 43f32, 122f32),
                        (71f32, 44f32, 123f32),
                        (70f32, 45f32, 124f32),
                        (70f32, 47f32, 124f32),
                        (70f32, 48f32, 125f32),
                        (70f32, 49f32, 126f32),
                        (69f32, 50f32, 127f32),
                        (69f32, 52f32, 127f32),
                        (69f32, 53f32, 128f32),
                        (69f32, 54f32, 129f32),
                        (68f32, 55f32, 129f32),
                        (68f32, 57f32, 130f32),
                        (67f32, 58f32, 131f32),
                        (67f32, 59f32, 131f32),
                        (67f32, 60f32, 132f32),
                        (66f32, 61f32, 132f32),
                        (66f32, 62f32, 133f32),
                        (66f32, 64f32, 133f32),
                        (65f32, 65f32, 134f32),
                        (65f32, 66f32, 134f32),
                        (64f32, 67f32, 135f32),
                        (64f32, 68f32, 135f32),
                        (63f32, 69f32, 135f32),
                        (63f32, 71f32, 136f32),
                        (62f32, 72f32, 136f32),
                        (62f32, 73f32, 137f32),
                        (61f32, 74f32, 137f32),
                        (61f32, 75f32, 137f32),
                        (61f32, 76f32, 137f32),
                        (60f32, 77f32, 138f32),
                        (60f32, 78f32, 138f32),
                        (59f32, 80f32, 138f32),
                        (59f32, 81f32, 138f32),
                        (58f32, 82f32, 139f32),
                        (58f32, 83f32, 139f32),
                        (57f32, 84f32, 139f32),
                        (57f32, 85f32, 139f32),
                        (56f32, 86f32, 139f32),
                        (56f32, 87f32, 140f32),
                        (55f32, 88f32, 140f32),
                        (55f32, 89f32, 140f32),
                        (54f32, 90f32, 140f32),
                        (54f32, 91f32, 140f32),
                        (53f32, 92f32, 140f32),
                        (53f32, 93f32, 140f32),
                        (52f32, 94f32, 141f32),
                        (52f32, 95f32, 141f32),
                        (51f32, 96f32, 141f32),
                        (51f32, 97f32, 141f32),
                        (50f32, 98f32, 141f32),
                        (50f32, 99f32, 141f32),
                        (49f32, 100f32, 141f32),
                        (49f32, 101f32, 141f32),
                        (49f32, 102f32, 141f32),
                        (48f32, 103f32, 141f32),
                        (48f32, 104f32, 141f32),
                        (47f32, 105f32, 141f32),
                        (47f32, 106f32, 141f32),
                        (46f32, 107f32, 142f32),
                        (46f32, 108f32, 142f32),
                        (46f32, 109f32, 142f32),
                        (45f32, 110f32, 142f32),
                        (45f32, 111f32, 142f32),
                        (44f32, 112f32, 142f32),
                        (44f32, 113f32, 142f32),
                        (44f32, 114f32, 142f32),
                        (43f32, 115f32, 142f32),
                        (43f32, 116f32, 142f32),
                        (42f32, 117f32, 142f32),
                        (42f32, 118f32, 142f32),
                        (42f32, 119f32, 142f32),
                        (41f32, 120f32, 142f32),
                        (41f32, 121f32, 142f32),
                        (40f32, 122f32, 142f32),
                        (40f32, 122f32, 142f32),
                        (40f32, 123f32, 142f32),
                        (39f32, 124f32, 142f32),
                        (39f32, 125f32, 142f32),
                        (39f32, 126f32, 142f32),
                        (38f32, 127f32, 142f32),
                        (38f32, 128f32, 142f32),
                        (38f32, 129f32, 142f32),
                        (37f32, 130f32, 142f32),
                        (37f32, 131f32, 141f32),
                        (36f32, 132f32, 141f32),
                        (36f32, 133f32, 141f32),
                        (36f32, 134f32, 141f32),
                        (35f32, 135f32, 141f32),
                        (35f32, 136f32, 141f32),
                        (35f32, 137f32, 141f32),
                        (34f32, 137f32, 141f32),
                        (34f32, 138f32, 141f32),
                        (34f32, 139f32, 141f32),
                        (33f32, 140f32, 141f32),
                        (33f32, 141f32, 140f32),
                        (33f32, 142f32, 140f32),
                        (32f32, 143f32, 140f32),
                        (32f32, 144f32, 140f32),
                        (32f32, 145f32, 140f32),
                        (31f32, 146f32, 140f32),
                        (31f32, 147f32, 139f32),
                        (31f32, 148f32, 139f32),
                        (31f32, 149f32, 139f32),
                        (31f32, 150f32, 139f32),
                        (30f32, 151f32, 138f32),
                        (30f32, 152f32, 138f32),
                        (30f32, 153f32, 138f32),
                        (30f32, 153f32, 138f32),
                        (30f32, 154f32, 137f32),
                        (30f32, 155f32, 137f32),
                        (30f32, 156f32, 137f32),
                        (30f32, 157f32, 136f32),
                        (30f32, 158f32, 136f32),
                        (30f32, 159f32, 136f32),
                        (30f32, 160f32, 135f32),
                        (31f32, 161f32, 135f32),
                        (31f32, 162f32, 134f32),
                        (31f32, 163f32, 134f32),
                        (32f32, 164f32, 133f32),
                        (32f32, 165f32, 133f32),
                        (33f32, 166f32, 133f32),
                        (33f32, 167f32, 132f32),
                        (34f32, 167f32, 132f32),
                        (35f32, 168f32, 131f32),
                        (35f32, 169f32, 130f32),
                        (36f32, 170f32, 130f32),
                        (37f32, 171f32, 129f32),
                        (38f32, 172f32, 129f32),
                        (39f32, 173f32, 128f32),
                        (40f32, 174f32, 127f32),
                        (41f32, 175f32, 127f32),
                        (42f32, 176f32, 126f32),
                        (43f32, 177f32, 125f32),
                        (44f32, 177f32, 125f32),
                        (46f32, 178f32, 124f32),
                        (47f32, 179f32, 123f32),
                        (48f32, 180f32, 122f32),
                        (50f32, 181f32, 122f32),
                        (51f32, 182f32, 121f32),
                        (53f32, 183f32, 120f32),
                        (54f32, 184f32, 119f32),
                        (56f32, 185f32, 118f32),
                        (57f32, 185f32, 118f32),
                        (59f32, 186f32, 117f32),
                        (61f32, 187f32, 116f32),
                        (62f32, 188f32, 115f32),
                        (64f32, 189f32, 114f32),
                        (66f32, 190f32, 113f32),
                        (68f32, 190f32, 112f32),
                        (69f32, 191f32, 111f32),
                        (71f32, 192f32, 110f32),
                        (73f32, 193f32, 109f32),
                        (75f32, 194f32, 108f32),
                        (77f32, 194f32, 107f32),
                        (79f32, 195f32, 105f32),
                        (81f32, 196f32, 104f32),
                        (83f32, 197f32, 103f32),
                        (85f32, 198f32, 102f32),
                        (87f32, 198f32, 101f32),
                        (89f32, 199f32, 100f32),
                        (91f32, 200f32, 98f32),
                        (94f32, 201f32, 97f32),
                        (96f32, 201f32, 96f32),
                        (98f32, 202f32, 95f32),
                        (100f32, 203f32, 93f32),
                        (103f32, 204f32, 92f32),
                        (105f32, 204f32, 91f32),
                        (107f32, 205f32, 89f32),
                        (109f32, 206f32, 88f32),
                        (112f32, 206f32, 86f32),
                        (114f32, 207f32, 85f32),
                        (116f32, 208f32, 84f32),
                        (119f32, 208f32, 82f32),
                        (121f32, 209f32, 81f32),
                        (124f32, 210f32, 79f32),
                        (126f32, 210f32, 78f32),
                        (129f32, 211f32, 76f32),
                        (131f32, 211f32, 75f32),
                        (134f32, 212f32, 73f32),
                        (136f32, 213f32, 71f32),
                        (139f32, 213f32, 70f32),
                        (141f32, 214f32, 68f32),
                        (144f32, 214f32, 67f32),
                        (146f32, 215f32, 65f32),
                        (149f32, 215f32, 63f32),
                        (151f32, 216f32, 62f32),
                        (154f32, 216f32, 60f32),
                        (157f32, 217f32, 58f32),
                        (159f32, 217f32, 56f32),
                        (162f32, 218f32, 55f32),
                        (165f32, 218f32, 53f32),
                        (167f32, 219f32, 51f32),
                        (170f32, 219f32, 50f32),
                        (173f32, 220f32, 48f32),
                        (175f32, 220f32, 46f32),
                        (178f32, 221f32, 44f32),
                        (181f32, 221f32, 43f32),
                        (183f32, 221f32, 41f32),
                        (186f32, 222f32, 39f32),
                        (189f32, 222f32, 38f32),
                        (191f32, 223f32, 36f32),
                        (194f32, 223f32, 34f32),
                        (197f32, 223f32, 33f32),
                        (199f32, 224f32, 31f32),
                        (202f32, 224f32, 30f32),
                        (205f32, 224f32, 29f32),
                        (207f32, 225f32, 28f32),
                        (210f32, 225f32, 27f32),
                        (212f32, 225f32, 26f32),
                        (215f32, 226f32, 25f32),
                        (218f32, 226f32, 24f32),
                        (220f32, 226f32, 24f32),
                        (223f32, 227f32, 24f32),
                        (225f32, 227f32, 24f32),
                        (228f32, 227f32, 24f32),
                        (231f32, 228f32, 25f32),
                        (233f32, 228f32, 25f32),
                        (236f32, 228f32, 26f32),
                        (238f32, 229f32, 27f32),
                        (241f32, 229f32, 28f32),
                        (243f32, 229f32, 30f32),
                        (246f32, 230f32, 31f32),
                        (248f32, 230f32, 33f32),
                        (250f32, 230f32, 34f32),
                        (253f32, 231f32, 36f32),
                    ]
                } else if palette.contains("gn_yl") {
                    vec![
                        (0f32, 104f32, 55f32),
                        (49f32, 163f32, 84f32),
                        (120f32, 198f32, 121f32),
                        (173f32, 221f32, 142f32),
                        (217f32, 240f32, 163f32),
                        (255f32, 255f32, 204f32),
                    ]
                } else if palette.contains("pi_y_g") {
                    vec![
                        (197f32, 27f32, 125f32),
                        (222f32, 119f32, 174f32),
                        (241f32, 182f32, 218f32),
                        (253f32, 224f32, 239f32),
                        (247f32, 247f32, 247f32),
                        (230f32, 245f32, 208f32),
                        (184f32, 225f32, 134f32),
                        (127f32, 188f32, 65f32),
                        (77f32, 146f32, 33f32),
                    ]
                } else if palette.contains("bl_yl_rd") {
                    vec![
                        (69f32, 117f32, 180f32),
                        (116f32, 173f32, 209f32),
                        (171f32, 217f32, 233f32),
                        (224f32, 243f32, 248f32),
                        (255f32, 255f32, 191f32),
                        (254f32, 224f32, 144f32),
                        (253f32, 174f32, 97f32),
                        (244f32, 109f32, 67f32),
                        (215f32, 48f32, 39f32),
                    ]
                } else {
                    // deep
                    vec![
                        (254f32, 254f32, 215f32),
                        (253f32, 254f32, 213f32),
                        (252f32, 253f32, 210f32),
                        (251f32, 253f32, 208f32),
                        (249f32, 253f32, 205f32),
                        (248f32, 252f32, 203f32),
                        (247f32, 252f32, 200f32),
                        (246f32, 251f32, 198f32),
                        (245f32, 251f32, 195f32),
                        (244f32, 251f32, 194f32),
                        (243f32, 250f32, 190f32),
                        (242f32, 250f32, 189f32),
                        (240f32, 249f32, 185f32),
                        (240f32, 249f32, 184f32),
                        (238f32, 248f32, 180f32),
                        (238f32, 248f32, 179f32),
                        (235f32, 247f32, 177f32),
                        (233f32, 246f32, 177f32),
                        (232f32, 246f32, 177f32),
                        (228f32, 244f32, 177f32),
                        (226f32, 243f32, 177f32),
                        (223f32, 242f32, 178f32),
                        (222f32, 242f32, 178f32),
                        (218f32, 240f32, 178f32),
                        (216f32, 239f32, 178f32),
                        (214f32, 239f32, 178f32),
                        (213f32, 238f32, 178f32),
                        (209f32, 237f32, 179f32),
                        (207f32, 236f32, 179f32),
                        (204f32, 235f32, 179f32),
                        (203f32, 234f32, 179f32),
                        (199f32, 233f32, 179f32),
                        (196f32, 231f32, 180f32),
                        (191f32, 230f32, 180f32),
                        (187f32, 228f32, 181f32),
                        (182f32, 226f32, 181f32),
                        (178f32, 224f32, 182f32),
                        (175f32, 223f32, 182f32),
                        (169f32, 221f32, 182f32),
                        (164f32, 219f32, 183f32),
                        (160f32, 217f32, 183f32),
                        (155f32, 216f32, 184f32),
                        (151f32, 214f32, 184f32),
                        (146f32, 212f32, 185f32),
                        (141f32, 210f32, 185f32),
                        (139f32, 209f32, 185f32),
                        (132f32, 207f32, 186f32),
                        (128f32, 205f32, 186f32),
                        (124f32, 204f32, 187f32),
                        (120f32, 202f32, 187f32),
                        (116f32, 201f32, 188f32),
                        (112f32, 199f32, 189f32),
                        (108f32, 198f32, 189f32),
                        (106f32, 197f32, 189f32),
                        (100f32, 195f32, 190f32),
                        (97f32, 193f32, 191f32),
                        (93f32, 192f32, 191f32),
                        (89f32, 191f32, 192f32),
                        (85f32, 189f32, 193f32),
                        (81f32, 188f32, 193f32),
                        (77f32, 186f32, 194f32),
                        (75f32, 185f32, 194f32),
                        (69f32, 183f32, 195f32),
                        (65f32, 182f32, 195f32),
                        (63f32, 180f32, 195f32),
                        (61f32, 177f32, 195f32),
                        (58f32, 175f32, 195f32),
                        (56f32, 173f32, 195f32),
                        (54f32, 170f32, 194f32),
                        (52f32, 168f32, 194f32),
                        (49f32, 166f32, 194f32),
                        (47f32, 164f32, 194f32),
                        (45f32, 161f32, 193f32),
                        (42f32, 159f32, 193f32),
                        (40f32, 157f32, 193f32),
                        (39f32, 155f32, 193f32),
                        (36f32, 152f32, 192f32),
                        (33f32, 150f32, 192f32),
                        (31f32, 147f32, 192f32),
                        (29f32, 145f32, 192f32),
                        (29f32, 142f32, 190f32),
                        (29f32, 139f32, 189f32),
                        (29f32, 135f32, 187f32),
                        (30f32, 132f32, 186f32),
                        (30f32, 129f32, 184f32),
                        (30f32, 126f32, 183f32),
                        (31f32, 123f32, 181f32),
                        (31f32, 119f32, 180f32),
                        (31f32, 116f32, 178f32),
                        (32f32, 113f32, 177f32),
                        (32f32, 110f32, 175f32),
                        (32f32, 108f32, 174f32),
                        (33f32, 103f32, 172f32),
                        (33f32, 100f32, 171f32),
                        (33f32, 97f32, 169f32),
                        (33f32, 94f32, 168f32),
                        (34f32, 91f32, 166f32),
                        (34f32, 89f32, 165f32),
                        (34f32, 86f32, 164f32),
                        (34f32, 83f32, 163f32),
                        (34f32, 81f32, 161f32),
                        (35f32, 78f32, 160f32),
                        (35f32, 75f32, 159f32),
                        (35f32, 73f32, 158f32),
                        (35f32, 70f32, 156f32),
                        (35f32, 67f32, 155f32),
                        (36f32, 65f32, 154f32),
                        (36f32, 64f32, 153f32),
                        (36f32, 60f32, 151f32),
                        (36f32, 57f32, 150f32),
                        (36f32, 54f32, 149f32),
                        (36f32, 52f32, 148f32),
                        (35f32, 50f32, 144f32),
                        (33f32, 49f32, 140f32),
                        (31f32, 47f32, 136f32),
                        (29f32, 46f32, 133f32),
                        (28f32, 44f32, 129f32),
                        (26f32, 43f32, 125f32),
                        (24f32, 41f32, 121f32),
                        (22f32, 40f32, 118f32),
                        (20f32, 39f32, 114f32),
                        (18f32, 37f32, 110f32),
                        (17f32, 36f32, 106f32),
                        (16f32, 35f32, 104f32),
                        (13f32, 33f32, 99f32),
                        (11f32, 31f32, 95f32),
                        (9f32, 30f32, 91f32),
                        (8f32, 28f32, 87f32),
                    ]
                };

                if reverse_palette {
                    elev_palette.reverse();
                }

                let step_size = 1f32 / (elev_palette.len() - 1) as f32;
                let elev_palette_cutoffs_l: Vec<f32> = (0..elev_palette.len())
                    .into_iter()
                    .map(|i| i as f32 * step_size)
                    .collect();
                let elev_palette_cutoffs_u: Vec<f32> = (0..elev_palette.len())
                    .into_iter()
                    .map(|i| (i + 1) as f32 * step_size)
                    .collect();
                let mut alpha3: f32;
                let (mut red, mut green, mut blue): (u32, u32, u32);
                let (mut red_relief, mut green_relief, mut blue_relief) = (0u32, 0u32, 0u32);
                let (mut proportion_r, mut proportion_g, mut proportion_b): (f32, f32, f32);
                let red_atm = 185u32;
                let green_atm = 220u32;
                let blue_atm = 255u32;
                let mut hs_val: i16;
                let hs_range = (new_max - new_min) as f32;
                let mut hs_proportion: f32;
                let dx = [
                    -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0,
                    1, 2,
                ];
                let dy = [
                    -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2,
                    2, 2,
                ];
                let mut num_vals: f32;
                let mut smoothed_hs: f32;
                let mut hs_n: f32;
                let mut elev: f64;
                let mut elev_proportion: f32;
                // let mut x: f32;
                let mut rng = SmallRng::from_entropy();
                let between = Uniform::from(0..400);
                let mut rn: f32;
                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    // for row in 0..rows {
                    let mut data = vec![256f64; columns as usize];
                    for col in 0..columns {
                        hs_val = hs.get_value(row, col);
                        if hs_val != -32768 {
                            // Do elevation first.
                            elev = input.get_value(row, col);
                            if elev <= new_elev_min {
                                elev_proportion = 0f32;
                                red_relief = elev_palette[0].0 as u32;
                                green_relief = elev_palette[0].1 as u32;
                                blue_relief = elev_palette[0].2 as u32;
                            } else if elev >= new_elev_max {
                                elev_proportion = 1f32;
                                red_relief = elev_palette[elev_palette.len() - 1].0 as u32;
                                green_relief = elev_palette[elev_palette.len() - 1].1 as u32;
                                blue_relief = elev_palette[elev_palette.len() - 1].2 as u32;
                            } else {
                                elev_proportion = ((elev - new_elev_min) / new_elev_range) as f32;

                                for i in 0..elev_palette.len() - 1 {
                                    if elev_proportion <= elev_palette_cutoffs_u[i] {
                                        red_relief = (elev_palette[i].0
                                            + (elev_proportion - elev_palette_cutoffs_l[i])
                                                / step_size
                                                * (elev_palette[i + 1].0 - elev_palette[i].0))
                                            .floor()
                                            as u32;
                                        green_relief = (elev_palette[i].1
                                            + (elev_proportion - elev_palette_cutoffs_l[i])
                                                / step_size
                                                * (elev_palette[i + 1].1 - elev_palette[i].1))
                                            .floor()
                                            as u32;
                                        blue_relief = (elev_palette[i].2
                                            + (elev_proportion - elev_palette_cutoffs_l[i])
                                                / step_size
                                                * (elev_palette[i + 1].2 - elev_palette[i].2))
                                            .floor()
                                            as u32;

                                        break;
                                    }
                                }
                            }

                            // Now the hillshade value.

                            // First calculate a smoothed HS value, to be used with atmospheric effects.
                            alpha3 = atmospheric_alpha * (1.0 - elev_proportion); // atm
                            if alpha3 > 0.001f32 {
                                rn = between.sample(&mut rng) as f32 / 1000f32 * alpha3;
                                alpha3 += rn;
                                smoothed_hs = 0f32;
                                num_vals = 0f32;
                                for n in 0..dx.len() {
                                    if hs.get_value(row + dy[n], col + dx[n]) != -32768 {
                                        hs_n = hs.get_value(row + dy[n], col + dx[n]) as f32;
                                        smoothed_hs += hs_n;
                                        num_vals += 1f32;
                                    }
                                }
                                smoothed_hs /= num_vals;
                                hs_val =
                                    (hs_val as f32 * (1.0 - alpha3) + smoothed_hs * alpha3) as i16;
                            }

                            if hs_val <= new_min {
                                hs_proportion = 0f32;
                            } else if hs_val >= new_max {
                                hs_proportion = 1f32;
                            } else {
                                hs_proportion = (hs_val - new_min) as f32 / hs_range;
                            }
                            
                            // x = -(hs_val as f32 - mu) / s;
                            // hs_proportion = 1f32 / (1f32 + std::f32::consts::E.powf(x));

                            // Scale the colour by again by elevation, with lower elevations having a light-blue (153, 204, 255) tinge.
                            // This calculates the atm portion and then re-scale the relief and hs portions accordingly
                            
                            // Full shadow is (0f32, 50f32, 100f32), not black, which is too dark.
                            hs_proportion = relief_alpha + hs_alpha * hs_proportion;
                            proportion_r = 1f32 * (1f32 - hs_proportion) + red_relief as f32 * hs_proportion;
                            proportion_g = 25f32 * (1f32 - hs_proportion) + green_relief as f32 * hs_proportion;
                            proportion_b = 50f32 * (1f32 - hs_proportion) + blue_relief as f32 * hs_proportion;

                            red =
                                ((proportion_r * (1f32 - alpha3)) + alpha3 * red_atm as f32) as u32;
                            green = ((proportion_g * (1f32 - alpha3)) + alpha3 * green_atm as f32)
                                as u32;
                            blue = ((proportion_b * (1f32 - alpha3)) + alpha3 * blue_atm as f32)
                                as u32;

                            if red > 255 {
                                red = 255;
                            }
                            if green > 255 {
                                green = 255;
                            }
                            if blue > 255 {
                                blue = 255;
                            }

                            data[col as usize] =
                                ((255 << 24) | (blue << 16) | (green << 8) | red) as f64;
                        } else {
                            data[col as usize] = out_nodata;
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
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
        // output.add_metadata_entry(format!("Azimuth: {}", azimuth));
        output.add_metadata_entry(format!("Altitude: {}", altitude));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
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

        Ok(())
    }
}
