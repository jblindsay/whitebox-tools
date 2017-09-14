/* 
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: August 31, 2017
Last Modified: August 31, 2017
License: MIT
*/
extern crate time;
extern crate num_cpus;

use std::env;
use std::path;
use std::f64;
use raster::*;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use tools::WhiteboxTool;

pub struct HistogramMatchingTwoImages {
    name: String,
    description: String,
    parameters: String,
    example_usage: String,
}

impl HistogramMatchingTwoImages {
    pub fn new() -> HistogramMatchingTwoImages {
        // public constructor
        let name = "HistogramMatchingTwoImages".to_string();

        let description = "This tool alters the cumululative distribution function of a raster image to that of another image."
            .to_string();

        let mut parameters = "--i1, --input1   Input raster file to modify.\n".to_owned();
        parameters.push_str("--i2, --input2   Input reference raster file.\n");
        parameters.push_str("-o, --output     Output raster file.\n");

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let p = format!("{}", env::current_dir().unwrap().display());
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut short_exe = e.replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --i1=input1.dep --i2=input2.dep -o=output.dep", short_exe, name).replace("*", &sep);

        HistogramMatchingTwoImages {
            name: name,
            description: description,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for HistogramMatchingTwoImages {
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        self.parameters.clone()
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn run<'a>(&self,
               args: Vec<String>,
               working_directory: &'a str,
               verbose: bool)
               -> Result<(), Error> {
        let mut input_file1 = String::new();
        let mut input_file2 = String::new();
        let mut output_file = String::new();
        
        if args.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "Tool run with no paramters. Please see help (-h) for parameter descriptions."));
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
            if vec[0].to_lowercase() == "-i1" || vec[0].to_lowercase() == "--i1" || vec[0].to_lowercase() == "--input1" {
                if keyval {
                    input_file1 = vec[1].to_string();
                } else {
                    input_file1 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-i2" || vec[0].to_lowercase() == "--i2" || vec[0].to_lowercase() == "--input2" {
                if keyval {
                    input_file2 = vec[1].to_string();
                } else {
                    input_file2 = args[i + 1].to_string();
                }
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
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

        if !input_file1.contains(&sep) {
            input_file1 = format!("{}{}", working_directory, input_file1);
        }
        if !input_file2.contains(&sep) {
            input_file2 = format!("{}{}", working_directory, input_file2);
        }
        if !output_file.contains(&sep) {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose { println!("Reading input data...") };
        let input1 = Arc::new(Raster::new(&input_file1, "r")?);
        // let input2 = Arc::new(Raster::new(&input_file2, "r")?);
        // let input1 = Raster::new(&input_file1, "r")?;
        let input2 = Raster::new(&input_file2, "r")?;

        if input1.configs.data_type == DataType::RGB24 ||
            input1.configs.data_type == DataType::RGB48 ||
            input1.configs.data_type == DataType::RGBA32 ||
            input1.configs.photometric_interp == PhotometricInterpretation::RGB {

            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool is for single-band greyscale images and cannot be applied to RGB colour-composite images."));
        }
        if input2.configs.data_type == DataType::RGB24 ||
            input2.configs.data_type == DataType::RGB48 ||
            input2.configs.data_type == DataType::RGBA32 ||
            input2.configs.photometric_interp == PhotometricInterpretation::RGB {

            return Err(Error::new(ErrorKind::InvalidInput,
                "This tool is for single-band greyscale images and cannot be applied to RGB colour-composite images."));
        }
        let start = time::now();

        let rows1 = input1.configs.rows as isize;
        let columns1 = input1.configs.columns as isize;
        let nodata1 = input1.configs.nodata;
        let min_value1 = input1.configs.minimum;
        let max_value1 = input1.configs.maximum;
        let num_bins1 = ((max_value1 - min_value1).max(1024f64)).ceil() as usize; //(2f64 * (max_value1 - min_value1 + 1f64).ceil().max((((rows1 * columns1) as f64).powf(1f64 / 3f64)).ceil())) as usize;
        let bin_size = (max_value1 - min_value1) / num_bins1 as f64;
        let mut histogram = vec![0f64; num_bins1];
        let num_bins_less_one1 = num_bins1 - 1;
        let mut z: f64;
        let mut numcells1: f64 = 0f64;
        let mut bin_num;
        for row in 0..rows1 {
            for col in 0..columns1 {
                z = input1[(row, col)];
                if z != nodata1 {
                    numcells1 += 1f64;
                    bin_num = ((z - min_value1) / bin_size) as usize;
                    if bin_num > num_bins_less_one1 { bin_num = num_bins_less_one1; }
                    histogram[bin_num] += 1f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows1 - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 1 of 3: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut cdf = vec![0f64; histogram.len()];
        cdf[0] = histogram[0];
        for i in 1..cdf.len() {
            cdf[i] = cdf[i - 1] + histogram[i];
        }
        for i in 0..cdf.len() {
            cdf[i] = cdf[i] / numcells1;
        }

        let rows2 = input2.configs.rows as isize;
        let columns2 = input2.configs.columns as isize;
        let nodata2 = input2.configs.nodata;

        let min_value2 = input2.configs.minimum;
        let max_value2 = input2.configs.maximum;
        let num_bins2 = ((max_value2 - min_value2).max(1024f64)).ceil() as usize; //(2f64 * (max_value2 - min_value2 + 1f64).ceil().max((((rows2 * columns2) as f64).powf(1f64 / 3f64)).ceil())) as usize;
        let num_bins_less_one2 = num_bins2 - 1;
        let mut numcells2: f64 = 0f64;
        let mut histogram2 = vec![0f64; num_bins2];
        
        for row in 0..rows2 {
            for col in 0..columns2 {
                z = input2[(row, col)];
                if z != nodata2 {
                    numcells2 += 1f64;
                    bin_num = ((z - min_value2) / bin_size) as usize;
                    if bin_num > num_bins_less_one2 { bin_num = num_bins_less_one2; }
                    histogram2[bin_num] += 1f64;
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows2 - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 2 of 3: {}%", progress);
                    old_progress = progress;
                }
            }
        }
        
        // convert the reference histogram to a cdf.
        let mut reference_cdf = vec![vec![0f64; 2]; num_bins2];
        reference_cdf[0][1] = histogram2[0]; 
        for i in 1..num_bins2 {
            reference_cdf[i][1] = reference_cdf[i - 1][1] + histogram2[i];
        }
        
        for i in 0..num_bins2 {
            reference_cdf[i][0] = min_value2 + (i as f64 / num_bins2 as f64) * (max_value2 - min_value2);
            reference_cdf[i][1] = reference_cdf[i][1] / numcells2;
        }
        
        let mut starting_vals = [0usize; 11];
        let mut p_val: f64;
        for i in 0..num_bins2 {
            p_val = reference_cdf[i][1];
            if p_val < 0.1 {
                starting_vals[1] = i;
            }
            if p_val < 0.2 {
                starting_vals[2] = i;
            }
            if p_val < 0.3 {
                starting_vals[3] = i;
            }
            if p_val < 0.4 {
                starting_vals[4] = i;
            }
            if p_val < 0.5 {
                starting_vals[5] = i;
            }
            if p_val < 0.6 {
                starting_vals[6] = i;
            }
            if p_val < 0.7 {
                starting_vals[7] = i;
            }
            if p_val < 0.8 {
                starting_vals[8] = i;
            }
            if p_val < 0.9 {
                starting_vals[9] = i;
            }
            if p_val <= 1f64 {
                starting_vals[10] = i;
            }
        }

        // let mut output = Raster::initialize_using_file(&output_file, &input1);
        // let mut j: usize;
        // let mut x_val = 0f64;
        // let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
        // for row in 0..rows1 {
        //     for col in 0..columns1 {
        //         z = input1[(row, col)];
        //         if z != nodata1 {
        //             bin_num = ((z - min_value1) / bin_size) as usize;
        //             if bin_num > num_bins_less_one1 { bin_num = num_bins_less_one1; }
        //             p_val = cdf[bin_num];
        //             j = ((p_val * 10f64).floor()) as usize;
        //             for i in starting_vals[j]..num_bins2 {
        //                 if reference_cdf[i][1] > p_val {
        //                     if i > 0 {
        //                         x1 = reference_cdf[i - 1][0];
        //                         x2 = reference_cdf[i][0];
        //                         p1 = reference_cdf[i - 1][1];
        //                         p2 = reference_cdf[i][1];
        //                         if p1 != p2 {
        //                             x_val = x1 + ((x2 - x1) * ((p_val - p1) / (p2 - p1)));
        //                         } else {
        //                             x_val = x1;
        //                         }
        //                     } else {
        //                         x_val = reference_cdf[i][0];
        //                     }
        //                     break;
        //                 }
        //             }
        //             output[(row, col)] = x_val;
        //         }
        //     }
            
        //     if verbose {
        //         progress = (100.0_f64 * row as f64 / (rows1 - 1) as f64) as usize;
        //         if progress != old_progress {
        //             println!("Loop 3 of 3: {}%", progress);
        //             old_progress = progress;
        //         }
        //     }
        // }



            


        let starting_vals = Arc::new(starting_vals);
        let reference_cdf = Arc::new(reference_cdf);
        let cdf = Arc::new(cdf);

        let num_procs = num_cpus::get() as isize;
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let input1 = input1.clone();
            let starting_vals = starting_vals.clone();
            let reference_cdf = reference_cdf.clone();
            let cdf = cdf.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut z: f64;
                let mut bin_num: usize;
                let mut j: usize;
                let mut x_val = 0f64;
                let mut p_val: f64;
                let (mut x1, mut x2, mut p1, mut p2): (f64, f64, f64, f64);
                for row in (0..rows1).filter(|r| r % num_procs == tid) {
                    let mut data: Vec<f64> = vec![nodata1; columns1 as usize];
                    for col in 0..columns1 {
                        z = input1[(row, col)];
                        if z != nodata1 {
                            bin_num = ((z - min_value1) / bin_size) as usize;
                            if bin_num > num_bins_less_one1 { bin_num = num_bins_less_one1; }
                            p_val = cdf[bin_num];
                            j = ((p_val * 10f64).floor()) as usize;
                            for i in starting_vals[j]..num_bins2 {
                                if reference_cdf[i][1] > p_val {
                                    if i > 0 {
                                        x1 = reference_cdf[i - 1][0];
                                        x2 = reference_cdf[i][0];
                                        p1 = reference_cdf[i - 1][1];
                                        p2 = reference_cdf[i][1];
                                        if p1 != p2 {
                                            x_val = x1 + ((x2 - x1) * ((p_val - p1) / (p2 - p1)));
                                        } else {
                                            x_val = x1;
                                        }
                                    } else {
                                        x_val = reference_cdf[i][0];
                                    }
                                    break;
                                }
                            }
                            data[col as usize] = x_val;
                        }
                    }
                    tx.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &input1);
        for r in 0..rows1 {
            let (row, data) = rx.recv().unwrap();
            output.set_row_data(row, data);
            if verbose {
                progress = (100.0_f64 * r as f64 / (rows1 - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Loop 3 of 3: {}%", progress);
                    old_progress = progress;
                }
            }
        }


        let end = time::now();
        let elapsed_time = end - start;
        output.add_metadata_entry(format!("Created by whitebox_tools\' {} tool",
                                          self.get_tool_name()));
        output.add_metadata_entry(format!("Input file to modify: {}", input_file1));
        output.add_metadata_entry(format!("Input reference file: {}", input_file2));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time)
                                      .replace("PT", ""));

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

        println!("{}",
                 &format!("Elapsed Time (excluding I/O): {}", elapsed_time).replace("PT", ""));

        Ok(())
    }
}
